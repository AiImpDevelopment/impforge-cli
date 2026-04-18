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
    match impforge_mcp_server::catalog_validator::validate_catalog(&root) {
        Ok(report) if report.is_clean() => {
            theme::print_success(&format!(
                "mcp-manifests: Crown-Jewel CLEAN — {}/{} pass (schema + license + cmd + url + tools)",
                report.clean, report.total_manifests
            ));
            true
        }
        Ok(report) => {
            theme::print_warning(&format!(
                "mcp-manifests: Crown-Jewel DIRTY — {}/{} fail",
                report.dirty, report.total_manifests
            ));
            for r in report.per_manifest.iter().filter(|r| !r.is_clean()) {
                println!(
                    "  BAD: {} ({:?})",
                    r.file.display(),
                    r.manifest_id
                );
                for issue in &r.issues {
                    println!("    - {issue}");
                }
            }
            if !report.duplicate_ids.is_empty() {
                println!("  duplicate ids: {:?}", report.duplicate_ids);
            }
            false
        }
        Err(e) => {
            theme::print_warning(&format!("mcp-manifests: validator failed: {e}"));
            false
        }
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
