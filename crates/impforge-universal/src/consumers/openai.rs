// SPDX-License-Identifier: MIT
//! OpenAI tools consumer — emits `tools: [{type:"function", ...}]` array.

use crate::consumers::ToolConsumer;
use crate::errors::{UniversalError, UniversalResult};
use crate::tool::{ToolCall, UniversalTool};
use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct OpenAiConsumer;

impl OpenAiConsumer {
    pub fn new() -> Self {
        Self
    }
}

impl ToolConsumer for OpenAiConsumer {
    fn dialect(&self) -> &'static str {
        "openai_tools"
    }

    fn render_catalog(&self, tools: &[UniversalTool]) -> UniversalResult<String> {
        let items: Vec<Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.id,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            })
            .collect();
        serde_json::to_string(&items).map_err(UniversalError::Serde)
    }

    fn parse_call(&self, model_output: &str) -> UniversalResult<Option<ToolCall>> {
        let v: Value = serde_json::from_str(model_output)?;
        let tool_calls = v
            .pointer("/choices/0/message/tool_calls")
            .and_then(|x| x.as_array());
        let Some(tc) = tool_calls.and_then(|a| a.first()) else {
            return Ok(None);
        };
        let fun = tc.get("function").ok_or_else(|| {
            UniversalError::Consumer("tool_calls[0].function missing".to_string())
        })?;
        let tool_id = fun
            .get("name")
            .and_then(|x| x.as_str())
            .ok_or_else(|| UniversalError::Consumer("function.name missing".to_string()))?
            .to_string();
        let args_str = fun
            .get("arguments")
            .and_then(|x| x.as_str())
            .unwrap_or("{}");
        let arguments: Value = serde_json::from_str(args_str)?;
        let call_id = tc
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or("openai-call")
            .to_string();
        Ok(Some(ToolCall {
            tool_id,
            arguments,
            call_id,
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
            description: "read".to_string(),
            input_schema: json!({"type":"object"}),
            output_schema: None,
            source: "fs".to_string(),
            cost: ToolCost::Low,
        }
    }

    #[test]
    fn render_emits_function_items() {
        let s = OpenAiConsumer::new().render_catalog(&[sample()]).expect("ok");
        assert!(s.contains("\"type\":\"function\""));
        assert!(s.contains("\"name\":\"fs:read\""));
    }

    #[test]
    fn parse_extracts_tool_call() {
        let out = r#"{
            "choices":[{"message":{"tool_calls":[
                {"id":"c1","function":{"name":"fs:read","arguments":"{\"p\":\"x\"}"}}
            ]}}]
        }"#;
        let c = OpenAiConsumer::new().parse_call(out).expect("ok").expect("some");
        assert_eq!(c.tool_id, "fs:read");
        assert_eq!(c.arguments["p"], "x");
    }

    #[test]
    fn parse_none_without_tool_call() {
        let out = r#"{"choices":[{"message":{"content":"hi"}}]}"#;
        assert!(OpenAiConsumer::new().parse_call(out).expect("ok").is_none());
    }

    #[test]
    fn parse_err_on_bad_json() {
        assert!(OpenAiConsumer::new().parse_call("nope").is_err());
    }

    #[test]
    fn dialect_name() {
        assert_eq!(OpenAiConsumer::new().dialect(), "openai_tools");
    }
}
