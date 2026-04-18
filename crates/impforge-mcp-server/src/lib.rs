// SPDX-License-Identifier: MIT
//! Lazy MCP stdio server for impforge-cli.
//!
//! ## Why "lazy"?
//!
//! Traditional MCP servers expose every tool's full JSON schema at
//! registration time.  With 50+ tools that can easily mean ~40 KB of
//! schema per connection — consuming tokens in the AI client's context
//! window before the first real tool call.
//!
//! We list **tool names only** at registration (~2 KB), and only emit the
//! full schema when the client calls `tools/<tool>/schema`.  This cuts
//! token usage by up to 90 %.

pub mod auto_suspend;
pub mod lazy_schema;
pub mod registration;
pub mod server;
pub mod transport;

use impforge_emergence::{
    Capability, CapabilityCost, HealthReport, MemoryEntry, MemoryEntryKind, Module, PowerMode,
};

pub struct Module_;

impl Module for Module_ {
    fn id(&self) -> &'static str { "impforge-mcp-server" }

    fn description(&self) -> &'static str {
        "Lazy MCP stdio server — 90% token savings via on-demand schema loading"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::new("mcp-serve", "start stdio MCP server", CapabilityCost::Medium),
            Capability::new("mcp-register-client", "generate config for AI client", CapabilityCost::Low),
            Capability::new("mcp-list-tools", "list tool names only (lazy)", CapabilityCost::Zero),
        ]
    }

    fn health(&self) -> HealthReport {
        HealthReport::healthy("MCP server idle", 0)
    }

    fn power_mode(&self) -> PowerMode { PowerMode::Idle }

    fn self_heal(&self) -> MemoryEntry {
        MemoryEntry {
            module_id: "impforge-mcp-server".to_string(),
            kind: MemoryEntryKind::SelfHeal,
            summary: "restarted MCP server".to_string(),
            details: None,
            occurred_at_unix: 0,
            quality: 1.0,
        }
    }

    fn is_lazy_mcp(&self) -> bool { true }
}
