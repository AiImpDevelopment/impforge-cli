// SPDX-License-Identifier: MIT
//! `impforge-cli doctor` — real probes across Ollama / filesystem / MCP.

use crate::theme;
use impforge_emergence::Orchestrator;
use impforge_models::ollama;
use std::path::PathBuf;
use std::sync::Arc;

pub fn run(orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    theme::print_info("doctor — probing every layer");
    let mut any_fail = false;

    any_fail |= !probe_ollama();
    any_fail |= !probe_templates();
    any_fail |= !probe_skills();
    any_fail |= !probe_mcp_manifests();
    any_fail |= !probe_brain_modelfile();
    any_fail |= !probe_config_dir();

    let now = chrono::Utc::now().timestamp();
    let reports = orc.tick_health(now)?;
    let unhealthy = reports.iter().filter(|r| !r.state.is_healthy()).count();
    if unhealthy > 0 {
        theme::print_warning(&format!("{unhealthy} module(s) reporting non-healthy"));
        let heal = orc.tick_self_heal(now)?;
        for entry in heal {
            println!("  {} → {}", entry.module_id, entry.summary);
        }
    } else {
        theme::print_success(&format!("{} modules healthy", reports.len()));
    }

    if any_fail {
        theme::print_warning("doctor found issues — see warnings above");
    } else {
        theme::print_success("doctor: everything looks healthy");
    }
    Ok(())
}

fn probe_ollama() -> bool {
    if ollama::is_reachable(None) {
        match ollama::list_local_models(None) {
            Ok(models) => {
                theme::print_success(&format!("Ollama reachable — {} local model(s)", models.len()));
                true
            }
            Err(e) => {
                theme::print_warning(&format!("Ollama reachable but /api/tags failed: {e}"));
                false
            }
        }
    } else {
        theme::print_warning("Ollama NOT reachable at 127.0.0.1:11434 — install from https://ollama.com");
        false
    }
}

fn probe_templates() -> bool {
    let root = resolve_repo_root().join("templates");
    if !root.exists() {
        theme::print_warning(&format!("templates dir missing: {}", root.display()));
        return false;
    }
    let count = std::fs::read_dir(&root)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .count()
        })
        .unwrap_or(0);
    theme::print_success(&format!("templates: {count} bundled"));
    count > 0
}

fn probe_skills() -> bool {
    let root = resolve_repo_root().join("skills");
    if !root.exists() {
        theme::print_warning(&format!("skills dir missing: {}", root.display()));
        return false;
    }
    let count = std::fs::read_dir(&root)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .count()
        })
        .unwrap_or(0);
    theme::print_success(&format!("skills: {count} bundled"));
    true
}

fn probe_mcp_manifests() -> bool {
    let root = resolve_repo_root().join("mcp-manifests").join("servers");
    if !root.exists() {
        theme::print_warning(&format!("mcp-manifests dir missing: {}", root.display()));
        return false;
    }
    let files: Vec<PathBuf> = std::fs::read_dir(&root)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    let mut bad = 0;
    for p in &files {
        let raw = match std::fs::read_to_string(p) {
            Ok(r) => r,
            Err(_) => {
                bad += 1;
                continue;
            }
        };
        if serde_json::from_str::<serde_json::Value>(&raw).is_err() {
            bad += 1;
        }
    }
    if bad == 0 {
        theme::print_success(&format!("mcp-manifests: {} valid", files.len()));
        true
    } else {
        theme::print_warning(&format!("mcp-manifests: {bad}/{} broken", files.len()));
        false
    }
}

fn probe_brain_modelfile() -> bool {
    let path = resolve_repo_root().join("brain").join("Modelfile");
    if path.exists() {
        theme::print_success("BRAIN Modelfile present");
        true
    } else {
        theme::print_warning(&format!("BRAIN Modelfile missing: {}", path.display()));
        false
    }
}

fn probe_config_dir() -> bool {
    match impforge_core::paths::config_dir() {
        Ok(dir) => {
            if !dir.exists() {
                let _ = std::fs::create_dir_all(&dir);
            }
            theme::print_success(&format!("config dir: {}", dir.display()));
            true
        }
        Err(e) => {
            theme::print_warning(&format!("config dir resolution failed: {e}"));
            false
        }
    }
}

fn resolve_repo_root() -> PathBuf {
    if let Ok(dir) = std::env::var("IMPFORGE_CLI_ROOT") {
        return PathBuf::from(dir);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}
