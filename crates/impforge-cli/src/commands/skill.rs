// SPDX-License-Identifier: MIT
//! `impforge-cli skill` subcommand.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum SkillCmd {
    List,
    Show { id: String },
    Apply { id: String, #[arg(default_value = ".")] target: PathBuf },
}

pub fn run(cmd: SkillCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    let root = resolve_skills_root()?;
    match cmd {
        SkillCmd::List => list(&root)?,
        SkillCmd::Show { id } => show(&root, &id)?,
        SkillCmd::Apply { id, target } => apply(&root, &id, &target)?,
    }
    Ok(())
}

fn resolve_skills_root() -> anyhow::Result<PathBuf> {
    if let Ok(r) = std::env::var("IMPFORGE_SKILLS_DIR") {
        return Ok(PathBuf::from(r));
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("skills"))
}

fn list(root: &std::path::Path) -> anyhow::Result<()> {
    if !root.exists() {
        theme::print_warning(&format!("skills dir not found at {}", root.display()));
        return Ok(());
    }
    let mut ids: Vec<String> = std::fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    ids.sort();
    theme::print_info(&format!("{} skills bundled", ids.len()));
    for id in ids {
        println!("  {}{}{}", theme::ACCENT_NEON, id, theme::RESET);
    }
    Ok(())
}

fn show(root: &std::path::Path, id: &str) -> anyhow::Result<()> {
    let path = root.join(id).join("skill.md");
    if !path.exists() {
        theme::print_error(&format!("skill '{id}' not found"));
        return Ok(());
    }
    println!("{}", std::fs::read_to_string(&path)?);
    Ok(())
}

fn apply(root: &std::path::Path, id: &str, target: &std::path::Path) -> anyhow::Result<()> {
    let src = root.join(id);
    if !src.exists() {
        theme::print_error(&format!("skill '{id}' not found"));
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    let dest = target.join(format!("SKILL-{id}.md"));
    let content = std::fs::read_to_string(src.join("skill.md"))?;
    std::fs::write(&dest, content)?;
    theme::print_success(&format!("applied skill '{id}' → {}", dest.display()));
    Ok(())
}
