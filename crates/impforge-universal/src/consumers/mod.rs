// SPDX-License-Identifier: MIT
//! Tool consumers — emit tools in a consumer's native protocol dialect.
//!
//! Each consumer takes `UniversalTool` descriptors out of the registry and
//! exposes them in the shape the downstream expects.  Six dialects shipped:
//!
//! * `mcp` — re-expose as an MCP server (pass-through).
//! * `openai_tools` — OpenAI `tools: [{type: "function", ...}]` shape.
//! * `anthropic_tool_use` — Claude `tool_use` block shape.
//! * `gemini` — Google Gemini function-calling.
//! * `react` — ReAct text prompt for ANY LLM (killer feature).
//! * `gbnf` — GBNF grammar for llama.cpp schema-constrained decoding.

pub mod anthropic;
pub mod gbnf;
pub mod gemini;
pub mod mcp_pass;
pub mod openai;
pub mod react;

use crate::errors::UniversalResult;
use crate::tool::{ToolCall, UniversalTool};

/// Emit a catalog in a specific native dialect + parse a model response
/// back into a `ToolCall`.
pub trait ToolConsumer: Send + Sync {
    fn dialect(&self) -> &'static str;
    fn render_catalog(&self, tools: &[UniversalTool]) -> UniversalResult<String>;
    fn parse_call(&self, model_output: &str) -> UniversalResult<Option<ToolCall>>;
}
