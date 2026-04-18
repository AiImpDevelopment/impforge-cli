// SPDX-License-Identifier: MIT
//! Community-contribution wizard.
//!
//! When a user runs `impforge-cli contribute template`, this module:
//! 1. Prompts for manifest fields (interactive).
//! 2. Validates via `impforge-core::TemplateManifest::validate`.
//! 3. Runs a security scan over supplied files.
//! 4. Scrubs free-text fields for prompt-injection patterns.
//! 5. Generates a GitHub PR body + contribution diff.
//! 6. Opens the user's browser to the PR page.

use impforge_emergence::{
    Capability, CapabilityCost, HealthReport, MemoryEntry, MemoryEntryKind, Module, PowerMode,
};

pub mod pr_builder;
pub mod validation;

pub struct Module_;

impl Module for Module_ {
    fn id(&self) -> &'static str { "impforge-contribute" }
    fn description(&self) -> &'static str {
        "Community-contribution wizard with local validation + PR builder"
    }
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::new("contribute-template", "submit a new template", CapabilityCost::Medium),
            Capability::new("contribute-skill", "submit a new skill", CapabilityCost::Medium),
            Capability::new("contribute-mcp", "submit a new MCP manifest", CapabilityCost::Medium),
        ]
    }
    fn health(&self) -> HealthReport { HealthReport::healthy("idle", 0) }
    fn power_mode(&self) -> PowerMode { PowerMode::DeepSleep }
    fn self_heal(&self) -> MemoryEntry {
        MemoryEntry {
            module_id: "impforge-contribute".to_string(),
            kind: MemoryEntryKind::SelfHeal,
            summary: "cleared stale draft".to_string(),
            details: None,
            occurred_at_unix: 0,
            quality: 1.0,
        }
    }
}
