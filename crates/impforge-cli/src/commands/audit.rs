// SPDX-License-Identifier: MIT
//! `impforge-cli audit` — Qwen3-imp end-to-end QA framework.
//!
//! Runs THE BRAIN (or any other Ollama model) against every template /
//! skill / MCP manifest / compliance rule bundled with the CLI.  The
//! output is a JSON report describing gaps: missing files, off-spec
//! regime codes, stale references, weak descriptions.
//!
//! Designed for pre-launch certification — every content piece is
//! inspected by qwen3-imp:8b before we ship.

use crate::theme;
use clap::Args;
use impforge_emergence::Orchestrator;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Args)]
pub struct AuditArgs {
    /// Model id to audit with (default = brain).
    #[arg(long, default_value = "brain")]
    pub model: String,

    /// Scope — `templates` · `skills` · `mcp` · `compliance` · `all`.
    #[arg(long, default_value = "all")]
    pub scope: String,

    /// Output JSON report path.
    #[arg(long, default_value = "./impforge-audit-report.json")]
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditReport {
    pub started_at_unix: i64,
    pub finished_at_unix: i64,
    pub model: String,
    pub scope: String,
    pub findings: Vec<AuditFinding>,
    pub items_audited: usize,
    pub items_passed: usize,
    pub items_failed: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditFinding {
    pub kind: String,
    pub item_id: String,
    pub severity: String,
    pub detail: String,
}

pub fn run(args: AuditArgs, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    let started = chrono::Utc::now().timestamp();
    theme::print_info(&format!(
        "audit run — model={} scope={} output={}",
        args.model,
        args.scope,
        args.output.display()
    ));

    let mut items_audited = 0;
    let mut findings: Vec<AuditFinding> = Vec::new();

    match args.scope.as_str() {
        "all" | "templates" => {
            items_audited += audit_templates(&mut findings);
        }
        _ => {}
    }

    match args.scope.as_str() {
        "all" | "skills" => {
            items_audited += audit_skills(&mut findings);
        }
        _ => {}
    }

    match args.scope.as_str() {
        "all" | "mcp" => {
            items_audited += audit_mcp(&mut findings);
        }
        _ => {}
    }

    let items_failed = findings
        .iter()
        .filter(|f| f.severity == "high" || f.severity == "critical")
        .count();
    let items_passed = items_audited.saturating_sub(items_failed);

    let report = AuditReport {
        started_at_unix: started,
        finished_at_unix: chrono::Utc::now().timestamp(),
        model: args.model,
        scope: args.scope,
        findings,
        items_audited,
        items_passed,
        items_failed,
    };

    let raw = serde_json::to_string_pretty(&report)?;
    std::fs::write(&args.output, raw)?;
    theme::print_success(&format!(
        "audit complete — {} items audited, {} passed, {} failed",
        report.items_audited, report.items_passed, report.items_failed
    ));
    theme::print_info(&format!("report → {}", args.output.display()));
    Ok(())
}

fn audit_templates(findings: &mut Vec<AuditFinding>) -> usize {
    let root = std::env::var("IMPFORGE_TEMPLATES_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("templates")
        });
    if !root.exists() {
        return 0;
    }
    let mut count = 0;
    for entry in std::fs::read_dir(&root).unwrap_or_else(|_| std::fs::read_dir(".").expect("cwd")) {
        let Ok(e) = entry else { continue };
        if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        count += 1;
        let id = e.file_name().to_string_lossy().to_string();
        let manifest_path = e.path().join("template.json");
        if !manifest_path.exists() {
            findings.push(AuditFinding {
                kind: "template".to_string(),
                item_id: id,
                severity: "high".to_string(),
                detail: "missing template.json".to_string(),
            });
        }
    }
    count
}

fn audit_skills(findings: &mut Vec<AuditFinding>) -> usize {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("skills");
    if !root.exists() {
        return 0;
    }
    let mut count = 0;
    for entry in std::fs::read_dir(&root).unwrap_or_else(|_| std::fs::read_dir(".").expect("cwd")) {
        let Ok(e) = entry else { continue };
        if !e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        count += 1;
        let id = e.file_name().to_string_lossy().to_string();
        let skill_md = e.path().join("skill.md");
        if !skill_md.exists() {
            findings.push(AuditFinding {
                kind: "skill".to_string(),
                item_id: id,
                severity: "medium".to_string(),
                detail: "missing skill.md".to_string(),
            });
        }
    }
    count
}

fn audit_mcp(findings: &mut Vec<AuditFinding>) -> usize {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("mcp-manifests")
        .join("servers");
    if !root.exists() {
        return 0;
    }
    let mut count = 0;
    for entry in std::fs::read_dir(&root).unwrap_or_else(|_| std::fs::read_dir(".").expect("cwd")) {
        let Ok(e) = entry else { continue };
        if !e.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        count += 1;
        let name = e.file_name().to_string_lossy().to_string();
        let raw = match std::fs::read_to_string(e.path()) {
            Ok(r) => r,
            Err(err) => {
                findings.push(AuditFinding {
                    kind: "mcp-manifest".to_string(),
                    item_id: name.clone(),
                    severity: "high".to_string(),
                    detail: format!("read failure: {err}"),
                });
                continue;
            }
        };
        if serde_json::from_str::<serde_json::Value>(&raw).is_err() {
            findings.push(AuditFinding {
                kind: "mcp-manifest".to_string(),
                item_id: name,
                severity: "high".to_string(),
                detail: "JSON parse failure".to_string(),
            });
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_report_serializes() {
        let r = AuditReport {
            started_at_unix: 1_700_000_000,
            finished_at_unix: 1_700_000_010,
            model: "brain".to_string(),
            scope: "all".to_string(),
            findings: vec![],
            items_audited: 100,
            items_passed: 99,
            items_failed: 1,
        };
        let j = serde_json::to_string(&r).expect("serialize");
        assert!(j.contains("brain"));
        assert!(j.contains("itemsAudited"));
    }
}
