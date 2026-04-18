// SPDX-License-Identifier: MIT
//! `impforge-cli export-config` — signed bundle for aiimp migration.

use crate::theme;
use impforge_emergence::Orchestrator;
use impforge_export::{write_bundle, ExportBundle};
use std::sync::Arc;

pub fn run(output: &str, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    let expanded = shellexpand(output);
    let path = std::path::PathBuf::from(&expanded);
    let mut bundle = ExportBundle {
        schema_version: 1,
        created_at_unix: chrono::Utc::now().timestamp(),
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        default_model: None,
        registered_mcp_clients: Vec::new(),
        autopilot_enabled: false,
        content_hash_sha256: String::new(),
        signature_hex: String::new(),
    };
    bundle.content_hash_sha256 = bundle.compute_content_hash();
    // Signature will be added when Ed25519 keypair management lands.
    write_bundle(&bundle, &path)?;
    theme::print_success(&format!("export bundle written → {}", path.display()));
    theme::print_info("next: run `impforge-aiimp import-cli-config ~/.impforge-cli/export.json` on the Pro app");
    Ok(())
}

fn shellexpand(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).display().to_string();
        }
    }
    p.to_string()
}
