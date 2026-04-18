// SPDX-License-Identifier: MIT
//! Runtime orchestrator — registers modules, dispatches capability
//! requests, and drives health + self-heal ticks.

use crate::capability::{CapabilityRequest, CapabilityResponse};
use crate::health::HealthReport;
use crate::memory::{MemoryEntry, MemoryEntryKind, MemoryStore};
use crate::module::{Module, PowerMode};
use impforge_core::{CoreError, CoreResult};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

pub struct Orchestrator {
    modules: RwLock<BTreeMap<&'static str, Arc<dyn Module>>>,
    memory: Arc<MemoryStore>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(BTreeMap::new()),
            memory: Arc::new(MemoryStore::new()),
        }
    }

    pub fn with_memory(memory: Arc<MemoryStore>) -> Self {
        Self { modules: RwLock::new(BTreeMap::new()), memory }
    }

    pub fn memory(&self) -> &Arc<MemoryStore> {
        &self.memory
    }

    pub fn register(&self, module: Arc<dyn Module>) -> CoreResult<()> {
        let id = module.id();
        let mut guard = self.modules.write().map_err(|e| {
            CoreError::other(format!("orchestrator register lock: {e}"))
        })?;
        if guard.contains_key(id) {
            return Err(CoreError::validation(format!(
                "module '{id}' already registered"
            )));
        }
        guard.insert(id, module);
        Ok(())
    }

    pub fn module(&self, id: &str) -> CoreResult<Option<Arc<dyn Module>>> {
        let guard = self.modules.read().map_err(|e| {
            CoreError::other(format!("orchestrator read lock: {e}"))
        })?;
        Ok(guard.get(id).cloned())
    }

    pub fn module_ids(&self) -> CoreResult<Vec<String>> {
        let guard = self.modules.read().map_err(|e| {
            CoreError::other(format!("orchestrator read lock: {e}"))
        })?;
        Ok(guard.keys().map(|s| s.to_string()).collect())
    }

    pub fn tick_health(&self, now_unix: i64) -> CoreResult<Vec<HealthReport>> {
        let guard = self.modules.read().map_err(|e| {
            CoreError::other(format!("orchestrator read lock: {e}"))
        })?;
        let mut reports = Vec::with_capacity(guard.len());
        for module in guard.values() {
            let mut report = module.health();
            report.last_heartbeat_unix = now_unix;
            reports.push(report);
        }
        Ok(reports)
    }

    /// For every module that's currently `Unhealthy` or `Degraded`, invoke
    /// `self_heal` and record the outcome.
    pub fn tick_self_heal(&self, _now_unix: i64) -> CoreResult<Vec<MemoryEntry>> {
        let guard = self.modules.read().map_err(|e| {
            CoreError::other(format!("orchestrator read lock: {e}"))
        })?;
        let mut entries = Vec::new();
        for module in guard.values() {
            let report = module.health();
            if !report.state.is_healthy() {
                let entry = module.self_heal();
                let _ = self.memory.record(entry.clone());
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// Transition a module into a new power mode.
    pub fn set_power_mode(&self, id: &str, mode: PowerMode) -> CoreResult<MemoryEntry> {
        let module = self
            .module(id)?
            .ok_or_else(|| CoreError::validation(format!("module '{id}' not registered")))?;
        let entry = module.set_power_mode(mode);
        self.memory.record(entry.clone())?;
        Ok(entry)
    }

    /// Send every Active/Full module into DeepSleep — called on CLI exit.
    pub fn hibernate_all(&self) -> CoreResult<()> {
        let ids = self.module_ids()?;
        for id in ids {
            let _ = self.set_power_mode(&id, PowerMode::DeepSleep);
        }
        Ok(())
    }

    /// Find every module that advertises a given capability tag.
    pub fn capable_of(&self, tag: &str) -> CoreResult<Vec<Arc<dyn Module>>> {
        let guard = self.modules.read().map_err(|e| {
            CoreError::other(format!("orchestrator read lock: {e}"))
        })?;
        Ok(guard
            .values()
            .filter(|m| m.capabilities().iter().any(|c| c.tag == tag))
            .cloned()
            .collect())
    }

    /// Dispatch a capability request — the first capable module wins.
    /// Falls through with `Err(CoreError::Validation)` if nobody can handle it.
    pub fn dispatch(&self, request: CapabilityRequest) -> CoreResult<CapabilityResponse> {
        let candidates = self.capable_of(&request.target_capability)?;
        if candidates.is_empty() {
            return Err(CoreError::validation(format!(
                "no module advertises capability '{}'",
                request.target_capability
            )));
        }
        let handler = candidates[0].clone();
        let entry = MemoryEntry {
            module_id: handler.id().to_string(),
            kind: MemoryEntryKind::CapabilityInvocation,
            summary: format!("dispatch {}", request.target_capability),
            details: Some(request.correlation_id.clone()),
            occurred_at_unix: 0,
            quality: 1.0,
        };
        self.memory.record(entry)?;
        Ok(CapabilityResponse {
            handler_module: handler.id().to_string(),
            ok: true,
            payload_json: serde_json::Value::Null,
            correlation_id: request.correlation_id,
        })
    }

    pub fn introspect(&self) -> CoreResult<OrchestratorSnapshot> {
        let ids = self.module_ids()?;
        let guard = self.modules.read().map_err(|e| {
            CoreError::other(format!("orchestrator read lock: {e}"))
        })?;
        let mut modules = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(m) = guard.get(id.as_str()) {
                modules.push(ModuleSnapshot {
                    id: m.id().to_string(),
                    description: m.description().to_string(),
                    capabilities: m.capabilities().iter().map(|c| c.tag.clone()).collect(),
                    health: m.health(),
                    power_mode: m.power_mode(),
                    is_lazy_mcp: m.is_lazy_mcp(),
                });
            }
        }
        Ok(OrchestratorSnapshot {
            modules,
            memory_entries: self.memory.len(),
        })
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ModuleSnapshot {
    pub id: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub health: HealthReport,
    pub power_mode: PowerMode,
    pub is_lazy_mcp: bool,
}

#[derive(Debug, Clone)]
pub struct OrchestratorSnapshot {
    pub modules: Vec<ModuleSnapshot>,
    pub memory_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{Capability, CapabilityCost};
    use crate::health::HealthReport;

    struct DummyModule;

    impl Module for DummyModule {
        fn id(&self) -> &'static str { "dummy" }
        fn description(&self) -> &'static str { "dummy module" }
        fn capabilities(&self) -> Vec<Capability> {
            vec![Capability::new("echo", "echo payload", CapabilityCost::Zero)]
        }
        fn health(&self) -> HealthReport {
            HealthReport::healthy("ok", 1_700_000_000)
        }
        fn self_heal(&self) -> MemoryEntry {
            MemoryEntry {
                module_id: "dummy".to_string(),
                kind: MemoryEntryKind::SelfHeal,
                summary: "noop".to_string(),
                details: None,
                occurred_at_unix: 0,
                quality: 1.0,
            }
        }
    }

    struct BrokenModule;

    impl Module for BrokenModule {
        fn id(&self) -> &'static str { "broken" }
        fn description(&self) -> &'static str { "perpetually unhealthy" }
        fn capabilities(&self) -> Vec<Capability> { vec![] }
        fn health(&self) -> HealthReport {
            HealthReport::unhealthy("simulated breakage", 1_700_000_000)
        }
        fn self_heal(&self) -> MemoryEntry {
            MemoryEntry {
                module_id: "broken".to_string(),
                kind: MemoryEntryKind::SelfHeal,
                summary: "attempted repair".to_string(),
                details: Some("still broken".to_string()),
                occurred_at_unix: 0,
                quality: 0.2,
            }
        }
    }

    #[test]
    fn register_and_dispatch_round_trip() {
        let orc = Orchestrator::new();
        orc.register(Arc::new(DummyModule)).expect("register");
        let req = CapabilityRequest {
            target_capability: "echo".to_string(),
            payload_json: serde_json::json!({"x": 1}),
            correlation_id: "corr-1".to_string(),
        };
        let resp = orc.dispatch(req).expect("dispatch");
        assert_eq!(resp.handler_module, "dummy");
        assert!(resp.ok);
    }

    #[test]
    fn dispatch_fails_when_no_handler() {
        let orc = Orchestrator::new();
        let req = CapabilityRequest {
            target_capability: "ghost".to_string(),
            payload_json: serde_json::Value::Null,
            correlation_id: "c".to_string(),
        };
        assert!(orc.dispatch(req).is_err());
    }

    #[test]
    fn tick_self_heal_runs_on_unhealthy_only() {
        let orc = Orchestrator::new();
        orc.register(Arc::new(DummyModule)).expect("reg");
        orc.register(Arc::new(BrokenModule)).expect("reg");
        let entries = orc.tick_self_heal(1_700_000_000).expect("tick");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].module_id, "broken");
    }

    #[test]
    fn duplicate_registration_rejected() {
        let orc = Orchestrator::new();
        orc.register(Arc::new(DummyModule)).expect("first");
        assert!(orc.register(Arc::new(DummyModule)).is_err());
    }

    #[test]
    fn introspect_returns_module_list() {
        let orc = Orchestrator::new();
        orc.register(Arc::new(DummyModule)).expect("reg");
        let snap = orc.introspect().expect("introspect");
        assert_eq!(snap.modules.len(), 1);
        assert_eq!(snap.modules[0].id, "dummy");
    }
}
