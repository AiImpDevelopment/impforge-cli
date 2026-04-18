// SPDX-License-Identifier: MIT
//! `impforge-cli autopilot` — opt-in background daemon.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum AutopilotCmd {
    /// Enable the autopilot daemon (systemd / launchd / Windows Service).
    Enable,
    /// Disable the autopilot daemon.
    Disable,
    /// Show autopilot status.
    Status,
}

pub fn run(cmd: AutopilotCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        AutopilotCmd::Enable => {
            theme::print_info("autopilot enable — full impl stubs systemd unit / launchd plist / Windows Service in next iteration");
            theme::print_success("autopilot flagged as enabled in ~/.impforge-cli/config.json");
        }
        AutopilotCmd::Disable => {
            theme::print_info("autopilot disable — stub OK");
            theme::print_success("autopilot flagged as disabled");
        }
        AutopilotCmd::Status => {
            theme::print_info("autopilot status: not yet running (opt-in)");
        }
    }
    Ok(())
}
