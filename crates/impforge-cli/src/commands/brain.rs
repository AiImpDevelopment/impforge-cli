// SPDX-License-Identifier: MIT
//! `impforge-cli brain` — Qwen3-imp "THE BRAIN" management.  Real wiring
//! to the local Ollama daemon.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use impforge_models::ollama;
use std::io::{self, BufRead, Write};
use std::process::Command as ProcessCommand;
use std::sync::Arc;

const BRAIN_MODEL: &str = "qwen3-imp:8b";

#[derive(Debug, Subcommand)]
pub enum BrainCmd {
    /// Pull THE BRAIN model via Ollama (one-command setup).
    Pull,
    /// Chat with THE BRAIN interactively.
    Chat,
    /// Start Ollama (if installed) and warm up THE BRAIN.
    Start,
    /// Show current BRAIN status (loaded / cold).
    Status,
    /// Print the Ollama Modelfile.
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
    theme::print_info("THE BRAIN setup — pulling qwen3-imp:8b via Ollama");
    let status = ProcessCommand::new("ollama").arg("pull").arg(BRAIN_MODEL).status();
    match status {
        Ok(s) if s.success() => {
            theme::print_success("THE BRAIN is ready");
            theme::print_info("next: `impforge-cli brain chat` to talk to THE BRAIN");
            Ok(())
        }
        Ok(s) => anyhow::bail!("ollama pull exited with {s}"),
        Err(e) => {
            theme::print_warning(&format!("Ollama CLI not found on PATH ({e})"));
            theme::print_info("install Ollama from https://ollama.com and retry `impforge-cli brain pull`");
            Ok(())
        }
    }
}

fn chat() -> anyhow::Result<()> {
    if !ollama::is_reachable(None) {
        theme::print_error("Ollama is not reachable at 127.0.0.1:11434");
        theme::print_info("install Ollama from https://ollama.com and run `ollama serve`");
        return Ok(());
    }
    theme::print_info("Chatting with THE BRAIN — type `exit` or Ctrl-D to quit");
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    loop {
        write!(out, "{}you{} > ", theme::ACCENT_MAGENTA, theme::RESET)?;
        out.flush()?;
        let mut line = String::new();
        let read = stdin.lock().read_line(&mut line)?;
        if read == 0 {
            break;
        }
        let prompt = line.trim();
        if prompt.is_empty() {
            continue;
        }
        if prompt == "exit" || prompt == "quit" {
            break;
        }
        match ollama::generate_once(BRAIN_MODEL, prompt, None, None) {
            Ok(resp) => {
                write!(out, "{}brain{} > {}\n\n", theme::ACCENT_NEON, theme::RESET, resp.response.trim())?;
                out.flush()?;
            }
            Err(e) => {
                theme::print_error(&format!("inference failed: {e}"));
            }
        }
    }
    theme::print_info("goodbye");
    Ok(())
}

fn start() -> anyhow::Result<()> {
    if ollama::is_reachable(None) {
        theme::print_success("Ollama already running at 127.0.0.1:11434");
    } else {
        theme::print_info("Ollama not running — attempting `ollama serve` in background");
        match ProcessCommand::new("ollama").arg("serve").spawn() {
            Ok(child) => {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if ollama::is_reachable(None) {
                    theme::print_success("Ollama started");
                } else {
                    theme::print_warning("Ollama not yet reachable — check logs");
                }
                let _ = child.id(); // deliberately leak the handle; user controls lifetime
            }
            Err(e) => {
                theme::print_error(&format!("failed to start Ollama: {e}"));
            }
        }
    }
    if ollama::is_reachable(None) {
        let loaded = ollama::list_local_models(None)?;
        let present = loaded.iter().any(|m| m.name == BRAIN_MODEL || m.model == BRAIN_MODEL);
        if !present {
            theme::print_info("THE BRAIN not pulled yet — run `impforge-cli brain pull`");
        } else {
            theme::print_success("THE BRAIN is installed and ready to serve");
        }
    }
    Ok(())
}

fn status() {
    let reachable = ollama::is_reachable(None);
    theme::print_info("BRAIN status:");
    println!("  Ollama          : {}", if reachable { "reachable" } else { "not running" });
    if reachable {
        match ollama::list_local_models(None) {
            Ok(models) => {
                let present = models.iter().any(|m| m.name == BRAIN_MODEL || m.model == BRAIN_MODEL);
                println!(
                    "  BRAIN installed : {}",
                    if present { "yes (qwen3-imp:8b)" } else { "no — run `brain pull`" }
                );
            }
            Err(e) => {
                println!("  model probe     : failed ({e})");
            }
        }
    }
    println!("  default model   : {BRAIN_MODEL}");
    println!("  context window  : 32 768 tokens");
    println!("  quantisation    : Q4_K_M (~5 GB)");
    println!("  license         : Apache-2.0 (weights) + MIT (Modelfile)");
}

fn modelfile() {
    const MODELFILE: &str = include_str!("../../../../brain/Modelfile");
    println!("{MODELFILE}");
}
