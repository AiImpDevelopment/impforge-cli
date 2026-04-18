// SPDX-License-Identifier: MIT
//! Unified local-model client.
//!
//! Under the hood this crate speaks to:
//! - **Ollama** via `ollama-rs` (HTTP, default)
//! - **HuggingFace Hub** via `hf-hub` (model download)
//! - **llama.cpp** via FFI (opt-in `llamacpp` feature)
//! - **Candle** (opt-in `candle` feature) — pure-Rust inference
//!
//! The CLI picks the backend based on the user's `CliConfig`.

use impforge_emergence::{
    Capability, CapabilityCost, HealthReport, MemoryEntry, MemoryEntryKind, Module, PowerMode,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

pub mod backend;
pub mod ollama;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListItem {
    pub id: String,
    pub backend: String,
    pub size_bytes: Option<u64>,
    pub family: Option<String>,
}

/// Singleton Module implementer for `impforge-models`.
pub struct Module_;

#[allow(non_upper_case_globals)]
static POWER: AtomicU8 = AtomicU8::new(0); // 0 = DeepSleep

fn power_from_u8(n: u8) -> PowerMode {
    match n {
        0 => PowerMode::DeepSleep,
        1 => PowerMode::Idle,
        2 => PowerMode::Active,
        _ => PowerMode::Full,
    }
}

fn power_to_u8(mode: PowerMode) -> u8 {
    match mode {
        PowerMode::DeepSleep => 0,
        PowerMode::Idle => 1,
        PowerMode::Active => 2,
        PowerMode::Full => 3,
    }
}

impl Module for Module_ {
    fn id(&self) -> &'static str { "impforge-models" }

    fn description(&self) -> &'static str {
        "Local-model client — Ollama / HuggingFace / llama.cpp / Candle"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::new("model-list", "list local models", CapabilityCost::Low),
            Capability::new("model-pull", "pull a model from a backend", CapabilityCost::High),
            Capability::new("model-run", "run inference on a local model", CapabilityCost::Medium),
            Capability::new("model-benchmark", "benchmark local hardware", CapabilityCost::Low),
        ]
    }

    fn health(&self) -> HealthReport {
        HealthReport::healthy("model client idle", 0)
    }

    fn power_mode(&self) -> PowerMode {
        power_from_u8(POWER.load(Ordering::Relaxed))
    }

    fn set_power_mode(&self, mode: PowerMode) -> MemoryEntry {
        POWER.store(power_to_u8(mode), Ordering::Relaxed);
        MemoryEntry {
            module_id: "impforge-models".to_string(),
            kind: MemoryEntryKind::HealthCheck,
            summary: format!("power → {mode:?}"),
            details: None,
            occurred_at_unix: 0,
            quality: 1.0,
        }
    }

    fn self_heal(&self) -> MemoryEntry {
        MemoryEntry {
            module_id: "impforge-models".to_string(),
            kind: MemoryEntryKind::SelfHeal,
            summary: "no-op (stateless client)".to_string(),
            details: None,
            occurred_at_unix: 0,
            quality: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_include_list_and_run() {
        let m = Module_;
        let caps = m.capabilities();
        let tags: Vec<&str> = caps.iter().map(|c| c.tag.as_str()).collect();
        assert!(tags.contains(&"model-list"));
        assert!(tags.contains(&"model-run"));
    }

    #[test]
    fn power_mode_roundtrips() {
        let m = Module_;
        m.set_power_mode(PowerMode::Active);
        assert_eq!(m.power_mode(), PowerMode::Active);
        m.set_power_mode(PowerMode::DeepSleep);
        assert_eq!(m.power_mode(), PowerMode::DeepSleep);
    }
}
