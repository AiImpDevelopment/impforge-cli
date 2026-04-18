// SPDX-License-Identifier: MIT
//! Anthropic tool_use consumer — Claude Messages `tools: [...]`.

use crate::consumers::ToolConsumer;
use crate::errors::{UniversalError, UniversalResult};
use crate::tool::{ToolCall, UniversalTool};
use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct AnthropicConsumer;

impl AnthropicConsumer {
    pub fn new() -> Self {
        Self
    }
}

impl ToolConsumer for AnthropicConsumer {
    fn dialect(&self) -> &'static str {
        "anthropic_tool_use"
    }

    fn render_catalog(&self, tools: &[UniversalTool]) -> UniversalResult<String> {
        let items: Vec<Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.id,
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            })
            .collect();
        serde_json::to_string(&items).map_err(UniversalError::Serde)
    }

    fn parse_call(&self, model_output: &str) -> UniversalResult<Option<ToolCall>> {
        let v: Value = serde_json::from_str(model_output)?;
        let Some(arr) = v.get("content").and_then(|x| x.as_array()) else {
            return Ok(None);
        };
        for block in arr {
            if block.get("type").and_then(|x| x.as_str()) == Some("tool_use") {
                let tool_id = block
                    .get("name")
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| {
                        UniversalError::Consumer("tool_use.name missing".to_string())
                    })?
                    .to_string();
                let arguments = block.get("input").cloned().unwrap_or(serde_json::json!({}));
                let call_id = block
                    .get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("anthropic-call")
                    .to_string();
                return Ok(Some(ToolCall {
                    tool_id,
                    arguments,
                    call_id,
                }));
            }
        }
        Ok(None)
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
    fn render_uses_input_schema_key() {
        let s = AnthropicConsumer::new().render_catalog(&[sample()]).expect("ok");
        assert!(s.contains("\"input_schema\""));
        assert!(s.contains("\"name\":\"fs:read\""));
    }

    #[test]
    fn parse_extracts_tool_use_block() {
        let out = r#"{
            "content":[
                {"type":"text","text":"I'll read"},
                {"type":"tool_use","id":"toolu_1","name":"fs:read","input":{"path":"a"}}
            ]
        }"#;
        let c = AnthropicConsumer::new()
            .parse_call(out)
            .expect("ok")
            .expect("some");
        assert_eq!(c.tool_id, "fs:read");
        assert_eq!(c.call_id, "toolu_1");
    }

    #[test]
    fn parse_none_without_tool_use() {
        let out = r#"{"content":[{"type":"text","text":"hi"}]}"#;
        assert!(AnthropicConsumer::new()
            .parse_call(out)
            .expect("ok")
            .is_none());
    }

    #[test]
    fn dialect_name() {
        assert_eq!(AnthropicConsumer::new().dialect(), "anthropic_tool_use");
    }
}
