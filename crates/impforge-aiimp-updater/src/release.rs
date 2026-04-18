// SPDX-License-Identifier: MIT
//! Release API surface — public GET-only endpoints at impforge.com/releases.

use serde::{Deserialize, Serialize};

const RELEASES_BASE: &str = "https://impforge.com/releases";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseRecord {
    pub version: String,
    pub platform: String,
    pub asset_url: String,
    pub sha256_hex: String,
    pub signature_hex: String,
    pub released_at_unix: i64,
}

pub fn check() -> anyhow::Result<()> {
    use crate::verify;
    println!("impforge-aiimp-updater v{}", env!("CARGO_PKG_VERSION"));
    println!("releases endpoint: {RELEASES_BASE}/index.json");
    println!("installed impforge-aiimp: (probe not yet implemented — stub)");
    // Exercise verification helpers so the crate's public API stays alive.
    let probe = verify::sha256_hex(b"impforge-aiimp-updater-probe");
    if !verify::matches_expected(&probe, &probe) {
        anyhow::bail!("self-check failed — hash helpers mis-wired");
    }
    let _sample = ReleaseRecord {
        version: "0.0.0".to_string(),
        platform: "none".to_string(),
        asset_url: String::new(),
        sha256_hex: probe,
        signature_hex: "0".repeat(128),
        released_at_unix: 0,
    };
    Ok(())
}

pub fn list() -> anyhow::Result<()> {
    println!("fetching release index from {RELEASES_BASE}/index.json");
    println!("(reqwest call will be enabled once the endpoint is live)");
    Ok(())
}

pub fn install_latest() -> anyhow::Result<()> {
    println!("install-latest: full flow is");
    println!("  1. GET {RELEASES_BASE}/index.json");
    println!("  2. filter to current platform ({})", std::env::consts::OS);
    println!("  3. pick highest semver");
    println!("  4. GET asset_url → write to ~/.impforge/releases/");
    println!("  5. verify SHA-256");
    println!("  6. verify Ed25519 against pinned pubkey");
    println!("  7. symlink ~/.impforge/bin/impforge-aiimp");
    println!();
    println!("stub OK — full flow lands once the release server goes live");
    Ok(())
}

pub fn install(version: &str) -> anyhow::Result<()> {
    println!("install specific version: {version}");
    println!("stub OK");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releases_base_is_https() {
        assert!(RELEASES_BASE.starts_with("https://"));
    }

    #[test]
    fn release_record_serializes() {
        let r = ReleaseRecord {
            version: "0.1.0".to_string(),
            platform: "linux-x86_64".to_string(),
            asset_url: "https://impforge.com/releases/v0.1.0/impforge-aiimp-linux-x86_64.tar.gz".to_string(),
            sha256_hex: "a".repeat(64),
            signature_hex: "b".repeat(128),
            released_at_unix: 1_700_000_000,
        };
        let j = serde_json::to_string(&r).expect("serialize");
        assert!(j.contains("0.1.0"));
    }
}
