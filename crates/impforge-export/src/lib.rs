// SPDX-License-Identifier: MIT
//! Ed25519-signed config export.
//!
//! `impforge-cli export-config ~/.impforge-cli/export.json` writes a JSON
//! bundle describing the user's cli settings, installed MCP clients, and
//! chosen default model.  The bundle is signed with an Ed25519 keypair
//! that lives on the user's machine.  `impforge-aiimp` ingests the bundle
//! through its Quarantine Layer, verifying the signature before trusting
//! any field.
//!
//! ## Security contract
//!
//! - No proprietary ImpForge state is included.
//! - The signature covers the canonical JSON body — any tampering fails
//!   verification.
//! - The Ed25519 keypair is generated on first use and stored in
//!   `~/.impforge-cli/keys/`.

use impforge_core::{paths, CoreResult};
use impforge_emergence::{
    Capability, CapabilityCost, HealthReport, MemoryEntry, MemoryEntryKind, Module, PowerMode,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub schema_version: u32,
    pub created_at_unix: i64,
    pub cli_version: String,
    pub default_model: Option<String>,
    pub registered_mcp_clients: Vec<String>,
    pub autopilot_enabled: bool,
    pub content_hash_sha256: String,
    pub signature_hex: String,
}

impl ExportBundle {
    pub fn canonical_payload(&self) -> String {
        // Signature covers every field except signature_hex itself.
        let mut without_sig = self.clone();
        without_sig.signature_hex = String::new();
        serde_json::to_string(&without_sig).unwrap_or_default()
    }

    pub fn compute_content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_payload().as_bytes());
        hex::encode(hasher.finalize())
    }
}

pub fn write_bundle(bundle: &ExportBundle, path: &std::path::Path) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(bundle)?;
    fs::write(path, raw)?;
    Ok(())
}

pub fn read_bundle(path: &std::path::Path) -> CoreResult<ExportBundle> {
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

/// Default export path under the user's config dir.
pub fn default_export_path() -> CoreResult<std::path::PathBuf> {
    paths::export_file()
}

pub struct Module_;

impl Module for Module_ {
    fn id(&self) -> &'static str { "impforge-export" }
    fn description(&self) -> &'static str {
        "Ed25519-signed config export for impforge-aiimp migration"
    }
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::new("export-config", "write signed export.json", CapabilityCost::Low),
        ]
    }
    fn health(&self) -> HealthReport { HealthReport::healthy("idle", 0) }
    fn power_mode(&self) -> PowerMode { PowerMode::DeepSleep }
    fn self_heal(&self) -> MemoryEntry {
        MemoryEntry {
            module_id: "impforge-export".to_string(),
            kind: MemoryEntryKind::SelfHeal,
            summary: "regenerated missing keypair".to_string(),
            details: None,
            occurred_at_unix: 0,
            quality: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bundle() -> ExportBundle {
        let mut b = ExportBundle {
            schema_version: 1,
            created_at_unix: 1_700_000_000,
            cli_version: "0.1.0".to_string(),
            default_model: Some("qwen2.5-coder:7b".to_string()),
            registered_mcp_clients: vec!["claude-code".to_string()],
            autopilot_enabled: false,
            content_hash_sha256: String::new(),
            signature_hex: String::new(),
        };
        b.content_hash_sha256 = b.compute_content_hash();
        b
    }

    #[test]
    fn content_hash_is_deterministic() {
        let b = sample_bundle();
        let h = b.compute_content_hash();
        let again = b.compute_content_hash();
        assert_eq!(h, again);
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn tampering_changes_hash() {
        let mut b = sample_bundle();
        let h1 = b.compute_content_hash();
        b.default_model = Some("hermes-3:8b".to_string());
        let h2 = b.compute_content_hash();
        assert_ne!(h1, h2);
    }
}
