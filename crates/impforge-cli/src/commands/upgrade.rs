// SPDX-License-Identifier: MIT
//! `impforge-cli upgrade` — prints Pro-upgrade info and a URL.

use crate::theme;

pub fn run() -> anyhow::Result<()> {
    theme::print_info("impforge-aiimp (Pro) — the full commercial AI workstation");
    println!();
    println!("  {}Free (impforge-cli){}              →   {}Pro (impforge-aiimp){}",
        theme::ACCENT_CYAN, theme::RESET, theme::ACCENT_NEON, theme::RESET);
    println!("  ───────────────────────────────────────────────────────────");
    println!("  78 templates                      →   78 templates + Pro Mesh join");
    println!("  2 600 compliance rules            →   157 870 Crown-Jewel quality rules");
    println!("  THE BRAIN (qwen3-imp 8B)          →   4-Model Collaboration Pipeline");
    println!("  Ollama / HF / llama.cpp           →   + Training Studio (DoRA / QLoRA)");
    println!("  Community MCP manifests           →   Quarantine Layer + signed snapshots");
    println!("  Terminal CLI                      →   Tauri desktop app + Live Preview");
    println!();
    println!("  {}Upgrade at: https://impforge.com/pro  ·  EUR 25/month{}", theme::ACCENT_NEON, theme::RESET);
    println!();
    theme::print_info("migration path:");
    theme::print_info("  impforge-cli export-config ~/.impforge-cli/export.json");
    theme::print_info("  (download impforge-aiimp, then:)");
    theme::print_info("  impforge-aiimp import-cli-config ~/.impforge-cli/export.json");
    Ok(())
}
