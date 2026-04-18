// SPDX-License-Identifier: MIT
//! `impforge-cli doctor` — health check + self-heal across all modules.

use crate::theme;
use impforge_emergence::Orchestrator;
use std::sync::Arc;

pub fn run(orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    theme::print_info("running doctor — health check across every module");
    let now = chrono::Utc::now().timestamp();
    let reports = orc.tick_health(now)?;
    let mut healthy = 0;
    let mut total = 0;
    for report in &reports {
        total += 1;
        let state_str = report.state.as_str();
        let color = if report.state.is_healthy() {
            healthy += 1;
            theme::ACCENT_NEON
        } else {
            theme::ACCENT_MAGENTA
        };
        println!("  {}{:<9}{}  {}", color, state_str, theme::RESET, report.detail);
    }
    theme::print_info(&format!("{healthy}/{total} modules healthy"));

    let heal_entries = orc.tick_self_heal(now)?;
    if heal_entries.is_empty() {
        theme::print_success("no self-heal needed — everything is fine");
    } else {
        theme::print_warning(&format!(
            "attempted self-heal on {} module(s)",
            heal_entries.len()
        ));
        for entry in heal_entries {
            println!("  {} → {}", entry.module_id, entry.summary);
        }
    }
    Ok(())
}
