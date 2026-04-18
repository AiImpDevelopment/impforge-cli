// SPDX-License-Identifier: MIT
//! Episodic memory store — the equivalent of ImpForge's `module_memory`,
//! scaled down for the CLI.

use impforge_core::{paths, CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Mutex, MutexGuard};

const MAX_ENTRIES: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    pub module_id: String,
    pub kind: MemoryEntryKind,
    pub summary: String,
    pub details: Option<String>,
    pub occurred_at_unix: i64,
    pub quality: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEntryKind {
    CapabilityInvocation,
    HealthCheck,
    SelfHeal,
    UserCommand,
    UpdateCheck,
    McpReconnect,
    Error,
}

/// In-memory ring buffer persisted to `~/.impforge-cli/memory.json`.
#[derive(Debug)]
pub struct MemoryStore {
    inner: Mutex<Vec<MemoryEntry>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self { inner: Mutex::new(Vec::with_capacity(MAX_ENTRIES)) }
    }

    pub fn load_from_disk() -> CoreResult<Self> {
        let path = paths::config_dir()?.join("memory.json");
        if !path.exists() {
            return Ok(Self::new());
        }
        let raw = fs::read_to_string(&path)?;
        let entries: Vec<MemoryEntry> = serde_json::from_str(&raw)?;
        Ok(Self { inner: Mutex::new(entries) })
    }

    pub fn persist_to_disk(&self) -> CoreResult<()> {
        let path = paths::config_dir()?.join("memory.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let guard = self.lock()?;
        let raw = serde_json::to_string_pretty(&*guard)?;
        fs::write(&path, raw)?;
        Ok(())
    }

    pub fn record(&self, entry: MemoryEntry) -> CoreResult<()> {
        let mut guard = self.lock()?;
        if guard.len() >= MAX_ENTRIES {
            guard.remove(0);
        }
        guard.push(entry);
        Ok(())
    }

    pub fn recent_for_module(&self, module_id: &str, limit: usize) -> CoreResult<Vec<MemoryEntry>> {
        let guard = self.lock()?;
        Ok(guard
            .iter()
            .rev()
            .filter(|e| e.module_id == module_id)
            .take(limit)
            .cloned()
            .collect())
    }

    pub fn recent(&self, limit: usize) -> CoreResult<Vec<MemoryEntry>> {
        let guard = self.lock()?;
        Ok(guard.iter().rev().take(limit).cloned().collect())
    }

    pub fn average_quality_for_module(&self, module_id: &str) -> CoreResult<Option<f32>> {
        let guard = self.lock()?;
        let samples: Vec<f32> = guard
            .iter()
            .filter(|e| e.module_id == module_id)
            .map(|e| e.quality)
            .collect();
        if samples.is_empty() {
            return Ok(None);
        }
        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        Ok(Some(mean))
    }

    pub fn len(&self) -> usize {
        self.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> CoreResult<MutexGuard<'_, Vec<MemoryEntry>>> {
        self.inner
            .lock()
            .map_err(|e| CoreError::other(format!("memory store lock: {e}")))
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(module: &str, kind: MemoryEntryKind, quality: f32) -> MemoryEntry {
        MemoryEntry {
            module_id: module.to_string(),
            kind,
            summary: "test".to_string(),
            details: None,
            occurred_at_unix: 1_700_000_000,
            quality,
        }
    }

    #[test]
    fn record_and_recent_round_trip() {
        let store = MemoryStore::new();
        store
            .record(entry("scaffold", MemoryEntryKind::UserCommand, 0.9))
            .expect("record");
        assert_eq!(store.len(), 1);
        let recent = store.recent(10).expect("recent");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].module_id, "scaffold");
    }

    #[test]
    fn ring_buffer_caps_at_max() {
        let store = MemoryStore::new();
        for i in 0..(MAX_ENTRIES + 200) {
            store
                .record(entry(&format!("m-{i}"), MemoryEntryKind::HealthCheck, 1.0))
                .expect("rec");
        }
        assert_eq!(store.len(), MAX_ENTRIES);
    }

    #[test]
    fn average_quality_computed() {
        let store = MemoryStore::new();
        store.record(entry("x", MemoryEntryKind::UserCommand, 0.8)).expect("1");
        store.record(entry("x", MemoryEntryKind::UserCommand, 1.0)).expect("2");
        let avg = store.average_quality_for_module("x").expect("ok").expect("some");
        assert!((avg - 0.9).abs() < 0.001);
    }

    #[test]
    fn average_quality_unknown_is_none() {
        let store = MemoryStore::new();
        assert!(store.average_quality_for_module("ghost").expect("ok").is_none());
    }
}
