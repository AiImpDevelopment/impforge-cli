// SPDX-License-Identifier: MIT
//! impforge-cli — MCP-native AI coding companion.
//!
//! ## Quick start
//!
//! ```bash
//! cargo install impforge-cli
//! impforge-cli template list
//! impforge-cli template scaffold fintech-saas ./my-app
//! impforge-cli mcp register claude-code
//! impforge-cli doctor
//! impforge-cli tui            # opt-in beautiful dashboard
//! ```

use clap::{Parser, Subcommand};
use std::sync::Arc;

mod commands;
mod runtime;
mod theme;

#[derive(Debug, Parser)]
#[command(
    name = "impforge-cli",
    about = "MCP-native AI coding companion · 78 templates · 2 600 compliance rules",
    version,
    author = "ImpForge Maintainers"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Suppress the futuristic banner.
    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage templates (scaffold, list, show compliance).
    #[command(subcommand)]
    Template(commands::template::TemplateCmd),

    /// Manage skills (list, apply).
    #[command(subcommand)]
    Skill(commands::skill::SkillCmd),

    /// Manage MCP integration (serve, register, list).
    #[command(subcommand)]
    Mcp(commands::mcp::McpCmd),

    /// Manage local models (Ollama / HuggingFace / llama.cpp / Candle).
    #[command(subcommand)]
    Model(commands::model::ModelCmd),

    /// Generate a project from a template + local model.
    Generate(commands::generate::GenerateArgs),

    /// Diagnose + self-heal.
    Doctor,

    /// Autopilot daemon (opt-in).
    #[command(subcommand)]
    Autopilot(commands::autopilot::AutopilotCmd),

    /// Update impforge-cli itself.
    Update,

    /// Introspect the live module graph.
    Introspect,

    /// Export a signed config bundle for impforge-aiimp migration.
    ExportConfig {
        #[arg(long, short, default_value = "~/.impforge-cli/export.json")]
        output: String,
    },

    /// Contribute a new template / skill / MCP manifest.
    #[command(subcommand)]
    Contribute(commands::contribute::ContributeCmd),

    /// Open the impforge-aiimp (Pro) upgrade page.
    Upgrade,

    /// Manage THE BRAIN (qwen3-imp:8b — the exact model that powers Pro).
    #[command(subcommand)]
    Brain(commands::brain::BrainCmd),

    /// Qwen3-imp end-to-end QA audit of all bundled content.
    Audit(commands::audit::AuditArgs),

    /// Crown-Jewel Guardian — 7-dimension code + behavior quality gate.
    #[command(subcommand, name = "crown-jewel")]
    CrownJewel(commands::crown_jewel::CrownJewelCmd),

    /// Scientific uplift benchmark — bare Ollama vs impforge-cli context.
    #[command(subcommand)]
    Bench(commands::bench::BenchCmd),

    /// Mobile bridge (Signal / Telegram / WhatsApp) — status, allowlist, test send.
    #[command(subcommand)]
    Remote(commands::remote::RemoteCmd),

    /// Multi-provider BYOK chat (OpenAI / Anthropic / Gemini / Ollama / OpenRouter).
    #[command(subcommand)]
    Provider(commands::provider::ProviderCmd),

    /// Index a file or directory into the local FTS5 knowledge base.
    Ingest(commands::ingest::IngestArgs),

    /// Search the local FTS5 knowledge base (BM25 ranked).
    Search(commands::search::SearchArgs),

    /// Launch the futuristic TUI dashboard.
    #[cfg(feature = "tui")]
    Tui,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "impforge_cli=info,warn".into()),
        )
        .with_target(false)
        .init();

    if !cli.quiet {
        theme::print_banner();
    }

    let orchestrator = Arc::new(runtime::bootstrap_orchestrator()?);

    match cli.command {
        Command::Template(cmd) => commands::template::run(cmd, &orchestrator)?,
        Command::Skill(cmd) => commands::skill::run(cmd, &orchestrator)?,
        Command::Mcp(cmd) => commands::mcp::run(cmd, &orchestrator)?,
        Command::Model(cmd) => commands::model::run(cmd, &orchestrator)?,
        Command::Generate(args) => commands::generate::run(args, &orchestrator)?,
        Command::Doctor => commands::doctor::run(&orchestrator)?,
        Command::Autopilot(cmd) => commands::autopilot::run(cmd, &orchestrator)?,
        Command::Update => commands::update::run(&orchestrator)?,
        Command::Introspect => commands::introspect::run(&orchestrator)?,
        Command::ExportConfig { output } => commands::export::run(&output, &orchestrator)?,
        Command::Contribute(cmd) => commands::contribute::run(cmd, &orchestrator)?,
        Command::Upgrade => commands::upgrade::run()?,
        Command::Brain(cmd) => commands::brain::run(cmd, &orchestrator)?,
        Command::Audit(args) => commands::audit::run(args, &orchestrator)?,
        Command::CrownJewel(cmd) => commands::crown_jewel::run(cmd, &orchestrator)?,
        Command::Bench(cmd) => commands::bench::run(cmd, &orchestrator)?,
        Command::Remote(cmd) => commands::remote::run(cmd, &orchestrator)?,
        Command::Provider(cmd) => commands::provider::run(cmd, &orchestrator)?,
        Command::Ingest(args) => commands::ingest::run(args, &orchestrator)?,
        Command::Search(args) => commands::search::run(args, &orchestrator)?,
        #[cfg(feature = "tui")]
        Command::Tui => commands::tui::run(&orchestrator)?,
    }

    let _ = orchestrator.hibernate_all();
    Ok(())
}
