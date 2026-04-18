// SPDX-License-Identifier: MIT
//! Capability discovery types.
//!
//! Instead of every crate reaching across crate boundaries via explicit
//! function calls, a module declares a set of [`Capability`] tags.  The
//! [`crate::Orchestrator`] can then answer "which module can satisfy
//! `scaffold_template`?" at runtime — no compile-time coupling.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    /// Stable tag — kebab-case (e.g. `"scaffold-template"`).
    pub tag: String,
    /// Short description shown in `impforge-cli introspect`.
    pub summary: String,
    /// Cost tier — influences the self-heal bus priority.
    pub cost: CapabilityCost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCost {
    /// In-process, no I/O.  Cheap.
    Zero,
    /// Local filesystem.  Cheap but not free.
    Low,
    /// Local process / network to localhost.
    Medium,
    /// External network (HuggingFace / GitHub).
    High,
}

impl Capability {
    pub fn new(tag: impl Into<String>, summary: impl Into<String>, cost: CapabilityCost) -> Self {
        Self { tag: tag.into(), summary: summary.into(), cost }
    }
}

/// Request envelope when dispatching a capability across modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRequest {
    pub target_capability: String,
    pub payload_json: serde_json::Value,
    pub correlation_id: String,
}

/// Response envelope returned by the module that handled the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityResponse {
    pub handler_module: String,
    pub ok: bool,
    pub payload_json: serde_json::Value,
    pub correlation_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_serializes_roundtrip() {
        let c = Capability::new("scaffold-template", "copy template files", CapabilityCost::Low);
        let j = serde_json::to_string(&c).expect("serialize");
        let back: Capability = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(c, back);
    }

    #[test]
    fn cost_ordering_is_sensible() {
        let costs = [
            CapabilityCost::Zero,
            CapabilityCost::Low,
            CapabilityCost::Medium,
            CapabilityCost::High,
        ];
        assert_eq!(costs.len(), 4);
    }
}
