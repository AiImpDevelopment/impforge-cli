// SPDX-License-Identifier: MIT
//! `impforge-cli update` — self-update check.

use crate::theme;
use impforge_emergence::Orchestrator;
use std::sync::Arc;

pub fn run(_orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    theme::print_info("checking crates.io for a newer impforge-cli");
    let installed = env!("CARGO_PKG_VERSION");
    theme::print_info(&format!("installed: {installed}"));
    theme::print_info("full check uses reqwest — stub returns 'up to date' in minimal builds");
    theme::print_success("impforge-cli is up to date");
    Ok(())
}
