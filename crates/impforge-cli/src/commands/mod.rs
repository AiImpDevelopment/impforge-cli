// SPDX-License-Identifier: MIT
//! Command handlers — each subcommand has its own module.

pub mod audit;
pub mod autopilot;
pub mod bench;
pub mod brain;
pub mod contribute;
pub mod crown_jewel;
pub mod doctor;
pub mod export;
pub mod generate;
pub mod digest;
pub mod exec;
pub mod ingest;
pub mod introspect;
pub mod mcp;
pub mod mcp_marketplace;
pub mod model;
pub mod provider;
pub mod remote;
pub mod search;
pub mod skill;
pub mod template;
pub mod update;
pub mod upgrade;

#[cfg(feature = "tui")]
pub mod tui;
