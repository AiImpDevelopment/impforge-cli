// SPDX-License-Identifier: MIT
//! Real Ollama HTTP client — talks to the local Ollama daemon at
//! `127.0.0.1:11434`.
//!
//! Every function here is network I/O — blocking variants block on a
//! tokio current-thread runtime so command handlers can call them
//! without dragging tokio into sync contexts.

use impforge_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

pub const DEFAULT_HOST: &str = "http://127.0.0.1:11434";

/// One entry from `GET /api/tags`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalModel {
    pub name: String,
    pub model: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub digest: String,
    #[serde(default)]
    pub modified_at: String,
    #[serde(default)]
    pub details: ModelDetails,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelDetails {
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub parameter_size: String,
    #[serde(default)]
    pub quantization_level: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<LocalModel>,
}

#[derive(Debug, Serialize)]
struct PullBody<'a> {
    model: &'a str,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct GenerateBody<'a> {
    model: &'a str,
    prompt: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct GenerateResponse {
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub total_duration: u64,
    #[serde(default)]
    pub eval_count: u32,
}

/// Blocking `GET /api/tags` that lists locally-installed models.
pub fn list_local_models(host: Option<&str>) -> CoreResult<Vec<LocalModel>> {
    let host = host.unwrap_or(DEFAULT_HOST);
    let url = format!("{host}/api/tags");
    let resp = reqwest::blocking::get(&url).map_err(|e| {
        CoreError::Network(format!("GET {url} failed: {e}"))
    })?;
    if !resp.status().is_success() {
        return Err(CoreError::Network(format!(
            "GET {url} returned HTTP {}",
            resp.status()
        )));
    }
    let body = resp
        .json::<TagsResponse>()
        .map_err(|e| CoreError::Network(format!("GET {url} decode failed: {e}")))?;
    Ok(body.models)
}

/// Blocking probe: is Ollama reachable?
pub fn is_reachable(host: Option<&str>) -> bool {
    let host = host.unwrap_or(DEFAULT_HOST);
    let url = format!("{host}/api/tags");
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()
        .ok()
        .and_then(|c| c.get(&url).send().ok())
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Blocking `POST /api/pull` (non-streaming) — ask Ollama to download a
/// model.  For large downloads the `ollama` CLI is still faster for live
/// progress display; we expose the API variant for scripting.
pub fn pull_model(model: &str, host: Option<&str>) -> CoreResult<()> {
    let host = host.unwrap_or(DEFAULT_HOST);
    let url = format!("{host}/api/pull");
    let body = PullBody { model, stream: false };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(1_800))
        .build()
        .map_err(|e| CoreError::Network(format!("client: {e}")))?;
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| CoreError::Network(format!("POST {url}: {e}")))?;
    if !resp.status().is_success() {
        return Err(CoreError::Network(format!(
            "POST {url} returned HTTP {}",
            resp.status()
        )));
    }
    Ok(())
}

/// Blocking `POST /api/generate` (non-streaming, single response).
/// For streaming we have a separate async path in the CLI command.
pub fn generate_once(
    model: &str,
    prompt: &str,
    system: Option<&str>,
    host: Option<&str>,
) -> CoreResult<GenerateResponse> {
    let host = host.unwrap_or(DEFAULT_HOST);
    let url = format!("{host}/api/generate");
    let body = GenerateBody { model, prompt, system, stream: false };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| CoreError::Network(format!("client: {e}")))?;
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| CoreError::Network(format!("POST {url}: {e}")))?;
    if !resp.status().is_success() {
        return Err(CoreError::Network(format!(
            "POST {url} returned HTTP {}",
            resp.status()
        )));
    }
    resp.json::<GenerateResponse>()
        .map_err(|e| CoreError::Network(format!("decode: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_host_is_localhost() {
        assert!(DEFAULT_HOST.starts_with("http://127.0.0.1"));
    }

    #[test]
    fn local_model_roundtrips_json() {
        let m = LocalModel {
            name: "qwen2.5-coder:7b".to_string(),
            model: "qwen2.5-coder:7b".to_string(),
            size: 4_200_000_000,
            digest: "abc".to_string(),
            modified_at: "2026-04-18T00:00:00Z".to_string(),
            details: ModelDetails {
                family: "qwen2".to_string(),
                parameter_size: "7B".to_string(),
                quantization_level: "Q4_K_M".to_string(),
            },
        };
        let j = serde_json::to_string(&m).expect("serialize");
        let back: LocalModel = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(m, back);
    }

    #[test]
    fn is_reachable_returns_bool() {
        let _ = is_reachable(Some("http://127.0.0.1:65535"));
    }
}
