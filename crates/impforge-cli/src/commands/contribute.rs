// SPDX-License-Identifier: MIT
//! `impforge-cli contribute` — community-contribution wizard.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum ContributeCmd {
    /// Submit a new template.
    Template,
    /// Submit a new skill.
    Skill,
    /// Submit a new MCP manifest.
    McpManifest,
}

pub fn run(cmd: ContributeCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        ContributeCmd::Template => {
            theme::print_info("contribute template — interactive wizard starting");
            theme::print_info("step 1: metadata (id, name, description, category, industry, framework)");
            theme::print_info("step 2: files (we'll walk your scaffold dir)");
            theme::print_info("step 3: local validation (schema + security-scan + prompt-injection-scrubber)");
            theme::print_info("step 4: PR body generation");
            theme::print_info("step 5: open https://github.com/AiImpDevelopment/impforge-cli/compare in browser");
            theme::print_success("wizard stub ready — full TTY prompt impl lands in next iteration");
        }
        ContributeCmd::Skill => {
            theme::print_info("contribute skill — wizard stub (same 5-step flow)");
            theme::print_success("stub OK");
        }
        ContributeCmd::McpManifest => {
            theme::print_info("contribute mcp-manifest — wizard stub (same 5-step flow)");
            theme::print_success("stub OK");
        }
    }
    Ok(())
}
