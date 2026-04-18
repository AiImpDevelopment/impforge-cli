// SPDX-License-Identifier: MIT
//! `impforge-cli generate` — scaffold + local-model generation.

use crate::theme;
use clap::Args;
use impforge_emergence::Orchestrator;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// Template id (e.g. `fintech-saas`).
    #[arg(long)]
    pub template: String,

    /// Local model id (e.g. `brain`, `qwen2.5-coder:7b`).
    #[arg(long, default_value = "brain")]
    pub model: String,

    /// Output directory.
    #[arg(long, short, default_value = "./impforge-generated")]
    pub output: PathBuf,

    /// Free-text user intent.
    #[arg(long)]
    pub prompt: Option<String>,
}

pub fn run(args: GenerateArgs, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    theme::print_info(&format!("generating {} @ {} → {}", args.template, args.model, args.output.display()));
    theme::print_info("step 1/3 — scaffolding template");
    theme::print_info("step 2/3 — running local model on user intent");
    theme::print_info("step 3/3 — applying model-suggested customisations");
    if let Some(p) = args.prompt {
        theme::print_info(&format!("user intent: {p}"));
    }
    theme::print_success("stub OK — full 3-step impl lands in next iteration");
    Ok(())
}
