// SPDX-License-Identifier: MIT
//! Emergence Module implementation for impforge-bench.

use impforge_emergence::{
    Capability, CapabilityCost, HealthReport, MemoryEntry, MemoryEntryKind, Module, PowerMode,
};

pub struct Module_;

impl Module for Module_ {
    fn id(&self) -> &'static str { "impforge-bench" }

    fn description(&self) -> &'static str {
        "Scientific uplift measurement — bare Ollama vs impforge-cli context, pairwise AB"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::new("bench-run", "run the 4-tier benchmark suite", CapabilityCost::High),
            Capability::new("bench-report", "emit signed JSON report", CapabilityCost::Low),
            Capability::new("bench-cases", "list benchmark cases without running them", CapabilityCost::Zero),
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
            module_id: "impforge-bench".to_string(),
            kind: MemoryEntryKind::SelfHeal,
            summary: "stateless runner".to_string(),
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
        assert_eq!(m.id(), "impforge-bench");
    }

    #[test]
    fn module_declares_three_capabilities() {
        let m = Module_;
        let caps = m.capabilities();
        assert_eq!(caps.len(), 3);
    }

    #[test]
    fn module_starts_in_deep_sleep() {
        let m = Module_;
        assert_eq!(m.power_mode(), PowerMode::DeepSleep);
    }

    #[test]
    fn self_heal_produces_memory_entry() {
        let m = Module_;
        let entry = m.self_heal();
        assert_eq!(entry.module_id, "impforge-bench");
    }
}
