// SPDX-License-Identifier: MIT
//! `impforge-cli provider` — Multi-Provider BYOK chat (Feature 1, Tier 1/3).
//!
//! Supports five providers:
//!   - **Ollama**     (local, no key required)
//!   - **OpenAI**     (`OPENAI_API_KEY`)
//!   - **Anthropic**  (`ANTHROPIC_API_KEY`)
//!   - **Gemini**     (`GEMINI_API_KEY`)
//!   - **OpenRouter** (`OPENROUTER_API_KEY` — OpenAI-compatible, custom base URL)
//!
//! Keys live in the OS keychain via `keyring-rs`. We never log a key, never
//! place it in argv, never write it to a config file. Round-trip: the user
//! invokes `impforge-cli provider add openai sk-...`, we store under the
//! `(impforge-cli, openai)` keychain entry; `provider chat` retrieves it
//! on demand and hands it to `genai` via an `AuthResolver`.
//!
//! See research report `2026-04-19-multi-provider-byok.md` §3 (security) +
//! §B Tier-1 row.

use crate::theme;
use clap::Subcommand;
use futures::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatRequest, ChatStreamEvent, StreamChunk};
use genai::resolver::{AuthData, AuthResolver, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use impforge_emergence::Orchestrator;
use std::io::Write;
use std::sync::Arc;

/// Service name registered with the OS keychain. Stable across versions —
/// changing it invalidates every stored key.
const KEYRING_SERVICE: &str = "impforge-cli";

/// The five canonical providers in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderId {
    Ollama,
    OpenAi,
    Anthropic,
    Gemini,
    OpenRouter,
}

impl ProviderId {
    /// Stable lower-case name used both in CLI args and as the keychain
    /// `username` slot. Renaming any of these breaks existing installs.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::Ollama => "ollama",
            ProviderId::OpenAi => "openai",
            ProviderId::Anthropic => "anthropic",
            ProviderId::Gemini => "gemini",
            ProviderId::OpenRouter => "openrouter",
        }
    }

    /// Parse from CLI input. Accepts any case for friendliness.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "ollama" => Some(ProviderId::Ollama),
            "openai" => Some(ProviderId::OpenAi),
            "anthropic" | "claude" => Some(ProviderId::Anthropic),
            "gemini" | "google" => Some(ProviderId::Gemini),
            "openrouter" | "router" => Some(ProviderId::OpenRouter),
            _ => None,
        }
    }

    /// Iterator over every supported provider — used by `provider list`.
    pub fn all() -> &'static [ProviderId] {
        &[
            ProviderId::Ollama,
            ProviderId::OpenAi,
            ProviderId::Anthropic,
            ProviderId::Gemini,
            ProviderId::OpenRouter,
        ]
    }

    /// True iff this provider needs an API key (Ollama is local).
    pub fn requires_key(&self) -> bool {
        !matches!(self, ProviderId::Ollama)
    }

    /// Default model when `--model` is omitted. Cheap-yet-capable picks per
    /// `2026-04-19-multi-provider-byok.md` §5 + §6 ("Recommended" badge).
    pub fn default_model(&self) -> &'static str {
        match self {
            ProviderId::Ollama => "qwen3:8b",
            ProviderId::OpenAi => "gpt-4o-mini",
            ProviderId::Anthropic => "claude-3-haiku-20240307",
            ProviderId::Gemini => "gemini-2.0-flash",
            ProviderId::OpenRouter => "openai/gpt-4o-mini",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum ProviderCmd {
    /// Add (or replace) an API key for a provider. Stored in the OS keychain.
    Add {
        /// Provider name (ollama / openai / anthropic / gemini / openrouter).
        provider: String,
        /// Raw API key. Tip: pass via `$(read -s K; echo $K)` to keep it out of shell history.
        key: String,
    },
    /// List configured providers and whether a key is stored.
    List,
    /// Remove a stored API key.
    Remove {
        /// Provider name.
        provider: String,
    },
    /// Send a one-shot chat prompt to the chosen provider and stream the response.
    Chat {
        /// User prompt.
        prompt: String,
        /// Provider name.
        #[arg(long)]
        provider: String,
        /// Model name; defaults to a sensible cheap+capable pick per provider.
        #[arg(long)]
        model: Option<String>,
    },
}

pub fn run(cmd: ProviderCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        ProviderCmd::Add { provider, key } => {
            let id = parse_or_bail(&provider)?;
            add_key(id, &key)?;
        }
        ProviderCmd::List => list_providers()?,
        ProviderCmd::Remove { provider } => {
            let id = parse_or_bail(&provider)?;
            remove_key(id)?;
        }
        ProviderCmd::Chat {
            prompt,
            provider,
            model,
        } => {
            let id = parse_or_bail(&provider)?;
            let model = model.unwrap_or_else(|| id.default_model().to_string());
            chat_blocking(id, &model, &prompt)?;
        }
    }
    Ok(())
}

fn parse_or_bail(name: &str) -> anyhow::Result<ProviderId> {
    ProviderId::parse(name).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown provider '{name}'. Try one of: ollama, openai, anthropic, gemini, openrouter",
        )
    })
}

/// Save a key to the OS keychain under `(KEYRING_SERVICE, provider)`.
fn add_key(provider: ProviderId, key: &str) -> anyhow::Result<()> {
    if !provider.requires_key() {
        theme::print_warning(&format!(
            "{} is local-only and needs no API key — skipping.",
            provider.as_str()
        ));
        return Ok(());
    }
    if key.trim().is_empty() {
        anyhow::bail!("refusing to store an empty key");
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider.as_str())?;
    entry.set_password(key)?;
    theme::print_success(&format!(
        "stored {} key in OS keychain (service={})",
        provider.as_str(),
        KEYRING_SERVICE
    ));
    Ok(())
}

/// Remove a key from the OS keychain.
fn remove_key(provider: ProviderId) -> anyhow::Result<()> {
    if !provider.requires_key() {
        theme::print_info(&format!(
            "{} has no key to remove (local-only).",
            provider.as_str()
        ));
        return Ok(());
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider.as_str())?;
    match entry.delete_credential() {
        Ok(()) => {
            theme::print_success(&format!("removed {} key", provider.as_str()));
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            theme::print_info(&format!(
                "no key was stored for {} — nothing to remove",
                provider.as_str()
            ));
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Read a key from the keychain. Returns `Ok(None)` on `NoEntry` so callers
/// can distinguish "missing" from "transport failure".
fn get_key(provider: ProviderId) -> anyhow::Result<Option<String>> {
    if !provider.requires_key() {
        return Ok(None);
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider.as_str())?;
    match entry.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Print a table of providers and their key status.
fn list_providers() -> anyhow::Result<()> {
    theme::print_info("Configured providers:");
    for &p in ProviderId::all() {
        let status = if p.requires_key() {
            match get_key(p)? {
                Some(_) => format!("{}key set{}", theme::ACCENT_NEON, theme::RESET),
                None => format!("{}no key{}", theme::ACCENT_MAGENTA, theme::RESET),
            }
        } else {
            format!("{}local{}", theme::ACCENT_CYAN, theme::RESET)
        };
        println!(
            "  {}{:<12}{}  status: {}  default: {}{}{}",
            theme::ACCENT_NEON,
            p.as_str(),
            theme::RESET,
            status,
            theme::ACCENT_CYAN,
            p.default_model(),
            theme::RESET
        );
    }
    Ok(())
}

/// Build a `genai::Client` configured for the chosen provider.
///
/// - **Ollama**: stock `genai` Ollama adapter (talks to localhost:11434).
/// - **OpenAI/Anthropic/Gemini**: stock adapters + `AuthResolver` that pulls
///   from the keychain (instead of env vars).
/// - **OpenRouter**: OpenAI adapter + `ServiceTargetResolver` that swaps the
///   base URL to `https://openrouter.ai/api/v1`. This is the canonical
///   OpenAI-compat-via-custom-base pattern.
fn build_client(provider: ProviderId, key: Option<String>) -> Client {
    match provider {
        ProviderId::Ollama => Client::default(),
        ProviderId::OpenAi | ProviderId::Anthropic | ProviderId::Gemini => {
            let key_for_resolver = key.unwrap_or_default();
            let auth = AuthResolver::from_resolver_fn(
                move |_: ModelIden| -> Result<Option<AuthData>, genai::resolver::Error> {
                    Ok(Some(AuthData::from_single(key_for_resolver.clone())))
                },
            );
            Client::builder().with_auth_resolver(auth).build()
        }
        ProviderId::OpenRouter => {
            let key_for_resolver = key.unwrap_or_default();
            let resolver = ServiceTargetResolver::from_resolver_fn(
                move |st: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                    let endpoint = Endpoint::from_static("https://openrouter.ai/api/v1/");
                    let auth = AuthData::from_single(key_for_resolver.clone());
                    let model = ModelIden::new(AdapterKind::OpenAI, st.model.model_name);
                    Ok(ServiceTarget {
                        endpoint,
                        auth,
                        model,
                    })
                },
            );
            Client::builder()
                .with_service_target_resolver(resolver)
                .build()
        }
    }
}

/// Run a streaming chat call, blocking on a one-shot tokio runtime so that
/// `commands::provider::run` stays sync (matches the rest of the CLI).
fn chat_blocking(provider: ProviderId, model: &str, prompt: &str) -> anyhow::Result<()> {
    let key = get_key(provider)?;
    if provider.requires_key() && key.is_none() {
        anyhow::bail!(
            "no key stored for {}. Run: impforge-cli provider add {} <KEY>",
            provider.as_str(),
            provider.as_str()
        );
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move { chat_async(provider, model, prompt, key).await })
}

async fn chat_async(
    provider: ProviderId,
    model: &str,
    prompt: &str,
    key: Option<String>,
) -> anyhow::Result<()> {
    let client = build_client(provider, key);
    let req = ChatRequest::new(vec![ChatMessage::user(prompt)]);

    theme::print_info(&format!("{} → {} (streaming)", provider.as_str(), model));

    let mut stream_resp = client.exec_chat_stream(model, req, None).await?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    while let Some(event) = stream_resp.stream.next().await {
        let event = event?;
        match event {
            ChatStreamEvent::Chunk(StreamChunk { content }) => {
                write!(out, "{content}")?;
                out.flush()?;
            }
            ChatStreamEvent::ReasoningChunk(StreamChunk { content }) => {
                write!(out, "{}{}{}", theme::ACCENT_MAGENTA, content, theme::RESET)?;
                out.flush()?;
            }
            ChatStreamEvent::Start | ChatStreamEvent::End(_) => {
                // Bracketing events — nothing to print at the chunk loop.
            }
            ChatStreamEvent::ToolCallChunk(_) => {
                // Tool-call streaming is out of scope for v1 BYOK chat.
            }
            ChatStreamEvent::ThoughtSignatureChunk(_) => {
                // Provider-internal thought-signature opaque blob — not user-facing.
            }
        }
    }
    writeln!(out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_parse_canonical() {
        assert_eq!(ProviderId::parse("ollama"), Some(ProviderId::Ollama));
        assert_eq!(ProviderId::parse("openai"), Some(ProviderId::OpenAi));
        assert_eq!(ProviderId::parse("anthropic"), Some(ProviderId::Anthropic));
        assert_eq!(ProviderId::parse("gemini"), Some(ProviderId::Gemini));
        assert_eq!(
            ProviderId::parse("openrouter"),
            Some(ProviderId::OpenRouter)
        );
    }

    #[test]
    fn provider_id_parse_aliases_and_case() {
        assert_eq!(ProviderId::parse("Claude"), Some(ProviderId::Anthropic));
        assert_eq!(ProviderId::parse("GOOGLE"), Some(ProviderId::Gemini));
        assert_eq!(ProviderId::parse("router"), Some(ProviderId::OpenRouter));
        assert_eq!(ProviderId::parse(" ollama "), Some(ProviderId::Ollama));
    }

    #[test]
    fn provider_id_parse_rejects_unknown() {
        assert!(ProviderId::parse("groq").is_none());
        assert!(ProviderId::parse("").is_none());
        assert!(ProviderId::parse("foo-bar").is_none());
    }

    #[test]
    fn requires_key_only_for_remote_providers() {
        assert!(!ProviderId::Ollama.requires_key());
        assert!(ProviderId::OpenAi.requires_key());
        assert!(ProviderId::Anthropic.requires_key());
        assert!(ProviderId::Gemini.requires_key());
        assert!(ProviderId::OpenRouter.requires_key());
    }

    #[test]
    fn default_model_per_provider_is_set() {
        for &p in ProviderId::all() {
            let m = p.default_model();
            assert!(!m.is_empty(), "{} has empty default model", p.as_str());
        }
    }

    #[test]
    fn keyring_service_is_stable() {
        // Renaming this constant invalidates every stored key.
        // Anyone bumping it must do so with a deliberate migration plan.
        assert_eq!(KEYRING_SERVICE, "impforge-cli");
    }

    #[test]
    fn all_returns_five_providers() {
        assert_eq!(ProviderId::all().len(), 5);
    }

    #[test]
    fn as_str_roundtrips_with_parse() {
        for &p in ProviderId::all() {
            assert_eq!(ProviderId::parse(p.as_str()), Some(p));
        }
    }
}
