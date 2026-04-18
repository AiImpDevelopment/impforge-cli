// SPDX-License-Identifier: MIT
//! # impforge-core — shared types for the impforge-cli workspace.
//!
//! This crate holds the canonical `TemplateManifest`, `ComplianceRule`,
//! `SkillManifest`, and `McpManifest` types, plus error definitions.
//!
//! ## Isolation contract
//!
//! impforge-core must NEVER import from the ImpForge (commercial) engine.
//! It stays a clean, auditable MIT crate that can be published to
//! crates.io without leaking proprietary code.

pub mod config;
pub mod error;
pub mod manifest;
pub mod paths;

pub use error::{CoreError, CoreResult};
pub use manifest::{
    ComplianceRule, McpManifest, SkillManifest, TemplateManifest, ALLOWED_CATEGORIES,
    ALLOWED_LICENSES, ALLOWED_SAFETY_CLASSES,
};
