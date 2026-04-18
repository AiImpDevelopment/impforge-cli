// SPDX-License-Identifier: MIT
//! Universal tool registry — the central SQLite-backed namespace that
//! holds every tool ingested from every `ToolProvider`.
//!
//! For impforge-cli v0 we use an in-memory `BTreeMap`.  The SQLite
//! persistence + hot-reload via `notify` is planned for v0.2 once the
//! MVP ReAct bridge ships.  Keeping the API the same lets the upgrade be
//! a drop-in replacement without touching callers.

use crate::errors::{UniversalError, UniversalResult};
use crate::tool::UniversalTool;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct UniversalToolRegistry {
    tools: RwLock<BTreeMap<String, UniversalTool>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryStats {
    pub total: usize,
    pub by_source: BTreeMap<String, usize>,
}

impl UniversalToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool.  Rejects duplicates by `id`.
    pub fn register(&self, tool: UniversalTool) -> UniversalResult<()> {
        let mut guard = self
            .tools
            .write()
            .map_err(|e| UniversalError::Other(format!("registry lock: {e}")))?;
        if guard.contains_key(&tool.id) {
            return Err(UniversalError::DuplicateTool(tool.id));
        }
        guard.insert(tool.id.clone(), tool);
        Ok(())
    }

    /// Upsert — replaces an existing tool with the same id.  Useful for
    /// hot-reload scenarios where a provider's metadata has changed.
    pub fn upsert(&self, tool: UniversalTool) -> UniversalResult<()> {
        let mut guard = self
            .tools
            .write()
            .map_err(|e| UniversalError::Other(format!("registry lock: {e}")))?;
        guard.insert(tool.id.clone(), tool);
        Ok(())
    }

    pub fn deregister(&self, id: &str) -> UniversalResult<Option<UniversalTool>> {
        let mut guard = self
            .tools
            .write()
            .map_err(|e| UniversalError::Other(format!("registry lock: {e}")))?;
        Ok(guard.remove(id))
    }

    pub fn get(&self, id: &str) -> UniversalResult<Option<UniversalTool>> {
        let guard = self
            .tools
            .read()
            .map_err(|e| UniversalError::Other(format!("registry lock: {e}")))?;
        Ok(guard.get(id).cloned())
    }

    pub fn by_source(&self, source: &str) -> UniversalResult<Vec<UniversalTool>> {
        let guard = self
            .tools
            .read()
            .map_err(|e| UniversalError::Other(format!("registry lock: {e}")))?;
        Ok(guard
            .values()
            .filter(|t| t.source == source)
            .cloned()
            .collect())
    }

    pub fn all(&self) -> UniversalResult<Vec<UniversalTool>> {
        let guard = self
            .tools
            .read()
            .map_err(|e| UniversalError::Other(format!("registry lock: {e}")))?;
        Ok(guard.values().cloned().collect())
    }

    pub fn len(&self) -> usize {
        self.tools.read().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn stats(&self) -> UniversalResult<RegistryStats> {
        let guard = self
            .tools
            .read()
            .map_err(|e| UniversalError::Other(format!("registry lock: {e}")))?;
        let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
        for t in guard.values() {
            *by_source.entry(t.source.clone()).or_insert(0) += 1;
        }
        Ok(RegistryStats {
            total: guard.len(),
            by_source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolCost;
    use serde_json::json;

    fn tool(id: &str, source: &str) -> UniversalTool {
        UniversalTool {
            id: id.to_string(),
            name: id.split(':').next_back().unwrap_or("x").to_string(),
            description: format!("{id} desc"),
            input_schema: json!({"type": "object"}),
            output_schema: None,
            source: source.to_string(),
            cost: ToolCost::Low,
        }
    }

    #[test]
    fn register_and_get() {
        let r = UniversalToolRegistry::new();
        r.register(tool("fs:read", "fs")).expect("reg");
        let got = r.get("fs:read").expect("ok").expect("present");
        assert_eq!(got.name, "read");
    }

    #[test]
    fn duplicate_register_rejected() {
        let r = UniversalToolRegistry::new();
        r.register(tool("fs:read", "fs")).expect("reg");
        assert!(r.register(tool("fs:read", "fs")).is_err());
    }

    #[test]
    fn upsert_replaces_existing() {
        let r = UniversalToolRegistry::new();
        r.register(tool("fs:read", "fs")).expect("reg");
        let mut t2 = tool("fs:read", "fs");
        t2.description = "updated".to_string();
        r.upsert(t2).expect("upsert");
        assert_eq!(r.get("fs:read").expect("ok").expect("p").description, "updated");
    }

    #[test]
    fn deregister_returns_removed() {
        let r = UniversalToolRegistry::new();
        r.register(tool("fs:read", "fs")).expect("reg");
        let removed = r.deregister("fs:read").expect("ok").expect("present");
        assert_eq!(removed.id, "fs:read");
        assert!(r.get("fs:read").expect("ok").is_none());
    }

    #[test]
    fn by_source_filters_correctly() {
        let r = UniversalToolRegistry::new();
        r.register(tool("fs:a", "fs")).expect("reg");
        r.register(tool("fs:b", "fs")).expect("reg");
        r.register(tool("gh:a", "github")).expect("reg");
        assert_eq!(r.by_source("fs").expect("ok").len(), 2);
        assert_eq!(r.by_source("github").expect("ok").len(), 1);
    }

    #[test]
    fn stats_group_by_source() {
        let r = UniversalToolRegistry::new();
        r.register(tool("fs:a", "fs")).expect("reg");
        r.register(tool("fs:b", "fs")).expect("reg");
        r.register(tool("gh:a", "github")).expect("reg");
        let s = r.stats().expect("stats");
        assert_eq!(s.total, 3);
        assert_eq!(s.by_source.get("fs"), Some(&2));
        assert_eq!(s.by_source.get("github"), Some(&1));
    }

    #[test]
    fn empty_registry_reports_empty() {
        let r = UniversalToolRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn all_returns_every_tool() {
        let r = UniversalToolRegistry::new();
        r.register(tool("fs:a", "fs")).expect("reg");
        r.register(tool("gh:a", "github")).expect("reg");
        assert_eq!(r.all().expect("all").len(), 2);
    }
}
