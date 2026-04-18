// SPDX-License-Identifier: MIT
//! `impforge-cli remote` — mobile bridge status / test / allowlist.

use crate::theme;
use clap::Subcommand;
use impforge_emergence::Orchestrator;
use impforge_remote::{
    allowlist, bridge::BridgeKind, telegram::TelegramBridge, Bridge,
};
use std::sync::Arc;

#[derive(Debug, Subcommand)]
pub enum RemoteCmd {
    /// Show bridge configuration + reachability.
    Status,
    /// Print the allowlist of commands the bridge will forward.
    Allowlist,
    /// Check whether a command would be allowed through the bridge.
    Check { command: Vec<String> },
    /// Send a one-off test reply through the Telegram bridge.
    SendTest {
        #[arg(long)]
        chat_id: String,
        #[arg(long, default_value = "impforge-cli remote bridge test OK")]
        message: String,
    },
}

pub fn run(cmd: RemoteCmd, _orc: &Arc<Orchestrator>) -> anyhow::Result<()> {
    match cmd {
        RemoteCmd::Status => status(),
        RemoteCmd::Allowlist => print_allowlist(),
        RemoteCmd::Check { command } => check(&command.join(" ")),
        RemoteCmd::SendTest { chat_id, message } => send_test(&chat_id, &message)?,
    }
    Ok(())
}

fn status() {
    let telegram = TelegramBridge::new();
    theme::print_info("remote bridge status:");
    for kind in BridgeKind::all() {
        let ok = match kind {
            BridgeKind::Telegram => telegram.is_configured(),
            _ => false,
        };
        let color = if ok { theme::ACCENT_NEON } else { theme::DIM };
        println!(
            "  {}{:<12}{}  {}",
            color,
            kind.display(),
            theme::RESET,
            if ok {
                "configured"
            } else {
                "not configured"
            }
        );
    }
    if !telegram.is_configured() {
        theme::print_info("set IMPFORGE_TELEGRAM_TOKEN env var to enable Telegram bridge");
    }
}

fn print_allowlist() {
    theme::print_info(&format!(
        "{} commands allowed via remote bridge (free tier):",
        allowlist::ALLOWED_COMMANDS.len()
    ));
    for c in allowlist::ALLOWED_COMMANDS {
        println!("  {}{}{}", theme::ACCENT_NEON, c, theme::RESET);
    }
    println!();
    theme::print_info(&format!(
        "{} commands BLOCKED (Pro-only or writes):",
        allowlist::BLOCKED_COMMANDS.len()
    ));
    for c in allowlist::BLOCKED_COMMANDS {
        println!("  {}{}{}", theme::ACCENT_MAGENTA, c, theme::RESET);
    }
}

fn check(command: &str) {
    if allowlist::is_command_allowed(command) {
        theme::print_success(&format!("'{command}' is allowed through the remote bridge"));
    } else {
        theme::print_warning(&format!("'{command}' is BLOCKED"));
        println!("{}", allowlist::upgrade_message_for_blocked(command));
    }
}

fn send_test(chat_id: &str, message: &str) -> anyhow::Result<()> {
    let bridge = TelegramBridge::new();
    if !bridge.is_configured() {
        anyhow::bail!("IMPFORGE_TELEGRAM_TOKEN not set — export it first");
    }
    let msg = impforge_remote::BridgeMessage {
        sender: chat_id.to_string(),
        text: String::new(),
        received_at_unix: chrono::Utc::now().timestamp(),
        kind: BridgeKind::Telegram,
    };
    bridge.send_reply(&msg, message)?;
    theme::print_success(&format!("sent Telegram reply to chat '{chat_id}'"));
    Ok(())
}
