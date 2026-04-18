// SPDX-License-Identifier: MIT
//! # impforge-universal — universal tool-protocol adapter.
//!
//! Bridges ANY MCP server to ANY model / client through N+M adapters
//! (instead of N×M specialised integrations).  Three traits form the core:
//!
//! * [`ToolProvider`] — ingest tools from a source (MCP stdio client,
//!   OpenAPI spec, Python function wrapper, …).
//! * [`UniversalTool`] — normalised tool representation (name,
//!   JSON-schema, handler).  Routing layer.
//! * [`ToolConsumer`] — emit tools in a consumer's native protocol:
//!   MCP server, OpenAI function-calling, Anthropic tool_use, ReAct
//!   text (for models without native function calling), GBNF-constrained
//!   JSON (llama.cpp grammar).
//!
//! ## Research anchors
//!
//! * MCP Bridge (arXiv 2504.08999) — RESTful proxy aggregating N MCP servers.
//! * ReAct (arXiv 2210.03629) — `Thought→Action→Observation` prompt loop
//!   works on ANY LLM without native function calling.
//! * Natural Language Tools (arXiv 2510.14453) — NL tool-calling restores
//!   27.3 pts lost to JSON-mode on GSM8K.
//! * Schema RL (arXiv 2502.18878) — 98.7% valid JSON via schema-as-reward.
//! * MCPShield (arXiv 2602.14281) — pre-/exec-/post-invocation security.
//!
//! ## Pillar fit (per impforge-cli positioning)
//!
//! 1. Pro preview — parity with ImpForge Pro's mesh routing.
//! 2. AI-tool upgrade — works with Ollama / Cursor / Claude Code instantly.
//! 3. Non-tech on-ramp — users don't need to know what "MCP" is; the
//!    Universal Server makes their local LLM tool-aware out-of-the-box.

pub mod consumers;
pub mod errors;
pub mod providers;
pub mod registry;
pub mod security_gateway;
pub mod tool;

use impforge_emergence::{
    Capability, CapabilityCost, HealthReport, MemoryEntry, MemoryEntryKind, Module, PowerMode,
};

pub use errors::UniversalError;
pub use registry::UniversalToolRegistry;
pub use tool::{
    CapabilityNegotiation, ClientProtocol, ToolCall, ToolInvocationResult, UniversalTool,
};

pub use consumers::anthropic::AnthropicConsumer;
pub use consumers::gbnf::GbnfConsumer;
pub use consumers::gemini::GeminiConsumer;
pub use consumers::mcp_pass::McpPassConsumer;
pub use consumers::openai::OpenAiConsumer;
pub use consumers::react::ReactConsumer;
pub use consumers::ToolConsumer;
pub use providers::mcp_client::McpClientProvider;
pub use providers::mcp_live::McpLiveProvider;
pub use providers::ToolProvider;

pub struct Module_;

impl Module for Module_ {
    fn id(&self) -> &'static str {
        "impforge-universal"
    }

    fn description(&self) -> &'static str {
        "Universal tool-protocol adapter — bridges any MCP server to any model via N+M adapters"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::new(
                "tool-register",
                "register a universal tool in the registry",
                CapabilityCost::Zero,
            ),
            Capability::new(
                "tool-invoke",
                "invoke a universal tool via the chosen consumer protocol",
                CapabilityCost::Medium,
            ),
            Capability::new(
                "protocol-negotiate",
                "negotiate the best protocol for a connecting client",
                CapabilityCost::Zero,
            ),
            Capability::new(
                "react-bridge",
                "ReAct text bridge for models without native function calling",
                CapabilityCost::Low,
            ),
        ]
    }

    fn health(&self) -> HealthReport {
        HealthReport::healthy("universal registry idle", 0)
    }

    fn power_mode(&self) -> PowerMode {
        PowerMode::Idle
    }

    fn self_heal(&self) -> MemoryEntry {
        MemoryEntry {
            module_id: "impforge-universal".to_string(),
            kind: MemoryEntryKind::SelfHeal,
            summary: "restarted universal adapter".to_string(),
            details: None,
            occurred_at_unix: 0,
            quality: 1.0,
        }
    }

    fn is_lazy_mcp(&self) -> bool {
        false
    }
}
