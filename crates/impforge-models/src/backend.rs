// SPDX-License-Identifier: MIT
//! Backend abstraction over Ollama / HF / llama.cpp / Candle.
//!
//! This is a deliberately thin wrapper so the CLI commands can target the
//! user's preferred backend without branching internally.

use impforge_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    Ollama,
    HuggingFace,
    LlamaCpp,
    Candle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelIdentifier {
    pub backend: Backend,
    pub name: String,
}

impl ModelIdentifier {
    pub fn parse(s: &str) -> CoreResult<Self> {
        if let Some(rest) = s.strip_prefix("ollama:") {
            return Ok(Self { backend: Backend::Ollama, name: rest.to_string() });
        }
        if let Some(rest) = s.strip_prefix("hf:") {
            return Ok(Self { backend: Backend::HuggingFace, name: rest.to_string() });
        }
        if let Some(rest) = s.strip_prefix("llama.cpp:") {
            return Ok(Self { backend: Backend::LlamaCpp, name: rest.to_string() });
        }
        if let Some(rest) = s.strip_prefix("candle:") {
            return Ok(Self { backend: Backend::Candle, name: rest.to_string() });
        }
        if s.is_empty() {
            return Err(CoreError::validation("model identifier is empty"));
        }
        Ok(Self { backend: Backend::Ollama, name: s.to_string() })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceRequest {
    pub model: ModelIdentifier,
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl InferenceRequest {
    pub fn quick(model: ModelIdentifier, prompt: impl Into<String>) -> Self {
        Self { model, prompt: prompt.into(), max_tokens: 512, temperature: 0.2 }
    }

    pub fn validate(&self) -> CoreResult<()> {
        if self.prompt.trim().is_empty() {
            return Err(CoreError::validation("prompt is empty"));
        }
        if self.max_tokens == 0 || self.max_tokens > 16_384 {
            return Err(CoreError::validation("max_tokens must be 1..=16384"));
        }
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(CoreError::validation("temperature must be 0.0..=2.0"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceResponse {
    pub text: String,
    pub tokens_generated: u32,
    pub duration_ms: u64,
    pub backend: Backend,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ollama_prefix() {
        let id = ModelIdentifier::parse("ollama:qwen2.5-coder:7b").expect("ok");
        assert_eq!(id.backend, Backend::Ollama);
        assert_eq!(id.name, "qwen2.5-coder:7b");
    }

    #[test]
    fn parse_hf_prefix() {
        let id = ModelIdentifier::parse("hf:Qwen/Qwen2.5-Coder-7B").expect("ok");
        assert_eq!(id.backend, Backend::HuggingFace);
    }

    #[test]
    fn parse_plain_defaults_to_ollama() {
        let id = ModelIdentifier::parse("llama3").expect("ok");
        assert_eq!(id.backend, Backend::Ollama);
        assert_eq!(id.name, "llama3");
    }

    #[test]
    fn empty_identifier_rejected() {
        assert!(ModelIdentifier::parse("").is_err());
    }

    #[test]
    fn inference_request_validates() {
        let req = InferenceRequest::quick(
            ModelIdentifier::parse("qwen2.5-coder:7b").expect("id"),
            "hello",
        );
        assert!(req.validate().is_ok());
    }

    #[test]
    fn empty_prompt_rejected() {
        let req = InferenceRequest {
            model: ModelIdentifier::parse("qwen2.5-coder:7b").expect("id"),
            prompt: "   ".to_string(),
            max_tokens: 100,
            temperature: 0.2,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn bad_temperature_rejected() {
        let req = InferenceRequest {
            model: ModelIdentifier::parse("qwen2.5-coder:7b").expect("id"),
            prompt: "hi".to_string(),
            max_tokens: 100,
            temperature: 5.0,
        };
        assert!(req.validate().is_err());
    }
}
