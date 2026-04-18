// SPDX-License-Identifier: MIT
//! # ReAct consumer — turns `UniversalTool` into a text prompt-loop bridge.
//!
//! ReAct (Yao et al. 2022, arXiv 2210.03629) is a `Thought:/Action:/
//! Action Input:/Observation:` loop that works on ANY LLM — including
//! models that have no native function calling (Qwen3-imp, Llama-3.2-3B,
//! Phi-4, Mistral-7B).  This is the **critical bridge** that lets
//! impforge-cli make EVERY Ollama model tool-aware out of the box.
//!
//! ## Pipeline
//!
//! 1. `render_system_prompt(&tools)` → system prompt with a tool catalog +
//!    explicit ReAct format contract.
//! 2. Model emits text.  We call [`parse_next_action`] to extract a
//!    `ToolCall`.
//! 3. We invoke the tool.  Result goes back into the conversation as an
//!    `Observation:` line.
//! 4. Model continues.  Parse next action; loop until `Final Answer:`.
//!
//! ## Research anchors
//!
//! * ReAct (arXiv 2210.03629) — original paper.
//! * Natural Language Tools (arXiv 2510.14453) — NL tool-calling restores
//!   27.3 pts lost to JSON-mode on GSM8K.  We use their recommended format.
//! * Guided-Structured Templates (arXiv 2509.18076) — "Thought-of-Structure"
//!   yields +44.89 % on structured output; we embed the same guidance
//!   inside the system prompt.

use crate::errors::{UniversalError, UniversalResult};
use crate::tool::{ToolCall, ToolInvocationResult, UniversalTool};
use regex::Regex;

/// Renders ReAct prompts and parses model output.
#[derive(Debug, Default, Clone)]
pub struct ReactConsumer;

impl ReactConsumer {
    pub fn new() -> Self {
        Self
    }

    /// Produce the system prompt that teaches the model how to call tools.
    pub fn render_system_prompt(&self, tools: &[UniversalTool]) -> String {
        let mut s = String::new();
        s.push_str("You are an AI assistant that can use tools to answer questions.\n\n");
        s.push_str("RESPOND IN THIS EXACT FORMAT:\n\n");
        s.push_str("Thought: <your reasoning>\n");
        s.push_str("Action: <tool_id from the list below>\n");
        s.push_str("Action Input: <one-line JSON object matching the tool's input schema>\n\n");
        s.push_str("After the user supplies an Observation, you may produce another Thought/Action/Action Input cycle, or finish with:\n\n");
        s.push_str("Thought: I have the answer.\n");
        s.push_str("Final Answer: <your answer to the user's question>\n\n");
        s.push_str("RULES:\n");
        s.push_str("- `Action:` MUST be the exact tool_id (e.g. `filesystem:read_file`).  No quotes, no trailing punctuation.\n");
        s.push_str("- `Action Input:` MUST be one-line valid JSON — no markdown fences, no line breaks.\n");
        s.push_str("- NEVER invent a tool.  If no tool fits, use `Final Answer:` directly.\n\n");
        s.push_str("AVAILABLE TOOLS:\n\n");
        for t in tools {
            s.push_str(&format!("- **{}** — {}\n", t.id, t.description));
            s.push_str(&format!("  input schema: {}\n", t.input_schema));
        }
        if tools.is_empty() {
            s.push_str("  (none — produce Final Answer: directly)\n");
        }
        s
    }

    /// Format a previous tool call's result as a single `Observation:` line
    /// that is fed back into the model's context.
    pub fn render_observation(&self, result: &ToolInvocationResult) -> String {
        let status = if result.ok { "OK" } else { "ERROR" };
        format!(
            "Observation: [{}] {} ({} ms)\n",
            status, result.text, result.elapsed_ms
        )
    }

    /// Parse the next action out of a model response.  Returns `None` if
    /// the model produced `Final Answer:` instead.
    pub fn parse_next_action(&self, model_output: &str) -> UniversalResult<Option<ToolCall>> {
        if model_output.contains("Final Answer:") {
            return Ok(None);
        }
        let action_re = Regex::new(r"(?m)^\s*Action:\s*([^\n]+?)\s*$").map_err(|e| {
            UniversalError::ReactParse(format!("action regex: {e}"))
        })?;
        let input_re = Regex::new(r"(?s)Action Input:\s*(\{.*?\})\s*(?:\n|$)").map_err(|e| {
            UniversalError::ReactParse(format!("input regex: {e}"))
        })?;

        let action = action_re
            .captures(model_output)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| {
                UniversalError::ReactParse("no `Action:` line found".to_string())
            })?;
        let input_raw = input_re
            .captures(model_output)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .ok_or_else(|| {
                UniversalError::ReactParse("no `Action Input:` JSON found".to_string())
            })?;

        let arguments: serde_json::Value = serde_json::from_str(&input_raw).map_err(|e| {
            UniversalError::ReactParse(format!(
                "Action Input is not valid JSON: {e} — got `{input_raw}`"
            ))
        })?;

        Ok(Some(ToolCall {
            tool_id: action,
            arguments,
            call_id: format!("react-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
        }))
    }

    /// Extract the `Final Answer:` content, if any.
    pub fn extract_final_answer(&self, model_output: &str) -> Option<String> {
        if let Some(idx) = model_output.find("Final Answer:") {
            let rest = &model_output[idx + "Final Answer:".len()..];
            Some(rest.trim().to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{ToolCost, UniversalTool};
    use serde_json::json;

    fn read_file_tool() -> UniversalTool {
        UniversalTool {
            id: "filesystem:read_file".to_string(),
            name: "read_file".to_string(),
            description: "Read a file from disk".to_string(),
            input_schema: json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
            output_schema: None,
            source: "filesystem".to_string(),
            cost: ToolCost::Low,
        }
    }

    #[test]
    fn system_prompt_lists_tools() {
        let c = ReactConsumer::new();
        let prompt = c.render_system_prompt(&[read_file_tool()]);
        assert!(prompt.contains("filesystem:read_file"));
        assert!(prompt.contains("Thought:"));
        assert!(prompt.contains("Action:"));
        assert!(prompt.contains("Action Input:"));
        assert!(prompt.contains("Final Answer:"));
    }

    #[test]
    fn system_prompt_handles_empty_tool_list() {
        let c = ReactConsumer::new();
        let prompt = c.render_system_prompt(&[]);
        assert!(prompt.contains("(none"));
    }

    #[test]
    fn parse_action_happy_path() {
        let c = ReactConsumer::new();
        let out = r#"Thought: I need to read README.md.
Action: filesystem:read_file
Action Input: {"path":"README.md"}
"#;
        let call = c.parse_next_action(out).expect("ok").expect("some");
        assert_eq!(call.tool_id, "filesystem:read_file");
        assert_eq!(call.arguments["path"], "README.md");
    }

    #[test]
    fn parse_skips_when_final_answer() {
        let c = ReactConsumer::new();
        let out = "Thought: Done.\nFinal Answer: Hello.";
        assert!(c.parse_next_action(out).expect("ok").is_none());
    }

    #[test]
    fn parse_errors_when_no_action() {
        let c = ReactConsumer::new();
        let out = "Thought: I am thinking...";
        assert!(c.parse_next_action(out).is_err());
    }

    #[test]
    fn parse_errors_when_action_input_not_json() {
        let c = ReactConsumer::new();
        let out = "Action: fs:read\nAction Input: not-json\n";
        assert!(c.parse_next_action(out).is_err());
    }

    #[test]
    fn observation_render_ok() {
        let c = ReactConsumer::new();
        let r = ToolInvocationResult {
            call_id: "c1".to_string(),
            tool_id: "fs:read".to_string(),
            ok: true,
            text: "hello world".to_string(),
            structured: None,
            elapsed_ms: 42,
        };
        let s = c.render_observation(&r);
        assert!(s.starts_with("Observation: [OK] hello world"));
        assert!(s.contains("42 ms"));
    }

    #[test]
    fn observation_render_error() {
        let c = ReactConsumer::new();
        let r = ToolInvocationResult {
            call_id: "c2".to_string(),
            tool_id: "fs:read".to_string(),
            ok: false,
            text: "file not found".to_string(),
            structured: None,
            elapsed_ms: 7,
        };
        let s = c.render_observation(&r);
        assert!(s.contains("[ERROR]"));
    }

    #[test]
    fn extract_final_answer_returns_rest() {
        let c = ReactConsumer::new();
        let out = "Thought: ok.\nFinal Answer: 42 is the answer.";
        assert_eq!(
            c.extract_final_answer(out),
            Some("42 is the answer.".to_string())
        );
    }

    #[test]
    fn extract_final_answer_none_when_absent() {
        let c = ReactConsumer::new();
        assert!(c.extract_final_answer("no final answer here").is_none());
    }

    #[test]
    fn parse_handles_multiline_json_input_one_line() {
        let c = ReactConsumer::new();
        let out = r#"Action: db:query
Action Input: {"sql":"SELECT * FROM users","limit":10}
"#;
        let call = c.parse_next_action(out).expect("ok").expect("some");
        assert_eq!(call.tool_id, "db:query");
        assert_eq!(call.arguments["limit"], 10);
    }
}
