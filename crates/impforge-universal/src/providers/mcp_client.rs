// SPDX-License-Identifier: MIT
//! `McpClientProvider` — ingests tools from an MCP server manifest.
//!
//! The v0 implementation synthesises `UniversalTool` descriptors directly
//! from the manifest's `tools: [...]` names (with a minimal
//! `{type: "object"}` schema).  This is enough for ReAct + GBNF routing
//! because those consumers ask the live MCP server for the FULL schema
//! only at invocation time.
//!
//! v0.2 will spawn the real MCP child, send `tools/list`, and cache the
//! full JSON-Schema into the registry — identical API, better fidelity.

use crate::errors::{UniversalError, UniversalResult};
use crate::providers::ToolProvider;
use crate::tool::{ToolCost, UniversalTool};
use impforge_mcp_server::catalog_validator::McpServerManifest;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct McpClientProvider {
    manifest: McpServerManifest,
}

impl McpClientProvider {
    pub fn new(manifest: McpServerManifest) -> Self {
        Self { manifest }
    }

    pub fn manifest(&self) -> &McpServerManifest {
        &self.manifest
    }
}

impl ToolProvider for McpClientProvider {
    fn source(&self) -> &str {
        &self.manifest.id
    }

    fn fetch_tools(&self) -> UniversalResult<Vec<UniversalTool>> {
        if self.manifest.tools.is_empty() {
            return Err(UniversalError::Provider(format!(
                "manifest '{}' has no tools",
                self.manifest.id
            )));
        }
        let source = self.manifest.id.clone();
        let cost = cost_for_source(&source);
        Ok(self
            .manifest
            .tools
            .iter()
            .map(|name| UniversalTool {
                id: format!("{source}:{name}"),
                name: name.clone(),
                description: format!("{} tool from {}", name, self.manifest.name),
                input_schema: json!({"type": "object"}),
                output_schema: None,
                source: source.clone(),
                cost,
            })
            .collect())
    }
}

fn cost_for_source(source: &str) -> ToolCost {
    // Cheap heuristic — local filesystem / sqlite / time are Zero; network
    // fetchers + vector-search are Medium; cloud APIs are High.  Adjusted
    // over time by the self-learning router.
    match source {
        "filesystem" | "memory" | "sqlite" | "time" | "git" | "sequential-thinking" => {
            ToolCost::Zero
        }
        "brave-search" | "fetch" | "tavily" | "exa" | "perplexity" => ToolCost::Medium,
        s if s.starts_with("openai") || s.starts_with("anthropic") => ToolCost::High,
        _ => ToolCost::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use impforge_mcp_server::catalog_validator::{
        McpServerManifest, TransportKind, VerificationStatus,
    };

    fn fs_manifest() -> McpServerManifest {
        McpServerManifest {
            id: "filesystem".to_string(),
            name: "Filesystem MCP".to_string(),
            description: "fs".to_string(),
            transport: TransportKind::Stdio,
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
            ]),
            url: None,
            tools: vec!["read_file".to_string(), "write_file".to_string()],
            license: "MIT".to_string(),
            category: "filesystem".to_string(),
            maintainer: "mcp".to_string(),
            upstream: "https://github.com/modelcontextprotocol/servers".to_string(),
            verification_status: VerificationStatus::Verified,
        }
    }

    #[test]
    fn fetch_tools_yields_one_per_manifest_entry() {
        let p = McpClientProvider::new(fs_manifest());
        let tools = p.fetch_tools().expect("ok");
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].id, "filesystem:read_file");
        assert_eq!(tools[0].source, "filesystem");
    }

    #[test]
    fn empty_tools_list_errors() {
        let mut m = fs_manifest();
        m.tools.clear();
        let p = McpClientProvider::new(m);
        assert!(p.fetch_tools().is_err());
    }

    #[test]
    fn cost_heuristic_matches_source() {
        assert_eq!(cost_for_source("filesystem"), ToolCost::Zero);
        assert_eq!(cost_for_source("tavily"), ToolCost::Medium);
        assert_eq!(cost_for_source("anthropic"), ToolCost::High);
        assert_eq!(cost_for_source("unknown-xyz"), ToolCost::Low);
    }

    #[test]
    fn source_prefix_matches_manifest_id() {
        let p = McpClientProvider::new(fs_manifest());
        assert_eq!(p.source(), "filesystem");
        for t in p.fetch_tools().expect("ok") {
            assert_eq!(t.source, p.source());
        }
    }
}
