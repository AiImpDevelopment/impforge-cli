// SPDX-License-Identifier: MIT
//! Live MCP client — spawns an MCP server child, performs the JSON-RPC
//! initialize + tools/list handshake, and converts the response into
//! `UniversalTool` descriptors with the REAL schemas (not the minimal
//! `{type:"object"}` stub the manifest provider uses).
//!
//! v0.2 of the CLI universal crate.  Backed by `std::process::Command` —
//! no extra crate dependency.  Reads one JSON-RPC frame per line from the
//! child's stdout.
//!
//! ## Protocol (MCP 2024-11-05)
//!
//! 1. `{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}`
//! 2. `{"jsonrpc":"2.0","id":2,"method":"tools/list"}`
//!
//! The server's `tools/list` result `{tools: [{name, description,
//! inputSchema}, ...]}` is lifted directly into `UniversalTool`.

use crate::errors::{UniversalError, UniversalResult};
use crate::providers::ToolProvider;
use crate::tool::{ToolCost, UniversalTool};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub const DEFAULT_SPAWN_TIMEOUT_MS: u64 = 5_000;
pub const DEFAULT_TOOLS_LIST_TIMEOUT_MS: u64 = 5_000;

/// Spawns the downstream MCP server as a child process and discovers its
/// tools via `tools/list`.  The process is killed when discovery is done.
pub struct McpLiveProvider {
    source_id: String,
    command: String,
    args: Vec<String>,
}

impl McpLiveProvider {
    pub fn new(source_id: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            source_id: source_id.into(),
            command: command.into(),
            args,
        }
    }
}

impl ToolProvider for McpLiveProvider {
    fn source(&self) -> &str {
        &self.source_id
    }

    fn fetch_tools(&self) -> UniversalResult<Vec<UniversalTool>> {
        spawn_and_list_tools(
            &self.source_id,
            &self.command,
            &self.args,
            DEFAULT_TOOLS_LIST_TIMEOUT_MS,
        )
    }
}

fn spawn_and_list_tools(
    source_id: &str,
    cmd: &str,
    args: &[String],
    timeout_ms: u64,
) -> UniversalResult<Vec<UniversalTool>> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| UniversalError::Provider(format!("spawn '{cmd}' failed: {e}")))?;

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"impforge-universal-live","version":"0.2"}}}"#;
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| UniversalError::Provider("child stdin unavailable".to_string()))?;
    if let Err(e) = writeln!(stdin, "{init}") {
        let _ = child.kill();
        return Err(UniversalError::Provider(format!("init write: {e}")));
    }
    if let Err(e) = writeln!(stdin, "{list}") {
        let _ = child.kill();
        return Err(UniversalError::Provider(format!("list write: {e}")));
    }
    drop(stdin);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| UniversalError::Provider("child stdout unavailable".to_string()))?;

    let (tx, rx) = mpsc::channel::<Vec<String>>();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut lines = Vec::new();
        for l in reader.lines().map_while(Result::ok) {
            if l.trim().is_empty() {
                continue;
            }
            lines.push(l);
            if lines.len() >= 2 {
                break;
            }
        }
        let _ = tx.send(lines);
    });

    let lines = rx
        .recv_timeout(Duration::from_millis(timeout_ms))
        .map_err(|_| {
            let _ = child.kill();
            UniversalError::Provider(format!(
                "tools/list timeout after {timeout_ms} ms ({source_id})"
            ))
        })?;
    let _ = child.kill();
    let _ = child.wait();

    for line in &lines {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("id").and_then(|x| x.as_i64()) != Some(2) {
            continue;
        }
        let tools_value = v
            .pointer("/result/tools")
            .and_then(|x| x.as_array())
            .ok_or_else(|| {
                UniversalError::Provider(format!(
                    "tools/list response has no result.tools array ({source_id})"
                ))
            })?;
        let mut out = Vec::with_capacity(tools_value.len());
        for t in tools_value {
            let name = t
                .get("name")
                .and_then(|x| x.as_str())
                .ok_or_else(|| {
                    UniversalError::Provider(format!(
                        "tools/list item missing name ({source_id})"
                    ))
                })?;
            let description = t
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let input_schema = t
                .get("inputSchema")
                .cloned()
                .unwrap_or(serde_json::json!({"type":"object"}));
            out.push(UniversalTool {
                id: format!("{source_id}:{name}"),
                name: name.to_string(),
                description,
                input_schema,
                output_schema: None,
                source: source_id.to_string(),
                cost: ToolCost::Low,
            });
        }
        return Ok(out);
    }
    Err(UniversalError::Provider(format!(
        "no tools/list response line seen ({source_id})"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_reports_id() {
        let p = McpLiveProvider::new("x", "true", vec![]);
        assert_eq!(p.source(), "x");
    }

    #[test]
    fn spawn_failure_returns_error() {
        let p = McpLiveProvider::new(
            "ghost",
            "definitely-not-a-real-binary-xyz-123",
            vec![],
        );
        let r = p.fetch_tools();
        assert!(r.is_err());
        let msg = r.unwrap_err().to_string();
        assert!(msg.contains("spawn") || msg.contains("timeout"));
    }

    #[test]
    fn fake_server_emits_tools_list() {
        // Build a shell one-liner that eats stdin and emits two JSON-RPC frames.
        let script = r#"
            cat > /dev/null &
            printf '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{}}}\n'
            printf '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"echo","inputSchema":{"type":"object"}}]}}\n'
            sleep 1
        "#;
        let p = McpLiveProvider::new(
            "fake",
            "sh",
            vec!["-c".to_string(), script.to_string()],
        );
        let tools = p.fetch_tools().expect("ok");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id, "fake:echo");
        assert_eq!(tools[0].name, "echo");
    }

    #[test]
    fn timeout_returns_error_on_silent_server() {
        let p = McpLiveProvider::new(
            "silent",
            "sh",
            vec!["-c".to_string(), "sleep 5".to_string()],
        );
        let out = spawn_and_list_tools(p.source(), "sh", &["-c".to_string(), "sleep 5".to_string()], 300);
        assert!(out.is_err());
    }
}
