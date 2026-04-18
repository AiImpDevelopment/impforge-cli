// SPDX-License-Identifier: MIT
//! MCP pass-through consumer — re-emits catalog as MCP `tools/list`.

use crate::consumers::ToolConsumer;
use crate::errors::{UniversalError, UniversalResult};
use crate::tool::{ToolCall, UniversalTool};
use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct McpPassConsumer;

impl McpPassConsumer {
    pub fn new() -> Self {
        Self
    }
}

impl ToolConsumer for McpPassConsumer {
    fn dialect(&self) -> &'static str {
        "mcp"
    }

    fn render_catalog(&self, tools: &[UniversalTool]) -> UniversalResult<String> {
        let items: Vec<Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.id,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                })
            })
            .collect();
        let envelope = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"tools": items},
        });
        serde_json::to_string(&envelope).map_err(UniversalError::Serde)
    }

    fn parse_call(&self, model_output: &str) -> UniversalResult<Option<ToolCall>> {
        let v: Value = serde_json::from_str(model_output)?;
        if v.get("method").and_then(|x| x.as_str()) != Some("tools/call") {
            return Ok(None);
        }
        let params = v.get("params").ok_or_else(|| {
            UniversalError::Consumer("tools/call missing params".to_string())
        })?;
        let tool_id = params
            .get("name")
            .and_then(|x| x.as_str())
            .ok_or_else(|| UniversalError::Consumer("params.name missing".to_string()))?
            .to_string();
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let call_id = v
            .get("id")
            .map(|x| x.to_string())
            .unwrap_or_else(|| "mcp-call".to_string());
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
            description: "r".to_string(),
            input_schema: json!({"type":"object"}),
            output_schema: None,
            source: "fs".to_string(),
            cost: ToolCost::Low,
        }
    }

    #[test]
    fn render_wraps_in_jsonrpc_envelope() {
        let s = McpPassConsumer::new().render_catalog(&[sample()]).expect("ok");
        assert!(s.contains("\"jsonrpc\":\"2.0\""));
        assert!(s.contains("fs:read"));
    }

    #[test]
    fn parse_extracts_tools_call() {
        let out = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"fs:read","arguments":{"p":"x"}}}"#;
        let c = McpPassConsumer::new().parse_call(out).expect("ok").expect("some");
        assert_eq!(c.tool_id, "fs:read");
    }

    #[test]
    fn parse_none_for_tools_list() {
        let out = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        assert!(McpPassConsumer::new().parse_call(out).expect("ok").is_none());
    }

    #[test]
    fn dialect_name() {
        assert_eq!(McpPassConsumer::new().dialect(), "mcp");
    }
}
