// SPDX-License-Identifier: MIT
//! `impforge-cli introspect` — dump the live module graph.

use crate::theme;
use impforge_emergence::Orchestrator;
use std::sync::Arc;

pub fn run(orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    let snap = orc.introspect()?;
    theme::print_info(&format!("modules registered: {}", snap.modules.len()));
    theme::print_info(&format!("memory entries: {}", snap.memory_entries));
    println!();
    for m in snap.modules {
        println!(
            "  {}{:<22}{} {}{:>10}{} {}",
            theme::ACCENT_NEON, m.id, theme::RESET,
            theme::ACCENT_CYAN, m.health.state.as_str(), theme::RESET,
            m.description
        );
        if !m.capabilities.is_empty() {
            println!(
                "    capabilities: {}",
                m.capabilities.join(" · ")
            );
        }
        if m.is_lazy_mcp {
            println!("    {}lazy-mcp{}", theme::ACCENT_MAGENTA, theme::RESET);
        }
    }
    Ok(())
}
