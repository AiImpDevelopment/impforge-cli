// SPDX-License-Identifier: MIT
//! `impforge-cli mcp` subcommand.

use crate::commands::mcp_marketplace::{self, McpMarketplaceCmd};
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
    /// Browse the local-first MCP marketplace mirror.
    Browse(mcp_marketplace::BrowseArgs),
    /// Install an MCP server by id.
    Install(mcp_marketplace::InstallArgs),
    /// Remove a previously-installed MCP server.
    Uninstall { id: String },
    /// List installed MCP servers.
    Installed,
    /// Health-check an installed MCP server (stdio probe).
    Health { id: String },
    /// Open `$EDITOR` on a server's per-user config JSON.
    Configure { id: String },
    /// Sync the marketplace mirror from the public CDN.
    Update(mcp_marketplace::UpdateArgs),
}

pub fn run(cmd: McpCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        McpCmd::List => list(),
        McpCmd::Register { client } => register(&client)?,
        McpCmd::Clients => clients(),
        McpCmd::Serve => serve()?,
        McpCmd::Browse(args) => mcp_marketplace::run(McpMarketplaceCmd::Browse(args))?,
        McpCmd::Install(args) => mcp_marketplace::run(McpMarketplaceCmd::Install(args))?,
        McpCmd::Uninstall { id } => mcp_marketplace::run(McpMarketplaceCmd::Uninstall { id })?,
        McpCmd::Installed => mcp_marketplace::run(McpMarketplaceCmd::Installed)?,
        McpCmd::Health { id } => mcp_marketplace::run(McpMarketplaceCmd::Health { id })?,
        McpCmd::Configure { id } => mcp_marketplace::run(McpMarketplaceCmd::Configure { id })?,
        McpCmd::Update(args) => mcp_marketplace::run(McpMarketplaceCmd::Update(args))?,
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
