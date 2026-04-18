// SPDX-License-Identifier: MIT
//! `impforge-cli template` subcommand.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use impforge_scaffold::scaffold_template;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum TemplateCmd {
    /// List every bundled template.
    List,
    /// Show the manifest of a specific template.
    Show { id: String },
    /// Copy a template into a target directory.
    Scaffold { id: String, target: PathBuf },
    /// List the compliance rules attached to a template.
    Compliance { id: String },
}

pub fn run(cmd: TemplateCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    let templates_root = resolve_templates_root()?;
    match cmd {
        TemplateCmd::List => list(&templates_root)?,
        TemplateCmd::Show { id } => show(&templates_root, &id)?,
        TemplateCmd::Scaffold { id, target } => {
            scaffold(&templates_root, &id, &target)?;
        }
        TemplateCmd::Compliance { id } => compliance(&templates_root, &id)?,
    }
    Ok(())
}

fn resolve_templates_root() -> anyhow::Result<PathBuf> {
    if let Ok(root) = std::env::var("IMPFORGE_TEMPLATES_DIR") {
        return Ok(PathBuf::from(root));
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("templates"))
}

fn list(root: &std::path::Path) -> anyhow::Result<()> {
    if !root.exists() {
        theme::print_warning(&format!(
            "templates directory not found at {} — set IMPFORGE_TEMPLATES_DIR",
            root.display()
        ));
        return Ok(());
    }
    let mut ids: Vec<String> = std::fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    ids.sort();
    theme::print_info(&format!("{} templates bundled", ids.len()));
    for id in ids {
        println!("  {}{}{}", theme::ACCENT_NEON, id, theme::RESET);
    }
    Ok(())
}

fn show(root: &std::path::Path, id: &str) -> anyhow::Result<()> {
    let manifest_path = root.join(id).join("template.json");
    if !manifest_path.exists() {
        theme::print_error(&format!("template '{id}' not found"));
        return Ok(());
    }
    let raw = std::fs::read_to_string(&manifest_path)?;
    println!("{raw}");
    Ok(())
}

fn scaffold(
    root: &std::path::Path,
    id: &str,
    target: &std::path::Path,
) -> anyhow::Result<()> {
    let report = scaffold_template(root, id, target)?;
    theme::print_success(&format!(
        "scaffolded {} files ({} bytes) to {}",
        report.file_count,
        report.total_bytes,
        report.target.display()
    ));
    theme::print_info(&format!(
        "compliance: {}",
        report.manifest.compliance.join(" · ")
    ));
    theme::print_info(&format!("content hash: {}", report.content_hash));
    println!();
    theme::print_info(&format!("next: cd {} && {}", report.target.display(), report.manifest.preview_command));
    Ok(())
}

fn compliance(root: &std::path::Path, id: &str) -> anyhow::Result<()> {
    let rules_path = root.join(id).join("compliance_rules.json");
    if !rules_path.exists() {
        theme::print_warning(&format!("no compliance_rules.json for '{id}'"));
        return Ok(());
    }
    let raw = std::fs::read_to_string(&rules_path)?;
    let rules: Vec<impforge_core::ComplianceRule> = serde_json::from_str(&raw)?;
    theme::print_info(&format!("{} compliance rules for {id}", rules.len()));
    for rule in rules.iter().take(10) {
        println!(
            "  {}{}{}  {}{:>8}{}  {}",
            theme::ACCENT_NEON,
            rule.id,
            theme::RESET,
            theme::ACCENT_CYAN,
            rule.regime,
            theme::RESET,
            rule.title
        );
    }
    if rules.len() > 10 {
        theme::print_info(&format!("… + {} more (use impforge-cli mcp serve for full query)", rules.len() - 10));
    }
    Ok(())
}
