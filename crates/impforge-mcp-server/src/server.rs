// SPDX-License-Identifier: MIT
//! Real MCP stdio JSON-RPC server.
//!
//! Handles the MCP lifecycle messages:
//!   * `initialize`      — negotiate protocol version + capabilities
//!   * `tools/list`      — advertise tool names (lazy; names only)
//!   * `tools/call`      — execute a named tool and return its result
//!   * `notifications/initialized` — accepted without response
//!   * shutdown          — clean exit
//!
//! The server reads line-delimited JSON from stdin, writes line-delimited
//! JSON to stdout, and sends log lines to stderr so the transport stays
//! clean.

use crate::lazy_schema::{SchemaCache, TOOL_DESCRIPTORS};
use crate::transport::{JsonRpcRequest, JsonRpcResponse};
use serde::Serialize;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

const PROTOCOL_VERSION: &str = "2024-11-05";

pub struct ServerContext {
    pub templates_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub manifests_dir: PathBuf,
    pub cache: Arc<SchemaCache>,
}

impl ServerContext {
    pub fn new(repo_root: &std::path::Path) -> Self {
        Self {
            templates_dir: repo_root.join("templates"),
            skills_dir: repo_root.join("skills"),
            manifests_dir: repo_root.join("mcp-manifests").join("servers"),
            cache: Arc::new(SchemaCache::new()),
        }
    }
}

pub fn run_stdio(ctx: ServerContext) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    eprintln!("[impforge-mcp-server] listening on stdio");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[impforge-mcp-server] stdin error: {e}");
                break;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[impforge-mcp-server] bad JSON: {e}");
                continue;
            }
        };
        let is_notification = req.id.is_null();
        let response = dispatch(&req, &ctx);
        if is_notification {
            continue;
        }
        let payload = serde_json::to_string(&response)?;
        writeln!(out, "{payload}")?;
        out.flush()?;
    }
    eprintln!("[impforge-mcp-server] stdin closed, shutting down");
    Ok(())
}

pub fn dispatch(req: &JsonRpcRequest, ctx: &ServerContext) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => handle_initialize(req),
        "initialized" | "notifications/initialized" => {
            JsonRpcResponse::ok(req.id.clone(), serde_json::Value::Null)
        }
        "tools/list" => handle_tools_list(req, ctx),
        "tools/call" => handle_tools_call(req, ctx),
        "ping" => JsonRpcResponse::ok(req.id.clone(), serde_json::json!({})),
        "shutdown" => JsonRpcResponse::ok(req.id.clone(), serde_json::Value::Null),
        _ => JsonRpcResponse::err(
            req.id.clone(),
            -32601,
            format!("method not found: {}", req.method),
        ),
    }
}

fn handle_initialize(req: &JsonRpcRequest) -> JsonRpcResponse {
    let payload = serde_json::json!({
        "protocolVersion": PROTOCOL_VERSION,
        "serverInfo": {
            "name": "impforge-cli",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "capabilities": {
            "tools": { "listChanged": false },
        }
    });
    JsonRpcResponse::ok(req.id.clone(), payload)
}

fn handle_tools_list(req: &JsonRpcRequest, _ctx: &ServerContext) -> JsonRpcResponse {
    let tools: Vec<serde_json::Value> = TOOL_DESCRIPTORS
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.summary,
                "inputSchema": { "type": "object", "properties": {} }
            })
        })
        .collect();
    JsonRpcResponse::ok(req.id.clone(), serde_json::json!({ "tools": tools }))
}

fn handle_tools_call(req: &JsonRpcRequest, ctx: &ServerContext) -> JsonRpcResponse {
    let name = req
        .params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let args = req.params.get("arguments").cloned().unwrap_or_default();
    let result = match name {
        "impforge_list_templates" => list_templates(ctx),
        "impforge_get_template" => get_template(ctx, &args),
        "impforge_get_compliance" => get_compliance(ctx, &args),
        "impforge_list_skills" => list_skills(ctx),
        "impforge_list_mcp_manifests" => list_manifests(ctx),
        "impforge_health" => health(ctx),
        _ => {
            return JsonRpcResponse::err(
                req.id.clone(),
                -32602,
                format!("unknown tool: {name}"),
            );
        }
    };
    match result {
        Ok(v) => JsonRpcResponse::ok(req.id.clone(), wrap_tool_result(&v)),
        Err(e) => JsonRpcResponse::err(req.id.clone(), -32000, e.to_string()),
    }
}

fn wrap_tool_result<T: Serialize>(value: &T) -> serde_json::Value {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    serde_json::json!({
        "content": [{ "type": "text", "text": text }],
        "isError": false
    })
}

fn list_templates(ctx: &ServerContext) -> anyhow::Result<serde_json::Value> {
    if !ctx.templates_dir.exists() {
        return Ok(serde_json::json!({ "templates": [] }));
    }
    let mut ids: Vec<String> = std::fs::read_dir(&ctx.templates_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    ids.sort();
    Ok(serde_json::json!({ "templates": ids, "count": ids.len() }))
}

fn get_template(
    ctx: &ServerContext,
    args: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing argument: id"))?;
    let manifest_path = ctx.templates_dir.join(id).join("template.json");
    if !manifest_path.exists() {
        anyhow::bail!("template '{id}' not found");
    }
    let raw = std::fs::read_to_string(&manifest_path)?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(parsed)
}

fn get_compliance(
    ctx: &ServerContext,
    args: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing argument: id"))?;
    let path = ctx.templates_dir.join(id).join("compliance_rules.json");
    if !path.exists() {
        return Ok(serde_json::json!({ "rules": [], "count": 0 }));
    }
    let raw = std::fs::read_to_string(&path)?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)?;
    let count = parsed.as_array().map(|a| a.len()).unwrap_or(0);
    Ok(serde_json::json!({ "rules": parsed, "count": count }))
}

fn list_skills(ctx: &ServerContext) -> anyhow::Result<serde_json::Value> {
    if !ctx.skills_dir.exists() {
        return Ok(serde_json::json!({ "skills": [] }));
    }
    let mut ids: Vec<String> = std::fs::read_dir(&ctx.skills_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    ids.sort();
    Ok(serde_json::json!({ "skills": ids, "count": ids.len() }))
}

fn list_manifests(ctx: &ServerContext) -> anyhow::Result<serde_json::Value> {
    if !ctx.manifests_dir.exists() {
        return Ok(serde_json::json!({ "manifests": [] }));
    }
    let mut ids: Vec<String> = std::fs::read_dir(&ctx.manifests_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect();
    ids.sort();
    Ok(serde_json::json!({ "manifests": ids, "count": ids.len() }))
}

fn health(_ctx: &ServerContext) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::json!({
        "impforge-cli": env!("CARGO_PKG_VERSION"),
        "status": "healthy",
        "tools_available": TOOL_DESCRIPTORS.len()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_context(td: &TempDir) -> ServerContext {
        let root = td.path();
        std::fs::create_dir_all(root.join("templates/demo")).expect("mkdir");
        std::fs::create_dir_all(root.join("skills/demo")).expect("mkdir");
        std::fs::create_dir_all(root.join("mcp-manifests/servers")).expect("mkdir");
        std::fs::write(
            root.join("templates/demo/template.json"),
            r#"{"id":"demo","name":"Demo","category":"web"}"#,
        )
        .expect("w");
        std::fs::write(
            root.join("mcp-manifests/servers/filesystem.json"),
            r#"{"id":"filesystem"}"#,
        )
        .expect("w");
        ServerContext::new(root)
    }

    fn req(method: &str, id_int: u64, params: serde_json::Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(id_int),
            method: method.to_string(),
            params,
        }
    }

    #[test]
    fn initialize_returns_protocol_version() {
        let td = TempDir::new().expect("td");
        let ctx = sample_context(&td);
        let r = dispatch(&req("initialize", 1, serde_json::Value::Null), &ctx);
        let result = r.result.expect("result");
        assert_eq!(result["protocolVersion"].as_str().expect("str"), PROTOCOL_VERSION);
    }

    #[test]
    fn tools_list_returns_descriptor_count() {
        let td = TempDir::new().expect("td");
        let ctx = sample_context(&td);
        let r = dispatch(&req("tools/list", 2, serde_json::Value::Null), &ctx);
        let result = r.result.expect("result");
        let tools = result["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), TOOL_DESCRIPTORS.len());
    }

    #[test]
    fn tools_call_list_templates_finds_demo() {
        let td = TempDir::new().expect("td");
        let ctx = sample_context(&td);
        let params = serde_json::json!({ "name": "impforge_list_templates", "arguments": {} });
        let r = dispatch(&req("tools/call", 3, params), &ctx);
        let result = r.result.expect("result");
        let text = result["content"][0]["text"].as_str().expect("str");
        assert!(text.contains("demo"));
    }

    #[test]
    fn tools_call_get_template_returns_manifest() {
        let td = TempDir::new().expect("td");
        let ctx = sample_context(&td);
        let params = serde_json::json!({
            "name": "impforge_get_template",
            "arguments": { "id": "demo" }
        });
        let r = dispatch(&req("tools/call", 4, params), &ctx);
        let result = r.result.expect("result");
        let text = result["content"][0]["text"].as_str().expect("str");
        assert!(text.contains("\"id\""));
    }

    #[test]
    fn tools_call_unknown_returns_error() {
        let td = TempDir::new().expect("td");
        let ctx = sample_context(&td);
        let params = serde_json::json!({ "name": "ghost", "arguments": {} });
        let r = dispatch(&req("tools/call", 5, params), &ctx);
        assert!(r.error.is_some());
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let td = TempDir::new().expect("td");
        let ctx = sample_context(&td);
        let r = dispatch(&req("tools/ghost", 6, serde_json::Value::Null), &ctx);
        let err = r.error.expect("error");
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn list_manifests_finds_filesystem() {
        let td = TempDir::new().expect("td");
        let ctx = sample_context(&td);
        let params = serde_json::json!({ "name": "impforge_list_mcp_manifests", "arguments": {} });
        let r = dispatch(&req("tools/call", 7, params), &ctx);
        let text = r.result.expect("result")["content"][0]["text"]
            .as_str()
            .expect("text")
            .to_string();
        assert!(text.contains("filesystem"));
    }

    #[test]
    fn ping_returns_ok() {
        let td = TempDir::new().expect("td");
        let ctx = sample_context(&td);
        let r = dispatch(&req("ping", 8, serde_json::Value::Null), &ctx);
        assert!(r.result.is_some());
    }
}
