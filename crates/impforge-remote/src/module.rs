// SPDX-License-Identifier: MIT
//! Emergence Module for impforge-remote.

use impforge_emergence::{
    Capability, CapabilityCost, HealthReport, MemoryEntry, MemoryEntryKind, Module, PowerMode,
};

pub struct Module_;

impl Module for Module_ {
    fn id(&self) -> &'static str { "impforge-remote" }

    fn description(&self) -> &'static str {
        "Signal / Telegram / WhatsApp bridge — send commands from phone to your local CLI"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::new("remote-start", "start the bridge daemon", CapabilityCost::Medium),
            Capability::new("remote-status", "report bridge configuration + reachability", CapabilityCost::Low),
            Capability::new("remote-send", "dispatch a reply to a bridge message", CapabilityCost::Medium),
        ]
    }

    fn health(&self) -> HealthReport {
        HealthReport::healthy("idle", 0)
    }

    fn power_mode(&self) -> PowerMode {
        PowerMode::DeepSleep
    }

    fn self_heal(&self) -> MemoryEntry {
        MemoryEntry {
            module_id: "impforge-remote".to_string(),
            kind: MemoryEntryKind::SelfHeal,
            summary: "rebuilt bridge client".to_string(),
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
    fn module_id_matches_crate_name() {
        let m = Module_;
        assert_eq!(m.id(), "impforge-remote");
    }

    #[test]
    fn module_declares_three_capabilities() {
        assert_eq!(Module_.capabilities().len(), 3);
    }

    #[test]
    fn module_starts_in_deep_sleep() {
        assert_eq!(Module_.power_mode(), PowerMode::DeepSleep);
    }

    #[test]
    fn self_heal_is_stateless_ack() {
        let entry = Module_.self_heal();
        assert_eq!(entry.kind, MemoryEntryKind::SelfHeal);
    }
}
