// SPDX-License-Identifier: MIT
//! The canonical `Module` trait every workspace crate implements.

use crate::capability::Capability;
use crate::health::HealthReport;
use crate::memory::MemoryEntry;
use serde::{Deserialize, Serialize};

/// Power modes for a module — directly mirrors ImpForge's
/// `module_lifecycle` pattern.  The default after install is `DeepSleep`
/// for every module, so the CLI consumes almost zero memory when idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerMode {
    /// <1 MB, zero background CPU.  Metadata only.
    #[default]
    DeepSleep,
    /// <5 MB, lazy-loaded state.  Ready to wake quickly.
    Idle,
    /// ~20 MB, fully loaded and serving requests.
    Active,
    /// Fully loaded + warm caches + worker threads spun up.
    Full,
}

impl PowerMode {
    pub fn memory_budget_mb(self) -> u64 {
        match self {
            PowerMode::DeepSleep => 1,
            PowerMode::Idle => 5,
            PowerMode::Active => 20,
            PowerMode::Full => 80,
        }
    }

    pub fn can_serve(self) -> bool {
        matches!(self, PowerMode::Active | PowerMode::Full)
    }
}

/// A crate-level "micro-program" in the emergence graph.  Every workspace
/// crate implements this trait so the [`crate::Orchestrator`] can register
/// it, monitor its health, dispatch capability requests, and suspend/resume
/// the module on demand.
pub trait Module: Send + Sync {
    /// Stable module id — matches the crate name (e.g. `"impforge-scaffold"`).
    fn id(&self) -> &'static str;

    /// Short human-readable description.
    fn description(&self) -> &'static str;

    /// List of capabilities this module provides.
    fn capabilities(&self) -> Vec<Capability>;

    /// Report current health.  The runtime calls this on every tick.
    fn health(&self) -> HealthReport;

    /// Current power mode.
    fn power_mode(&self) -> PowerMode {
        PowerMode::Idle
    }

    /// Transition to a new power mode.  Default implementation is a no-op;
    /// modules with heavy resources should implement actual suspend logic.
    fn set_power_mode(&self, _new_mode: PowerMode) -> MemoryEntry {
        MemoryEntry {
            module_id: self.id().to_string(),
            kind: crate::memory::MemoryEntryKind::HealthCheck,
            summary: "power mode change (no-op)".to_string(),
            details: None,
            occurred_at_unix: 0,
            quality: 1.0,
        }
    }

    /// Attempt to repair a degraded state.  Returns a `MemoryEntry` that
    /// records the outcome.
    fn self_heal(&self) -> MemoryEntry;

    /// Optional episodic memory snapshot — the module can describe what
    /// it just did in one line.
    fn memory_snapshot(&self) -> Option<MemoryEntry> {
        None
    }

    /// Whether this module is a lazy MCP-tool provider.  Tools from lazy
    /// modules are listed by name only until a client actually invokes
    /// them — saves ~90% of tokens when paired with AI-coding clients.
    fn is_lazy_mcp(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_power_mode_is_deep_sleep() {
        assert_eq!(PowerMode::default(), PowerMode::DeepSleep);
    }

    #[test]
    fn deep_sleep_cannot_serve_requests() {
        assert!(!PowerMode::DeepSleep.can_serve());
        assert!(!PowerMode::Idle.can_serve());
        assert!(PowerMode::Active.can_serve());
        assert!(PowerMode::Full.can_serve());
    }

    #[test]
    fn memory_budgets_are_monotonic() {
        assert!(
            PowerMode::DeepSleep.memory_budget_mb()
                < PowerMode::Idle.memory_budget_mb()
        );
        assert!(
            PowerMode::Idle.memory_budget_mb()
                < PowerMode::Active.memory_budget_mb()
        );
        assert!(
            PowerMode::Active.memory_budget_mb()
                < PowerMode::Full.memory_budget_mb()
        );
    }
}
