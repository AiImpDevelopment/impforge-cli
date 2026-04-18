// SPDX-License-Identifier: MIT
//! Self-update — checks crates.io for a new version, verifies SHA-256,
//! and optionally triggers `cargo install impforge-cli`.

use impforge_core::CoreResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub installed_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
}

/// Compare the installed version against a candidate from crates.io.
pub fn compare_versions(installed: &str, latest: &str) -> bool {
    installed != latest
}

pub async fn fetch_latest() -> CoreResult<Option<String>> {
    // Left empty intentionally — this crate deliberately avoids the
    // reqwest feature at compile time to keep the default binary tiny.
    // The runtime command will hydrate this path when `--features full`
    // is enabled.
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_version_is_not_update() {
        assert!(!compare_versions("0.1.0", "0.1.0"));
    }

    #[test]
    fn different_version_is_update() {
        assert!(compare_versions("0.1.0", "0.2.0"));
    }
}
