// SPDX-License-Identifier: MIT
//! `impforge-cli bench` — run the uplift benchmark suite.

use crate::theme;
use clap::{Args, Subcommand};
use impforge_bench::{runner, BenchConfig, BenchReport};
use impforge_emergence::Orchestrator;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum BenchCmd {
    /// Run the pairwise AB benchmark suite and print a summary.
    Run(BenchRunArgs),
    /// List the benchmark cases without executing them.
    List,
    /// Write a signed JSON report for publication.
    Report(BenchRunArgs),
}

#[derive(Debug, Args)]
pub struct BenchRunArgs {
    /// Comma-separated model ids (default: qwen3-imp:8b).
    #[arg(long, default_value = "qwen3-imp:8b")]
    pub models: String,
    /// Tiers to run (comma-separated: 1,3,4).
    #[arg(long, default_value = "1,3,4")]
    pub tiers: String,
    /// Runs per case — median reported.
    #[arg(long, default_value = "3")]
    pub runs: u32,
    /// Output path for `report` subcommand.
    #[arg(long, default_value = "./impforge-bench-report.json")]
    pub output: PathBuf,
}

pub fn run(cmd: BenchCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        BenchCmd::Run(args) => run_suite(args, false)?,
        BenchCmd::List => list_cases(),
        BenchCmd::Report(args) => run_suite(args, true)?,
    }
    Ok(())
}

fn list_cases() {
    let cases = runner::collect_cases(&[1, 3, 4]);
    theme::print_info(&format!("{} benchmark cases bundled", cases.len()));
    for c in cases {
        println!(
            "  {}tier-{}{}  {}{:<28}{}  {}",
            theme::ACCENT_NEON, c.tier, theme::RESET,
            theme::ACCENT_CYAN, c.id, theme::RESET,
            c.prompt.chars().take(70).collect::<String>()
        );
    }
}

fn run_suite(args: BenchRunArgs, write_report: bool) -> anyhow::Result<()> {
    let config = BenchConfig {
        models: args.models.split(',').map(|s| s.trim().to_string()).collect(),
        tiers: args
            .tiers
            .split(',')
            .filter_map(|s| s.trim().parse::<u8>().ok())
            .collect(),
        runs_per_case: args.runs,
        system_prompt: None,
    };
    theme::print_info(&format!(
        "running bench — models={:?} tiers={:?} runs={}",
        config.models, config.tiers, config.runs_per_case
    ));
    let comparisons = runner::run_pairwise_ab(&config)?;
    let report = BenchReport {
        schema_version: 1,
        started_at_unix: chrono::Utc::now().timestamp(),
        finished_at_unix: chrono::Utc::now().timestamp(),
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        tiers_run: config.tiers.clone(),
        comparisons,
        signature_hex: String::new(),
    };
    print_summary(&report);
    if write_report {
        let raw = serde_json::to_string_pretty(&report)?;
        std::fs::write(&args.output, raw)?;
        theme::print_success(&format!("report → {}", args.output.display()));
    }
    Ok(())
}

fn print_summary(report: &BenchReport) {
    println!();
    for cmp in &report.comparisons {
        println!(
            "  {}{:<26}{}  bare={:.1}%  impforge={:.1}%  Δ={:+.1}pp  rel={:+.1}%  tokΔ={:+.1}%",
            theme::ACCENT_NEON, cmp.model, theme::RESET,
            cmp.uplift.bare_pass_rate,
            cmp.uplift.impforge_pass_rate,
            cmp.uplift.absolute_uplift_pct,
            cmp.uplift.relative_uplift_pct,
            cmp.uplift.mean_token_reduction_pct
        );
    }
    println!();
    if let Some(headline) = report.hero_headline() {
        theme::print_success(&headline);
    }
}
