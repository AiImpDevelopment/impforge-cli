// SPDX-License-Identifier: MIT
//! Tool consumers — emit tools in a consumer's native protocol dialect.
//!
//! Each consumer takes `UniversalTool` descriptors out of the registry and
//! exposes them in the shape the downstream expects.  Five dialects are
//! planned:
//!
//! * `mcp` — re-expose as an MCP server (pass-through).
//! * `openai_tools` — OpenAI `tools: [{type: "function", ...}]` shape.
//! * `anthropic_tool_use` — Claude `tool_use` block shape.
//! * `react` (this module's v0) — ReAct text prompt for ANY LLM.
//! * `gbnf` — GBNF grammar for llama.cpp schema-constrained decoding.

pub mod react;
