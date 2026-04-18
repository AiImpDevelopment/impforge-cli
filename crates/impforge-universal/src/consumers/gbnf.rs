// SPDX-License-Identifier: MIT
//! GBNF grammar consumer for llama.cpp schema-constrained decoding.

use crate::consumers::ToolConsumer;
use crate::errors::{UniversalError, UniversalResult};
use crate::tool::{ToolCall, UniversalTool};
use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct GbnfConsumer;

impl GbnfConsumer {
    pub fn new() -> Self {
        Self
    }

    pub fn render_grammar(&self, tools: &[UniversalTool]) -> String {
        let mut s = String::new();
        s.push_str("root ::= \"{\" space \"\\\"tool\\\"\" space \":\" space tool \",\" space \"\\\"args\\\"\" space \":\" space object space \"}\"\n");
        s.push_str("tool ::= ");
        if tools.is_empty() {
            s.push_str("string\n");
        } else {
            let alts: Vec<String> =
                tools.iter().map(|t| format!("\"\\\"{}\\\"\"", t.id)).collect();
            s.push_str(&alts.join(" | "));
            s.push('\n');
        }
        s.push_str("object ::= \"{\" space (string space \":\" space value (space \",\" space string space \":\" space value)*)? space \"}\"\n");
        s.push_str("value ::= string | number | object | array | \"true\" | \"false\" | \"null\"\n");
        s.push_str("array ::= \"[\" space (value (space \",\" space value)*)? space \"]\"\n");
        s.push_str("string ::= \"\\\"\" ([^\"\\\\] | \"\\\\\" .)* \"\\\"\"\n");
        s.push_str("number ::= \"-\"? [0-9]+ (\".\" [0-9]+)?\n");
        s.push_str("space ::= [ \\t\\n]*\n");
        s
    }
}

impl ToolConsumer for GbnfConsumer {
    fn dialect(&self) -> &'static str {
        "gbnf_json"
    }

    fn render_catalog(&self, tools: &[UniversalTool]) -> UniversalResult<String> {
        Ok(self.render_grammar(tools))
    }

    fn parse_call(&self, model_output: &str) -> UniversalResult<Option<ToolCall>> {
        let v: Value = serde_json::from_str(model_output.trim())?;
        let tool_id = v
            .get("tool")
            .and_then(|x| x.as_str())
            .ok_or_else(|| UniversalError::Consumer("missing `tool` field".to_string()))?
            .to_string();
        let arguments = v.get("args").cloned().unwrap_or(serde_json::json!({}));
        Ok(Some(ToolCall {
            tool_id,
            arguments,
            call_id: "gbnf-call".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolCost;
    use serde_json::json;

    fn sample() -> UniversalTool {
        UniversalTool {
            id: "fs:read".to_string(),
            name: "read".to_string(),
            description: "r".to_string(),
            input_schema: json!({"type":"object"}),
            output_schema: None,
            source: "fs".to_string(),
            cost: ToolCost::Low,
        }
    }

    #[test]
    fn grammar_contains_tool_alt() {
        let g = GbnfConsumer::new().render_grammar(&[sample()]);
        assert!(g.contains("fs:read"));
    }

    #[test]
    fn empty_tools_fallback_to_string() {
        let g = GbnfConsumer::new().render_grammar(&[]);
        assert!(g.contains("tool ::= string"));
    }

    #[test]
    fn parse_happy_path() {
        let out = r#"{"tool":"fs:read","args":{"p":"x"}}"#;
        let c = GbnfConsumer::new().parse_call(out).expect("ok").expect("some");
        assert_eq!(c.tool_id, "fs:read");
    }

    #[test]
    fn parse_err_on_missing_tool() {
        assert!(GbnfConsumer::new().parse_call(r#"{"args":{}}"#).is_err());
    }

    #[test]
    fn dialect_name() {
        assert_eq!(GbnfConsumer::new().dialect(), "gbnf_json");
    }
}
