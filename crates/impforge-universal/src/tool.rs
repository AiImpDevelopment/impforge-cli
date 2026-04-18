// SPDX-License-Identifier: MIT
//! Universal tool representation + capability-negotiation types.
//!
//! A `UniversalTool` is the lingua franca the registry stores.  Every
//! provider (MCP, OpenAPI, Python fn) converts INTO this shape; every
//! consumer (MCP server, OpenAI, Anthropic, ReAct, GBNF) converts FROM
//! this shape.  N×M specialised bridges become N+M adapters — the
//! N×(1/M) amplification this crate exists for.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Normalised tool descriptor shared across every provider + consumer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniversalTool {
    /// Globally-unique namespaced id: `{server_id}:{tool_name}`.
    /// Prevents name collisions across 90+ MCP servers.
    pub id: String,
    /// Human-readable tool name (the unprefixed last segment of `id`).
    pub name: String,
    pub description: String,
    /// JSON Schema (Draft 2020-12) for input arguments.
    pub input_schema: Value,
    /// Optional JSON Schema for the output shape (helps GBNF-constrained
    /// consumers).
    pub output_schema: Option<Value>,
    /// The server/source the tool originated from — e.g. `"filesystem"`,
    /// `"github"`, `"openapi:sentry"`.  Used by the security gateway to
    /// apply per-source policy.
    pub source: String,
    /// Cost hint — lets the registry prioritise cheaper tools when an
    /// intent matches multiple providers.
    pub cost: ToolCost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolCost {
    /// Purely local, no I/O, microseconds.
    Zero,
    /// Local I/O (filesystem, SQLite) — milliseconds.
    #[default]
    Low,
    /// Local LLM call / vector search — tens of ms.
    Medium,
    /// Cloud call / expensive computation.
    High,
}

/// A call invocation that the consumer → registry → provider path carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub tool_id: String,
    pub arguments: Value,
    /// Correlation id the consumer supplies (opaque to the registry).
    pub call_id: String,
}

/// Result of a tool invocation.  Text-first (largest compatibility surface
/// with ReAct models), but structured results are also carried for native
/// JSON consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInvocationResult {
    pub call_id: String,
    pub tool_id: String,
    pub ok: bool,
    pub text: String,
    pub structured: Option<Value>,
    pub elapsed_ms: u64,
}

/// Client-declared protocol dialect — drives which consumer the registry
/// routes the call through.  Follows the capability-negotiation handshake
/// from the research (MCP Bridge 2504.08999, Natural-Language-Tools 2510.14453).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientProtocol {
    /// Native MCP client (Claude Desktop, Cursor, Continue, Zed, etc.).
    Mcp,
    /// OpenAI Chat Completions with `tools: [...]` (OpenAI, Groq, Together,
    /// vLLM's OpenAI-compatible endpoint, LM Studio, any OAI-compat model).
    OpenAiTools,
    /// Anthropic Messages with `tool_use` blocks (Claude direct or
    /// via Bedrock).
    AnthropicToolUse,
    /// ReAct text — `Thought:/Action:/Action Input:/Observation:` prompt
    /// loop.  Works on ANY LLM, no native function calling needed.
    /// Ideal for Qwen3-imp, Llama-3.2-3B, Phi-4, any Ollama model.
    ReactText,
    /// llama.cpp GBNF-constrained JSON (98.7 % valid-JSON, Schema-RL 2502.18878).
    /// Used when we have a llama.cpp handle and want strict output shape.
    GbnfJson,
}

impl ClientProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            ClientProtocol::Mcp => "mcp",
            ClientProtocol::OpenAiTools => "openai_tools",
            ClientProtocol::AnthropicToolUse => "anthropic_tool_use",
            ClientProtocol::ReactText => "react_text",
            ClientProtocol::GbnfJson => "gbnf_json",
        }
    }

    /// Does this protocol require the universal server to handle
    /// tool-call parsing out of raw text?  True for ReAct; false for the
    /// structured protocols.
    pub fn needs_text_parser(self) -> bool {
        matches!(self, ClientProtocol::ReactText)
    }
}

/// Capability-negotiation declaration from the client.  The server picks
/// a `ClientProtocol` based on this:
///
/// * `native_tools=true` → MCP / OpenAI / Anthropic (whichever matched).
/// * `native_tools=false` + `gbnf_support=true` → GBNF.
/// * `native_tools=false` + `gbnf_support=false` → ReAct text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityNegotiation {
    pub client_name: String,
    pub native_tools: bool,
    pub gbnf_support: bool,
    /// Max tokens the client can consume in a single response.
    pub max_response_tokens: u32,
    /// Does the client handle streaming?
    pub streaming: bool,
}

impl CapabilityNegotiation {
    /// Pick the best protocol for this client.  Rule:
    ///
    /// * MCP if the client is known to speak MCP natively.
    /// * OpenAI tools if the client is OpenAI-compatible.
    /// * Anthropic tool_use if the client is Claude-native.
    /// * GBNF if no native but grammar-constrained decoding supported.
    /// * ReAct text as the universal last-resort.
    pub fn pick_protocol(&self) -> ClientProtocol {
        let lname = self.client_name.to_ascii_lowercase();
        if lname.contains("claude") || lname.contains("anthropic") {
            return ClientProtocol::AnthropicToolUse;
        }
        if lname.contains("gpt") || lname.contains("openai") || lname.contains("groq") {
            return ClientProtocol::OpenAiTools;
        }
        if lname.contains("mcp") || lname.contains("cursor") || lname.contains("continue") {
            return ClientProtocol::Mcp;
        }
        if self.native_tools {
            return ClientProtocol::OpenAiTools;
        }
        if self.gbnf_support {
            return ClientProtocol::GbnfJson;
        }
        ClientProtocol::ReactText
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_tool() -> UniversalTool {
        UniversalTool {
            id: "filesystem:read_file".to_string(),
            name: "read_file".to_string(),
            description: "Read a file from disk".to_string(),
            input_schema: json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
            output_schema: None,
            source: "filesystem".to_string(),
            cost: ToolCost::Low,
        }
    }

    #[test]
    fn default_cost_is_low() {
        assert_eq!(ToolCost::default(), ToolCost::Low);
    }

    #[test]
    fn tool_serde_roundtrip() {
        let t = sample_tool();
        let j = serde_json::to_string(&t).expect("ser");
        let back: UniversalTool = serde_json::from_str(&j).expect("de");
        assert_eq!(t, back);
    }

    #[test]
    fn claude_client_routes_to_anthropic() {
        let nego = CapabilityNegotiation {
            client_name: "Claude Desktop 0.9".to_string(),
            native_tools: true,
            gbnf_support: false,
            max_response_tokens: 4096,
            streaming: true,
        };
        assert_eq!(nego.pick_protocol(), ClientProtocol::AnthropicToolUse);
    }

    #[test]
    fn cursor_client_routes_to_mcp() {
        let nego = CapabilityNegotiation {
            client_name: "Cursor IDE".to_string(),
            native_tools: true,
            gbnf_support: false,
            max_response_tokens: 8192,
            streaming: true,
        };
        assert_eq!(nego.pick_protocol(), ClientProtocol::Mcp);
    }

    #[test]
    fn openai_gpt_routes_to_openai_tools() {
        let nego = CapabilityNegotiation {
            client_name: "GPT-4o".to_string(),
            native_tools: true,
            gbnf_support: false,
            max_response_tokens: 4096,
            streaming: true,
        };
        assert_eq!(nego.pick_protocol(), ClientProtocol::OpenAiTools);
    }

    #[test]
    fn no_native_no_grammar_falls_back_to_react() {
        let nego = CapabilityNegotiation {
            client_name: "Qwen3-imp 8B".to_string(),
            native_tools: false,
            gbnf_support: false,
            max_response_tokens: 4096,
            streaming: true,
        };
        assert_eq!(nego.pick_protocol(), ClientProtocol::ReactText);
    }

    #[test]
    fn no_native_but_gbnf_uses_gbnf() {
        let nego = CapabilityNegotiation {
            client_name: "llama.cpp local".to_string(),
            native_tools: false,
            gbnf_support: true,
            max_response_tokens: 8192,
            streaming: false,
        };
        assert_eq!(nego.pick_protocol(), ClientProtocol::GbnfJson);
    }

    #[test]
    fn react_needs_text_parser_others_dont() {
        assert!(ClientProtocol::ReactText.needs_text_parser());
        assert!(!ClientProtocol::Mcp.needs_text_parser());
        assert!(!ClientProtocol::OpenAiTools.needs_text_parser());
    }

    #[test]
    fn tool_id_follows_namespaced_pattern() {
        let t = sample_tool();
        assert!(t.id.contains(':'));
        let parts: Vec<&str> = t.id.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], t.source);
        assert_eq!(parts[1], t.name);
    }
}
