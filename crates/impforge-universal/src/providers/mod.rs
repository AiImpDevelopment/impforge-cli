// SPDX-License-Identifier: MIT
//! Tool providers — ingest tools FROM a source INTO the universal registry.
//!
//! Each provider knows how to talk to ONE kind of upstream (MCP stdio,
//! OpenAPI spec, Python function wrapper) and converts its native tool
//! descriptors into [`UniversalTool`].  Providers are async by nature
//! (they may spawn processes or fetch remote specs) but expose a sync
//! facade for the registry.

pub mod mcp_client;

use crate::errors::UniversalResult;
use crate::tool::UniversalTool;

/// Any source of tools that the universal registry can ingest from.
pub trait ToolProvider: Send + Sync {
    /// Unique identifier for this provider instance — becomes the tool
    /// source prefix (`"filesystem"`, `"github"`, `"openapi:sentry"`, …).
    fn source(&self) -> &str;

    /// Fetch every tool this provider exposes.  The returned tools MUST
    /// have their `source` field set to `self.source()` and their `id`
    /// field set to `"{source}:{name}"`.
    fn fetch_tools(&self) -> UniversalResult<Vec<UniversalTool>>;
}
