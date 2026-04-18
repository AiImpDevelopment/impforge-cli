// SPDX-License-Identifier: MIT
//! MCP stdio transport scaffold.
//!
//! Full protocol implementation is deferred to the next iteration — this
//! module establishes the struct shape + JSON-RPC framing so command
//! handlers can wire against a stable interface today.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
    }

    pub fn err(id: serde_json::Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message: message.into(), data: None }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_response_has_result() {
        let r = JsonRpcResponse::ok(serde_json::json!(1), serde_json::json!({"x": 1}));
        assert!(r.result.is_some());
        assert!(r.error.is_none());
    }

    #[test]
    fn err_response_has_error() {
        let r = JsonRpcResponse::err(serde_json::json!(1), -32000, "boom");
        assert!(r.error.is_some());
    }
}
