// SPDX-License-Identifier: MIT
//! # impforge-remote — mobile bridge for impforge-cli.
//!
//! Lets a user send commands from their phone (Signal / Telegram /
//! WhatsApp Business) to their local impforge-cli running on their
//! workstation.  The bridge is **outbound-only** — the user talks TO
//! their CLI, never the other way around, so it never leaks private
//! context out of the laptop.
//!
//! Supported allowed commands (freemium, read-only):
//!
//! * `template list` · `template show <id>`
//! * `compliance <id>` (show compliance rules)
//! * `skill list` · `skill show <id>`
//! * `mcp list` · `doctor`
//! * `brain chat <prompt>` (if user opts in)
//!
//! Blocked commands (Pro-only — bridge returns `upgrade` link):
//!
//! * `template scaffold` (writes to FS)
//! * `mcp serve` (long-running)
//! * `autopilot` (daemon)
//! * everything under `crown-jewel gate`

pub mod allowlist;
pub mod bridge;
pub mod module;
pub mod telegram;

pub use allowlist::{is_command_allowed, ALLOWED_COMMANDS};
pub use bridge::{Bridge, BridgeKind, BridgeMessage};
pub use module::Module_;
