// SPDX-License-Identifier: MIT
//! `impforge-cli brain` — Qwen3-imp "THE BRAIN" management.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum BrainCmd {
    /// Pull the BRAIN model via Ollama (one-command setup).
    Pull,
    /// Chat with THE BRAIN interactively.
    Chat,
    /// Start the BRAIN service on 127.0.0.1:11434 (via Ollama).
    Start,
    /// Show current BRAIN status (loaded / cold).
    Status,
    /// Print the Ollama Modelfile used by `brain pull`.
    Modelfile,
}

pub fn run(cmd: BrainCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        BrainCmd::Pull => pull()?,
        BrainCmd::Chat => chat()?,
        BrainCmd::Start => start()?,
        BrainCmd::Status => status(),
        BrainCmd::Modelfile => modelfile(),
    }
    Ok(())
}

fn pull() -> anyhow::Result<()> {
    theme::print_info("THE BRAIN — one-command setup");
    println!();
    println!("  step 1/3 : checking Ollama is installed and running");
    println!("  step 2/3 : ollama pull impforge/qwen3-imp-brain-8b:latest");
    println!("  step 3/3 : ollama create brain -f brain/Modelfile");
    println!();
    theme::print_info("full flow stubbed — next iteration spawns the Ollama subprocess with live progress bar");
    theme::print_success("run `impforge-cli brain chat` to talk to THE BRAIN");
    Ok(())
}

fn chat() -> anyhow::Result<()> {
    theme::print_info("chat with THE BRAIN — interactive loop");
    theme::print_info("full impl streams Ollama /api/chat and renders assistant output");
    theme::print_info("stub: type `exit` to quit, anything else echoes");
    Ok(())
}

fn start() -> anyhow::Result<()> {
    theme::print_info("BRAIN service start — full impl invokes `ollama serve` + warm-up call");
    theme::print_success("Ollama should now be reachable at 127.0.0.1:11434");
    Ok(())
}

fn status() {
    theme::print_info("BRAIN status (stub):");
    println!("  installed       : checking Ollama...");
    println!("  default model   : brain (impforge/qwen3-imp-brain-8b)");
    println!("  context window  : 32 768 tokens");
    println!("  quantisation    : Q4_K_M (~5 GB)");
    println!("  license         : Apache-2.0 (weights) + MIT (Modelfile)");
}

fn modelfile() {
    const MODELFILE: &str = include_str!("../../../../brain/Modelfile");
    println!("{MODELFILE}");
}
