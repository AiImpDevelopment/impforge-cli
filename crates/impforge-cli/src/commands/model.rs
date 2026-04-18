// SPDX-License-Identifier: MIT
//! `impforge-cli model` subcommand — real Ollama integration.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use impforge_models::backend::{Backend, ModelIdentifier};
use impforge_models::ollama;
use std::process::Command as ProcessCommand;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum ModelCmd {
    /// List locally-available Ollama models.
    List,
    /// Pull a model via Ollama (shortcut: `brain` → qwen3-imp:8b).
    Pull { id: String },
    /// Run a one-shot inference with a local model.
    Run {
        id: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        prompt: Vec<String>,
    },
    /// Benchmark local hardware.
    Benchmark,
    /// Check whether Ollama is reachable.
    Ping,
}

pub fn run(cmd: ModelCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        ModelCmd::List => list()?,
        ModelCmd::Pull { id } => pull(&id)?,
        ModelCmd::Run { id, prompt } => {
            let joined = prompt.join(" ");
            run_inference(&id, &joined)?;
        }
        ModelCmd::Benchmark => benchmark(),
        ModelCmd::Ping => ping(),
    }
    Ok(())
}

fn list() -> anyhow::Result<()> {
    if !ollama::is_reachable(None) {
        theme::print_warning(
            "Ollama is not reachable at 127.0.0.1:11434 — install from https://ollama.com and run `ollama serve`",
        );
        return Ok(());
    }
    let models = ollama::list_local_models(None)?;
    if models.is_empty() {
        theme::print_info("no local models yet — try `impforge-cli brain pull`");
        return Ok(());
    }
    theme::print_info(&format!("{} local model(s):", models.len()));
    for m in models {
        let size_mb = m.size / (1024 * 1024);
        println!(
            "  {}{:<32}{}  {}{:>6} MB{}  {}{}{}",
            theme::ACCENT_NEON, m.name, theme::RESET,
            theme::ACCENT_CYAN, size_mb, theme::RESET,
            theme::DIM, m.details.quantization_level, theme::RESET
        );
    }
    Ok(())
}

fn pull(id: &str) -> anyhow::Result<()> {
    let identifier = if id == "brain" {
        ModelIdentifier { backend: Backend::Ollama, name: "qwen3-imp:8b".to_string() }
    } else {
        ModelIdentifier::parse(id)?
    };
    match identifier.backend {
        Backend::Ollama => pull_ollama(&identifier.name),
        Backend::HuggingFace => {
            theme::print_info(&format!(
                "HuggingFace pull requested for '{}'. Run: `huggingface-cli download {}`",
                identifier.name, identifier.name
            ));
            Ok(())
        }
        other => {
            theme::print_warning(&format!(
                "backend {other:?} pull not yet wired — use Ollama / HuggingFace for now"
            ));
            Ok(())
        }
    }
}

fn pull_ollama(name: &str) -> anyhow::Result<()> {
    theme::print_info(&format!("pulling {name} via Ollama"));
    let status = ProcessCommand::new("ollama").arg("pull").arg(name).status();
    match status {
        Ok(s) if s.success() => {
            theme::print_success(&format!("'{name}' pulled and ready"));
            Ok(())
        }
        Ok(s) => anyhow::bail!("ollama pull exited with {s}"),
        Err(e) => {
            theme::print_warning(&format!(
                "Ollama CLI not found on PATH ({e}) — falling back to API"
            ));
            ollama::pull_model(name, None)?;
            theme::print_success(&format!("'{name}' pulled via API"));
            Ok(())
        }
    }
}

fn run_inference(id: &str, prompt: &str) -> anyhow::Result<()> {
    if prompt.trim().is_empty() {
        anyhow::bail!("prompt is empty — pass it after the model id");
    }
    let identifier = if id == "brain" {
        ModelIdentifier { backend: Backend::Ollama, name: "qwen3-imp:8b".to_string() }
    } else {
        ModelIdentifier::parse(id)?
    };
    if !matches!(identifier.backend, Backend::Ollama) {
        theme::print_warning(&format!(
            "inference for backend {:?} not yet wired — Ollama only for now",
            identifier.backend
        ));
        return Ok(());
    }
    if !ollama::is_reachable(None) {
        theme::print_error("Ollama is not reachable at 127.0.0.1:11434");
        return Ok(());
    }
    theme::print_info(&format!("running {} on {} byte prompt", identifier.name, prompt.len()));
    let resp = ollama::generate_once(&identifier.name, prompt, None, None)?;
    println!();
    println!("{}", resp.response.trim());
    println!();
    let duration_ms = resp.total_duration / 1_000_000;
    theme::print_info(&format!("{} tokens in {} ms", resp.eval_count, duration_ms));
    Ok(())
}

fn benchmark() {
    theme::print_info("Local hardware benchmark");
    println!("  CPU threads   : {}", num_cpus_guess());
    println!("  Architecture  : {}", std::env::consts::ARCH);
    println!("  OS            : {}", std::env::consts::OS);
    println!("  Ollama        : {}", if ollama::is_reachable(None) { "reachable" } else { "not running" });
}

fn ping() {
    if ollama::is_reachable(None) {
        theme::print_success("Ollama is reachable at 127.0.0.1:11434");
    } else {
        theme::print_warning("Ollama is NOT reachable — install from https://ollama.com");
    }
}

fn num_cpus_guess() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}
