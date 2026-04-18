// SPDX-License-Identifier: MIT
//! impforge-cli persistent configuration — stored in `~/.impforge-cli/config.json`.

use crate::error::{CoreError, CoreResult};
use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliConfig {
    pub schema_version: u32,
    #[serde(default)]
    pub telemetry_opt_in: bool,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub default_model_backend: ModelBackend,
    #[serde(default)]
    pub autopilot_enabled: bool,
    #[serde(default)]
    pub update_check_interval_hours: u32,
    #[serde(default)]
    pub last_update_check_unix: i64,
    #[serde(default)]
    pub registered_mcp_clients: Vec<String>,
    #[serde(default)]
    pub hf_token: Option<String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            telemetry_opt_in: false,
            default_model: None,
            default_model_backend: ModelBackend::default(),
            autopilot_enabled: false,
            update_check_interval_hours: 24,
            last_update_check_unix: 0,
            registered_mcp_clients: Vec::new(),
            hf_token: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelBackend {
    #[default]
    Ollama,
    LlamaCpp,
    Candle,
    HuggingFace,
}

impl CliConfig {
    pub fn path() -> CoreResult<PathBuf> {
        Ok(paths::config_dir()?.join("config.json"))
    }

    pub fn load() -> CoreResult<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save(&self) -> CoreResult<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(&path, raw)?;
        Ok(())
    }

    pub fn set_default_model(&mut self, model: impl Into<String>, backend: ModelBackend) {
        self.default_model = Some(model.into());
        self.default_model_backend = backend;
    }

    pub fn enable_autopilot(&mut self) {
        self.autopilot_enabled = true;
    }

    pub fn disable_autopilot(&mut self) {
        self.autopilot_enabled = false;
    }

    pub fn validate(&self) -> CoreResult<()> {
        if self.schema_version != 1 {
            return Err(CoreError::validation(format!(
                "unsupported config schema version {}",
                self.schema_version
            )));
        }
        if self.update_check_interval_hours > 720 {
            return Err(CoreError::validation(
                "update check interval must be <= 720 hours (30 days)",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_validates() {
        assert!(CliConfig::default().validate().is_ok());
    }

    #[test]
    fn bad_interval_rejected() {
        let mut c = CliConfig::default();
        c.update_check_interval_hours = 10_000;
        assert!(c.validate().is_err());
    }

    #[test]
    fn roundtrips_through_json() {
        let c = CliConfig {
            default_model: Some("qwen2.5-coder:7b".to_string()),
            default_model_backend: ModelBackend::Ollama,
            autopilot_enabled: true,
            ..Default::default()
        };
        let raw = serde_json::to_string(&c).expect("serialize");
        let back: CliConfig = serde_json::from_str(&raw).expect("deserialize");
        assert_eq!(c, back);
    }
}
