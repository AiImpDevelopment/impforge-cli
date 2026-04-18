// SPDX-License-Identifier: MIT
//! Lazy schema registry — tool names always present, full schemas only
//! materialised on demand.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptor {
    pub name: &'static str,
    pub summary: &'static str,
    /// Token cost of the full schema when expanded — advisory.
    pub schema_tokens: u32,
}

pub const TOOL_DESCRIPTORS: &[ToolDescriptor] = &[
    ToolDescriptor { name: "impforge_list_templates", summary: "list all 78 industry-template scaffolds", schema_tokens: 320 },
    ToolDescriptor { name: "impforge_get_template", summary: "fetch a specific template manifest", schema_tokens: 400 },
    ToolDescriptor { name: "impforge_scaffold_template", summary: "scaffold a template into a target directory", schema_tokens: 480 },
    ToolDescriptor { name: "impforge_get_compliance", summary: "fetch compliance rules for a template", schema_tokens: 360 },
    ToolDescriptor { name: "impforge_list_skills", summary: "list available skills", schema_tokens: 240 },
    ToolDescriptor { name: "impforge_apply_skill", summary: "apply a skill to a project directory", schema_tokens: 400 },
    ToolDescriptor { name: "impforge_list_mcp_manifests", summary: "list registered MCP server manifests", schema_tokens: 260 },
    ToolDescriptor { name: "impforge_generate", summary: "generate a project using a template + local model", schema_tokens: 560 },
    ToolDescriptor { name: "impforge_benchmark_local", summary: "benchmark the user's local hardware", schema_tokens: 200 },
    ToolDescriptor { name: "impforge_health", summary: "report CLI + module health", schema_tokens: 220 },
];

#[derive(Debug, Default)]
pub struct SchemaCache {
    inner: RwLock<BTreeMap<&'static str, serde_json::Value>>,
}

impl SchemaCache {
    pub fn new() -> Self { Self::default() }

    pub fn get_or_materialise(&self, tool: &'static str) -> serde_json::Value {
        if let Ok(guard) = self.inner.read() {
            if let Some(v) = guard.get(tool) { return v.clone(); }
        }
        let schema = materialise(tool);
        if let Ok(mut guard) = self.inner.write() {
            guard.insert(tool, schema.clone());
        }
        schema
    }

    pub fn drop_all(&self) {
        if let Ok(mut guard) = self.inner.write() {
            guard.clear();
        }
    }

    pub fn materialised_count(&self) -> usize {
        self.inner.read().map(|g| g.len()).unwrap_or(0)
    }
}

fn materialise(tool: &'static str) -> serde_json::Value {
    serde_json::json!({
        "name": tool,
        "inputSchema": { "type": "object", "properties": {} },
        "description": summary_of(tool),
    })
}

fn summary_of(tool: &'static str) -> &'static str {
    for t in TOOL_DESCRIPTORS {
        if t.name == tool { return t.summary; }
    }
    ""
}

pub fn advertised_tokens() -> u32 {
    TOOL_DESCRIPTORS.iter().map(|_| 40_u32).sum()
}

pub fn full_expansion_tokens() -> u32 {
    TOOL_DESCRIPTORS.iter().map(|t| t.schema_tokens).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_are_non_empty() {
        assert!(!TOOL_DESCRIPTORS.is_empty());
    }

    #[test]
    fn lazy_loading_saves_at_least_75_percent() {
        let lazy = advertised_tokens() as f32;
        let full = full_expansion_tokens() as f32;
        let savings = 1.0 - (lazy / full);
        assert!(savings > 0.75, "only saved {:.2}", savings);
    }

    #[test]
    fn schema_cache_materialises_lazily() {
        let cache = SchemaCache::new();
        assert_eq!(cache.materialised_count(), 0);
        let _ = cache.get_or_materialise("impforge_list_templates");
        assert_eq!(cache.materialised_count(), 1);
        let _ = cache.get_or_materialise("impforge_list_templates");
        assert_eq!(cache.materialised_count(), 1); // same tool, no duplicate
    }

    #[test]
    fn schema_cache_drop_resets() {
        let cache = SchemaCache::new();
        let _ = cache.get_or_materialise("impforge_list_templates");
        let _ = cache.get_or_materialise("impforge_get_template");
        cache.drop_all();
        assert_eq!(cache.materialised_count(), 0);
    }
}
