// SPDX-License-Identifier: MIT
//! Google Gemini function-calling consumer.

use crate::consumers::ToolConsumer;
use crate::errors::{UniversalError, UniversalResult};
use crate::tool::{ToolCall, UniversalTool};
use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct GeminiConsumer;

impl GeminiConsumer {
    pub fn new() -> Self {
        Self
    }
}

impl ToolConsumer for GeminiConsumer {
    fn dialect(&self) -> &'static str {
        "gemini"
    }

    fn render_catalog(&self, tools: &[UniversalTool]) -> UniversalResult<String> {
        let declarations: Vec<Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.id,
                    "description": t.description,
                    "parameters": t.input_schema,
                })
            })
            .collect();
        let wrapped = serde_json::json!({"functionDeclarations": declarations});
        serde_json::to_string(&wrapped).map_err(UniversalError::Serde)
    }

    fn parse_call(&self, model_output: &str) -> UniversalResult<Option<ToolCall>> {
        let v: Value = serde_json::from_str(model_output)?;
        let Some(arr) = v
            .pointer("/candidates/0/content/parts")
            .and_then(|x| x.as_array())
        else {
            return Ok(None);
        };
        for part in arr {
            if let Some(fc) = part.get("functionCall") {
                let tool_id = fc
                    .get("name")
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| {
                        UniversalError::Consumer("functionCall.name missing".to_string())
                    })?
                    .to_string();
                let arguments = fc.get("args").cloned().unwrap_or(serde_json::json!({}));
                return Ok(Some(ToolCall {
                    tool_id,
                    arguments,
                    call_id: "gemini-call".to_string(),
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
    fn render_uses_function_declarations() {
        let s = GeminiConsumer::new().render_catalog(&[sample()]).expect("ok");
        assert!(s.contains("functionDeclarations"));
        assert!(s.contains("\"name\":\"fs:read\""));
    }

    #[test]
    fn parse_extracts_function_call() {
        let out = r#"{
            "candidates":[{"content":{"parts":[
                {"text":"let me read"},
                {"functionCall":{"name":"fs:read","args":{"path":"a"}}}
            ]}}]
        }"#;
        let c = GeminiConsumer::new().parse_call(out).expect("ok").expect("some");
        assert_eq!(c.tool_id, "fs:read");
    }

    #[test]
    fn parse_none_without_call() {
        let out = r#"{"candidates":[{"content":{"parts":[{"text":"hi"}]}}]}"#;
        assert!(GeminiConsumer::new().parse_call(out).expect("ok").is_none());
    }

    #[test]
    fn dialect_name() {
        assert_eq!(GeminiConsumer::new().dialect(), "gemini");
    }
}
