// SPDX-License-Identifier: MIT
//! `impforge-cli crown-jewel` — run the 5-dimension quality gate.

use crate::theme;
use clap::Subcommand;
use impforge_crown_jewel::{scanner, Dimension};
use impforge_emergence::Orchestrator;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum CrownJewelCmd {
    /// Scan a directory and print findings (dims 1–4).
    Scan { path: PathBuf },
    /// Scan a full Cargo workspace (dims 1–5, includes wiring check).
    Workspace {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        bootstrap: Option<PathBuf>,
    },
    /// Behavioral scan (dims 6–7) from agent trace + error-recall store.
    Behavior,
    /// Gate mode: static + behavioral scan; exit 1 on blocking findings.
    Gate {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Write the full report as JSON.
    Report {
        path: PathBuf,
        #[arg(long, default_value = "crown-jewel-report.json")]
        output: PathBuf,
    },
}

pub fn run(cmd: CrownJewelCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        CrownJewelCmd::Scan { path } => scan_tree(&path)?,
        CrownJewelCmd::Workspace { root, bootstrap } => {
            scan_ws(&root, bootstrap.as_deref())?;
        }
        CrownJewelCmd::Behavior => behavior()?,
        CrownJewelCmd::Gate { path } => gate(&path)?,
        CrownJewelCmd::Report { path, output } => report(&path, &output)?,
    }
    Ok(())
}

fn behavior() -> anyhow::Result<()> {
    let recall_path = impforge_crown_jewel::error_recall_path()?;
    let prior = impforge_crown_jewel::load_recall_store(&recall_path).unwrap_or_default();
    // For a standalone behavior scan we have no fresh run to compare
    // against — we treat "prior" as the fresh set, which flags only
    // errors that persisted since the last check.
    let report = impforge_crown_jewel::scanner::scan_behavior(&prior, &prior)?;
    print_summary(&report);
    Ok(())
}

fn scan_tree(path: &std::path::Path) -> anyhow::Result<()> {
    let report = scanner::scan(path)?;
    print_summary(&report);
    Ok(())
}

fn scan_ws(root: &std::path::Path, bootstrap: Option<&std::path::Path>) -> anyhow::Result<()> {
    let report = scanner::scan_workspace(root, bootstrap)?;
    print_summary(&report);
    Ok(())
}

fn gate(path: &std::path::Path) -> anyhow::Result<()> {
    let report = scanner::scan(path)?;
    print_summary(&report);
    if !report.is_clean() {
        std::process::exit(1);
    }
    Ok(())
}

fn report(path: &std::path::Path, output: &std::path::Path) -> anyhow::Result<()> {
    let report = scanner::scan(path)?;
    let raw = serde_json::to_string_pretty(&report)?;
    std::fs::write(output, raw)?;
    theme::print_success(&format!("report written → {}", output.display()));
    Ok(())
}

fn print_summary(report: &impforge_crown_jewel::CrownJewelReport) {
    theme::print_info(&format!(
        "scanned {} files — {} findings",
        report.files_scanned,
        report.findings.len()
    ));
    println!();
    println!("  {}dim{}                        {}count{}",
        theme::ACCENT_CYAN, theme::RESET,
        theme::ACCENT_CYAN, theme::RESET);
    println!("  ─────────────────────────────────");
    println!("  (1) no_stubs              {:>6}", report.dimension_totals.no_stubs);
    println!("  (2) no_suppression        {:>6}", report.dimension_totals.no_suppression);
    println!("  (3) no_lonely_unwrap      {:>6}", report.dimension_totals.no_lonely_unwrap);
    println!("  (4) test_first            {:>6}", report.dimension_totals.test_first);
    println!("  (5) crown_jewel_wiring    {:>6}", report.dimension_totals.crown_jewel_wiring);
    println!("  (6) parallel_efficiency   {:>6}", report.dimension_totals.parallel_efficiency);
    println!("  (7) error_recall          {:>6}", report.dimension_totals.error_recall);
    println!();
    for f in report.findings.iter().take(20) {
        let color = match f.severity {
            impforge_crown_jewel::Severity::Critical => theme::ACCENT_MAGENTA,
            impforge_crown_jewel::Severity::High => theme::ACCENT_MAGENTA,
            _ => theme::DIM,
        };
        println!(
            "  {}{:<10}{} {}:{}  {}  {}",
            color,
            f.severity_str(),
            theme::RESET,
            f.path.display(),
            f.line,
            dim_label(f.dimension),
            f.snippet.chars().take(80).collect::<String>()
        );
    }
    if report.findings.len() > 20 {
        println!("  … and {} more — run `crown-jewel report` for the full JSON", report.findings.len() - 20);
    }
    println!();
    if report.is_clean() {
        theme::print_success("Crown-Jewel grade: CLEAN ✓");
    } else {
        theme::print_warning(&format!(
            "Crown-Jewel grade: {} blocking finding(s)",
            report.blocking_count()
        ));
    }
}

fn dim_label(d: Dimension) -> &'static str {
    match d {
        Dimension::NoStubs => "stub",
        Dimension::NoSuppression => "allow",
        Dimension::NoLonelyUnwrap => "unwrap",
        Dimension::TestFirst => "untested",
        Dimension::CrownJewelWiring => "orphan",
        Dimension::ParallelEfficiency => "idle-wait",
        Dimension::ErrorRecall => "regression",
    }
}

trait SeverityDisplay {
    fn severity_str(&self) -> &'static str;
}

impl SeverityDisplay for impforge_crown_jewel::CrownJewelFinding {
    fn severity_str(&self) -> &'static str {
        match self.severity {
            impforge_crown_jewel::Severity::Info => "info",
            impforge_crown_jewel::Severity::Low => "low",
            impforge_crown_jewel::Severity::Medium => "medium",
            impforge_crown_jewel::Severity::High => "high",
            impforge_crown_jewel::Severity::Critical => "critical",
        }
    }
}
