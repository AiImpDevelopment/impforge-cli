// SPDX-License-Identifier: MIT
//! `impforge-cli model` subcommand — including the `brain` aliases.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use impforge_models::backend::{Backend, ModelIdentifier};
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum ModelCmd {
    /// List locally-available models (via Ollama).
    List,
    /// Pull a model from a backend.  Shortcut: `brain` → qwen3-imp:8b via Ollama.
    Pull { id: String },
    /// Run a one-shot inference.
    Run {
        id: String,
        prompt: String,
    },
    /// Benchmark local hardware.
    Benchmark,
}

pub fn run(cmd: ModelCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        ModelCmd::List => list(),
        ModelCmd::Pull { id } => pull(&id)?,
        ModelCmd::Run { id, prompt } => run_inference(&id, &prompt)?,
        ModelCmd::Benchmark => benchmark(),
    }
    Ok(())
}

fn list() {
    theme::print_info("listing Ollama models (requires Ollama running at 127.0.0.1:11434)");
    theme::print_info("impforge-cli model list — full impl will stream Ollama /api/tags");
}

fn pull(id: &str) -> anyhow::Result<()> {
    let identifier = if id == "brain" {
        ModelIdentifier { backend: Backend::Ollama, name: "qwen3-imp:8b".to_string() }
    } else {
        ModelIdentifier::parse(id)?
    };
    theme::print_info(&format!(
        "pulling {:?} : {} ",
        identifier.backend, identifier.name
    ));
    theme::print_success(&format!(
        "stub OK — full impl will exec `ollama pull {}` in next iteration",
        identifier.name
    ));
    Ok(())
}

fn run_inference(id: &str, prompt: &str) -> anyhow::Result<()> {
    let identifier = if id == "brain" {
        ModelIdentifier { backend: Backend::Ollama, name: "qwen3-imp:8b".to_string() }
    } else {
        ModelIdentifier::parse(id)?
    };
    theme::print_info(&format!("dispatching to {:?}:{}", identifier.backend, identifier.name));
    theme::print_info(&format!("prompt (truncated): {}", &prompt.chars().take(80).collect::<String>()));
    theme::print_success("stub OK — full impl will stream Ollama /api/generate next iteration");
    Ok(())
}

fn benchmark() {
    theme::print_info("Local hardware benchmark");
    println!("  CPU cores     : {}", num_cpus_guess());
    println!("  Architecture  : {}", std::env::consts::ARCH);
    println!("  OS            : {}", std::env::consts::OS);
    theme::print_info("full GPU/RAM/bandwidth probe lands in the next iteration");
}

fn num_cpus_guess() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}
