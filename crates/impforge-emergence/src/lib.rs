// SPDX-License-Identifier: MIT
//! # Neural-Trias-Lite — intelligent runtime for impforge-cli
//!
//! Every crate in the workspace registers itself with the [`Orchestrator`]
//! as a [`Module`] implementer.  The runtime then provides:
//!
//! * **Capability Discovery** — ask "who can do X?" across the graph
//! * **Episodic Memory** — every operation leaves a trace in a ring buffer
//!   persisted to `~/.impforge-cli/memory.json`
//! * **Health Monitoring** — each module reports its own state
//! * **Self-Heal Bus** — unhealthy modules get a repair tick and emit a
//!   report
//! * **Introspection** — `impforge-cli introspect` dumps the live module
//!   graph for debugging
//!
//! This is an MIT-licensed distillation of the Emergence Kernel pattern
//! used in the commercial `impforge-aiimp` — same architectural shape,
//! without any engine internals.

pub mod capability;
pub mod health;
pub mod memory;
pub mod module;
pub mod orchestrator;

pub use capability::{Capability, CapabilityCost, CapabilityRequest, CapabilityResponse};
pub use health::{HealthReport, HealthState};
pub use memory::{MemoryEntry, MemoryEntryKind, MemoryStore};
pub use module::{Module, PowerMode};
pub use orchestrator::{ModuleSnapshot, Orchestrator, OrchestratorSnapshot};
