// SPDX-License-Identifier: MIT
//! Command handlers — each subcommand has its own module.

pub mod autopilot;
pub mod contribute;
pub mod doctor;
pub mod export;
pub mod generate;
pub mod introspect;
pub mod mcp;
pub mod model;
pub mod skill;
pub mod template;
pub mod update;
pub mod upgrade;

#[cfg(feature = "tui")]
pub mod tui;
