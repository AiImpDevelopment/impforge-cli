// SPDX-License-Identifier: MIT
//! Cross-platform XDG-aware paths for impforge-cli.

use crate::error::{CoreError, CoreResult};
use std::path::PathBuf;

/// `~/.impforge-cli/` or the platform-specific equivalent.
pub fn config_dir() -> CoreResult<PathBuf> {
    let base = dirs::home_dir()
        .ok_or_else(|| CoreError::other("could not resolve $HOME"))?;
    Ok(base.join(".impforge-cli"))
}

/// `~/.impforge-cli/cache/` — model downloads, manifests, etc.
pub fn cache_dir() -> CoreResult<PathBuf> {
    Ok(config_dir()?.join("cache"))
}

/// `~/.impforge-cli/models/` — local GGUF / safetensors snapshots.
pub fn models_dir() -> CoreResult<PathBuf> {
    Ok(config_dir()?.join("models"))
}

/// `~/.impforge-cli/logs/` — autopilot daemon logs.
pub fn logs_dir() -> CoreResult<PathBuf> {
    Ok(config_dir()?.join("logs"))
}

/// `~/.impforge-cli/health.json` — autopilot watchdog state.
pub fn health_file() -> CoreResult<PathBuf> {
    Ok(config_dir()?.join("health.json"))
}

/// `~/.impforge-cli/export.json` — signed migration export for impforge-aiimp.
pub fn export_file() -> CoreResult<PathBuf> {
    Ok(config_dir()?.join("export.json"))
}

/// Bundled templates / skills / mcp-manifests root inside a cargo-installed
/// binary.  When running from source, falls back to `<workspace>/..`.
pub fn bundled_content_dir() -> CoreResult<PathBuf> {
    let exe = std::env::current_exe().map_err(CoreError::Io)?;
    let parent = exe.parent().ok_or_else(|| CoreError::other("exe has no parent"))?;
    Ok(parent.join("impforge-content"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_under_home() {
        let d = config_dir().expect("config_dir");
        assert!(d.ends_with(".impforge-cli"));
    }

    #[test]
    fn cache_under_config() {
        let c = cache_dir().expect("cache");
        let p = config_dir().expect("config");
        assert!(c.starts_with(p));
    }

    #[test]
    fn health_file_is_json() {
        let h = health_file().expect("health");
        assert_eq!(h.file_name().and_then(|s| s.to_str()), Some("health.json"));
    }
}
