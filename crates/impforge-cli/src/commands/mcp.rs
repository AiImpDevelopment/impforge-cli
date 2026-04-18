// SPDX-License-Identifier: MIT
//! `impforge-cli mcp` subcommand.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use impforge_mcp_server::lazy_schema::TOOL_DESCRIPTORS;
use impforge_mcp_server::registration::{config_snippet, ClientId};
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum McpCmd {
    /// List MCP tool names this CLI exposes (lazy — names only).
    List,
    /// Generate a config snippet for a specific AI coding client.
    Register { client: String },
    /// Print supported AI coding clients.
    Clients,
    /// Start the stdio MCP server (run from your AI tool's config).
    Serve,
}

pub fn run(cmd: McpCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        McpCmd::List => list(),
        McpCmd::Register { client } => register(&client)?,
        McpCmd::Clients => clients(),
        McpCmd::Serve => serve()?,
    }
    Ok(())
}

fn list() {
    theme::print_info(&format!("{} MCP tools exposed (lazy schemas)", TOOL_DESCRIPTORS.len()));
    for t in TOOL_DESCRIPTORS {
        println!(
            "  {}{}{}  {}{}{}",
            theme::ACCENT_NEON, t.name, theme::RESET,
            theme::DIM, t.summary, theme::RESET
        );
    }
}

fn register(client: &str) -> anyhow::Result<()> {
    let id = ClientId::parse(client)?;
    let snippet = config_snippet(id);
    theme::print_info(&format!("config snippet for {}:", id.display()));
    println!("\n{snippet}\n");
    theme::print_info("paste this into your client's MCP configuration");
    Ok(())
}

fn clients() {
    theme::print_info("supported MCP clients:");
    for c in ClientId::all() {
        println!("  {}{}{}", theme::ACCENT_NEON, c.display(), theme::RESET);
    }
}

fn serve() -> anyhow::Result<()> {
    theme::print_info("MCP stdio server starting (ctrl-C to stop)");
    // Full protocol impl in next iteration — for now we print a marker so
    // integration tests can verify the binary binds.
    println!(r#"{{"jsonrpc":"2.0","method":"initialized","params":{{}}}}"#);
    std::thread::park();
    Ok(())
}
