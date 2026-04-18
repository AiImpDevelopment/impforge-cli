// SPDX-License-Identifier: MIT
//! # Crown-Jewel Guardian — 8-dimension code + behavior quality gate.
//!
//! Walks a directory tree and the local agent telemetry and scores across
//! eight enforcement dimensions:
//!
//! | # | Dimension | What it catches |
//! |---|-----------|-----------------|
//! | 1 | **No stubs**             | `stub OK`, `next iteration`, `unimplemented!()`, `todo!()`, marketing-excuse comments |
//! | 2 | **No suppression**       | `#[allow(*)]`, `#![allow]`, typos like `alloy` / `allo`, `rustfmt::skip` |
//! | 3 | **No lonely unwrap**     | `.unwrap()` without a matching test that exercises the un-wrapped path |
//! | 4 | **Test-first**           | Every `pub fn` / `pub struct` / `pub enum` has at least one `#[test]` |
//! | 5 | **Crown-Jewel wiring**   | Every new workspace module implements `Module` and is registered in `bootstrap_orchestrator()` |
//! | 6 | **Parallel-work efficiency** | When a long-running task runs in background, the agent must start independent work within threshold — no idle waiting |
//! | 7 | **Error recall**         | Errors from cargo / clippy / pnpm that appeared in a previous run and re-appear today — blocking regression |
//! | 8 | **Kind routing** (NEW)   | Every dispatcher (hub / bus / router) must inspect a `kind` / `MessageKind` / `Direction` field before choosing a recipient — blind fan-out is a privacy + UX violation |
//!
//! Dims 1-5 + 8 are static file scans.  Dims 6-7 are behavioral, driven by
//! persistent telemetry in `~/.impforge-cli/`.
//!
//! Free users get the scanner; Pro users get the full gate wired into the
//! Quarantine Layer + auto-fix via BRAIN.

pub mod behavior;
pub mod dims;
pub mod kind_routing;
pub mod module;
pub mod report;
pub mod scanner;

pub use behavior::{
    agent_trace_path, append_trace, dim6_parallel_efficiency, dim7_error_recall,
    error_recall_path, fingerprint_error, load_recall_store, read_trace, save_recall_store,
    AgentTraceEntry, AgentTraceKind, ErrorRecallEntry,
};
pub use kind_routing::scan_rust_file as dim8_kind_routing;
pub use module::Module_;
pub use report::{CrownJewelFinding, CrownJewelReport, Dimension, Severity};
pub use scanner::{scan, scan_workspace};
