// SPDX-License-Identifier: MIT
//! Crown-Jewel-grade MCP server catalog validator.
//!
//! Loads every `mcp-manifests/servers/*.json`, validates it through three
//! layers:
//!   1. **Schema** — strict typed deserialisation; unknown fields rejected,
//!      required fields enforced, id shape checked.
//!   2. **Business rules** — license must be MIT / Apache-2.0 / BSD /
//!      Elastic-2.0 / MPL-2.0 (no GPL / AGPL / proprietary); command must
//!      be in the allowlist (npx / node / python / uvx / bunx / cargo);
//!      upstream URL must be https:// and on a known code-host domain.
//!   3. **Duplicate-id** — no two manifests share an id.
//!
//! Crown-Jewel grade means every manifest either passes all layers OR is
//! flagged with a specific, actionable error.

use impforge_core::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The full, typed MCP server manifest schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct McpServerManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub transport: TransportKind,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub tools: Vec<String>,
    pub license: String,
    pub category: String,
    pub maintainer: String,
    pub upstream: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Stdio,
    Http,
    Sse,
}

/// Layer-by-layer validation result for one manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub file: PathBuf,
    pub manifest_id: Option<String>,
    pub schema_ok: bool,
    pub license_ok: bool,
    pub command_ok: bool,
    pub url_ok: bool,
    pub tools_ok: bool,
    pub issues: Vec<String>,
}

impl ValidationReport {
    pub fn is_clean(&self) -> bool {
        self.schema_ok
            && self.license_ok
            && self.command_ok
            && self.url_ok
            && self.tools_ok
            && self.issues.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogReport {
    pub root: PathBuf,
    pub total_manifests: usize,
    pub clean: usize,
    pub dirty: usize,
    pub per_manifest: Vec<ValidationReport>,
    pub duplicate_ids: Vec<String>,
}

impl CatalogReport {
    pub fn is_clean(&self) -> bool {
        self.dirty == 0 && self.duplicate_ids.is_empty()
    }
}

/// Allowlist of package runners — anything else is rejected for security.
const ALLOWED_COMMANDS: &[&str] = &[
    "npx", "bunx", "node", "python", "python3", "uvx", "uv", "cargo", "pipx",
];

/// Approved SPDX licences (open-source, non-copyleft).
const APPROVED_LICENSES: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "Elastic-2.0",
    "MPL-2.0",
    "ISC",
    "Unlicense",
    "CC0-1.0",
];

/// Forbidden licences (copyleft or proprietary).
const FORBIDDEN_LICENSES: &[&str] = &[
    "GPL-2.0",
    "GPL-3.0",
    "AGPL-3.0",
    "LGPL-2.1",
    "LGPL-3.0",
    "Proprietary",
    "Commercial",
];

/// Known source-host domains we accept in `upstream`.
const TRUSTED_HOSTS: &[&str] = &[
    "github.com",
    "gitlab.com",
    "bitbucket.org",
    "codeberg.org",
    "sourcehut.org",
];

/// Scan a manifests directory and validate every JSON file.
pub fn validate_catalog(dir: &Path) -> CoreResult<CatalogReport> {
    if !dir.exists() {
        return Err(CoreError::Validation(format!(
            "manifests directory not found: {}",
            dir.display()
        )));
    }

    let mut per_manifest: Vec<ValidationReport> = Vec::new();
    let mut ids_seen: BTreeMap<String, usize> = BTreeMap::new();

    for entry in std::fs::read_dir(dir).map_err(|e| {
        CoreError::Validation(format!("failed to read {}: {e}", dir.display()))
    })? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let report = validate_file(&path);
        if let Some(id) = &report.manifest_id {
            *ids_seen.entry(id.clone()).or_insert(0) += 1;
        }
        per_manifest.push(report);
    }

    let total_manifests = per_manifest.len();
    let clean = per_manifest.iter().filter(|r| r.is_clean()).count();
    let dirty = total_manifests.saturating_sub(clean);
    let duplicate_ids: Vec<String> = ids_seen
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| id)
        .collect();

    per_manifest.sort_by(|a, b| a.file.cmp(&b.file));

    Ok(CatalogReport {
        root: dir.to_path_buf(),
        total_manifests,
        clean,
        dirty,
        per_manifest,
        duplicate_ids,
    })
}

/// Validate one manifest file through all layers.
pub fn validate_file(path: &Path) -> ValidationReport {
    let mut report = ValidationReport {
        file: path.to_path_buf(),
        manifest_id: None,
        schema_ok: false,
        license_ok: false,
        command_ok: false,
        url_ok: false,
        tools_ok: false,
        issues: Vec::new(),
    };

    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) => {
            report.issues.push(format!("read failed: {e}"));
            return report;
        }
    };

    let manifest: McpServerManifest = match serde_json::from_str(&raw) {
        Ok(m) => {
            report.schema_ok = true;
            m
        }
        Err(e) => {
            report.issues.push(format!("schema: {e}"));
            return report;
        }
    };

    report.manifest_id = Some(manifest.id.clone());

    validate_license(&manifest, &mut report);
    validate_command(&manifest, &mut report);
    validate_url(&manifest, &mut report);
    validate_tools(&manifest, &mut report);
    validate_id_shape(&manifest, &mut report);

    report
}

fn validate_license(m: &McpServerManifest, r: &mut ValidationReport) {
    if FORBIDDEN_LICENSES.iter().any(|l| *l == m.license) {
        r.issues
            .push(format!("license '{}' is copyleft/forbidden", m.license));
        return;
    }
    if !APPROVED_LICENSES.iter().any(|l| *l == m.license) {
        r.issues
            .push(format!("license '{}' not in approved list", m.license));
        return;
    }
    r.license_ok = true;
}

fn validate_command(m: &McpServerManifest, r: &mut ValidationReport) {
    match m.transport {
        TransportKind::Stdio => {
            let Some(cmd) = m.command.as_deref() else {
                r.issues.push("stdio transport missing command".to_string());
                return;
            };
            if !ALLOWED_COMMANDS.iter().any(|a| *a == cmd) {
                r.issues.push(format!(
                    "command '{cmd}' not in allowlist ({})",
                    ALLOWED_COMMANDS.join(" / ")
                ));
                return;
            }
            if m.args.is_none() || m.args.as_ref().map_or(true, |a| a.is_empty()) {
                r.issues.push("stdio manifest missing args".to_string());
                return;
            }
        }
        TransportKind::Http | TransportKind::Sse => {
            if m.url.as_ref().map_or(true, |u| u.is_empty()) {
                r.issues.push(format!("{:?} transport missing url", m.transport));
                return;
            }
        }
    }
    r.command_ok = true;
}

fn validate_url(m: &McpServerManifest, r: &mut ValidationReport) {
    let upstream = &m.upstream;
    if !upstream.starts_with("https://") {
        r.issues
            .push(format!("upstream '{upstream}' must be https://"));
        return;
    }
    let host_ok = TRUSTED_HOSTS.iter().any(|h| upstream.contains(h));
    if !host_ok {
        r.issues.push(format!(
            "upstream host not in trusted list ({})",
            TRUSTED_HOSTS.join(" / ")
        ));
        return;
    }
    r.url_ok = true;
}

fn validate_tools(m: &McpServerManifest, r: &mut ValidationReport) {
    if m.tools.is_empty() {
        r.issues.push("tools list empty".to_string());
        return;
    }
    for tool in &m.tools {
        if tool.is_empty()
            || !tool
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            r.issues.push(format!(
                "tool '{tool}' invalid — must be [A-Za-z0-9_-]+"
            ));
            return;
        }
    }
    r.tools_ok = true;
}

fn validate_id_shape(m: &McpServerManifest, r: &mut ValidationReport) {
    if m.id.is_empty() {
        r.issues.push("id is empty".to_string());
    }
    if !m
        .id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        r.issues.push(format!("id '{}' must match [A-Za-z0-9_-]+", m.id));
    }
    if m.id.contains(' ') {
        r.issues.push(format!("id '{}' must not contain spaces", m.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn sample_valid_manifest() -> &'static str {
        r#"{
            "id": "sample",
            "name": "Sample MCP",
            "description": "Sample for testing",
            "transport": "stdio",
            "command": "npx",
            "args": ["-y", "@scope/sample"],
            "url": null,
            "tools": ["do_thing", "do_other"],
            "license": "MIT",
            "category": "testing",
            "maintainer": "scope",
            "upstream": "https://github.com/scope/sample"
        }"#
    }

    #[test]
    fn valid_manifest_passes_all_layers() {
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("sample.json");
        fs::write(&f, sample_valid_manifest()).expect("write");
        let r = validate_file(&f);
        assert!(r.schema_ok);
        assert!(r.license_ok);
        assert!(r.command_ok);
        assert!(r.url_ok);
        assert!(r.tools_ok);
        assert!(r.is_clean(), "issues: {:?}", r.issues);
    }

    #[test]
    fn gpl_license_is_blocked() {
        let raw = sample_valid_manifest().replace("\"MIT\"", "\"GPL-3.0\"");
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("gpl.json");
        fs::write(&f, &raw).expect("write");
        let r = validate_file(&f);
        assert!(!r.license_ok);
        assert!(!r.is_clean());
    }

    #[test]
    fn non_allowlisted_command_is_blocked() {
        let raw = sample_valid_manifest().replace("\"npx\"", "\"evil-curl\"");
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("bad-cmd.json");
        fs::write(&f, &raw).expect("write");
        let r = validate_file(&f);
        assert!(!r.command_ok);
    }

    #[test]
    fn http_url_in_upstream_is_blocked() {
        let raw =
            sample_valid_manifest().replace("\"https://github.com", "\"http://github.com");
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("http.json");
        fs::write(&f, &raw).expect("write");
        let r = validate_file(&f);
        assert!(!r.url_ok);
    }

    #[test]
    fn untrusted_host_is_blocked() {
        let raw = sample_valid_manifest()
            .replace("https://github.com/scope/sample", "https://evil.example.com/sample");
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("untrusted.json");
        fs::write(&f, &raw).expect("write");
        let r = validate_file(&f);
        assert!(!r.url_ok);
    }

    #[test]
    fn empty_tools_list_blocked() {
        let raw = sample_valid_manifest().replace("[\"do_thing\", \"do_other\"]", "[]");
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("empty-tools.json");
        fs::write(&f, &raw).expect("write");
        let r = validate_file(&f);
        assert!(!r.tools_ok);
    }

    #[test]
    fn malformed_tool_name_blocked() {
        let raw = sample_valid_manifest().replace(
            "[\"do_thing\", \"do_other\"]",
            "[\"has spaces invalid\"]",
        );
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("bad-tool.json");
        fs::write(&f, &raw).expect("write");
        let r = validate_file(&f);
        assert!(!r.tools_ok);
    }

    #[test]
    fn kebab_case_tool_names_allowed() {
        let raw = sample_valid_manifest().replace(
            "[\"do_thing\", \"do_other\"]",
            "[\"resolve-library-id\", \"API-post-search\"]",
        );
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("kebab.json");
        fs::write(&f, &raw).expect("write");
        let r = validate_file(&f);
        assert!(r.tools_ok);
    }

    #[test]
    fn empty_id_blocked() {
        let raw = sample_valid_manifest().replace("\"sample\"", "\"\"");
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("empty-id.json");
        fs::write(&f, &raw).expect("write");
        let r = validate_file(&f);
        assert!(!r.is_clean());
    }

    #[test]
    fn duplicate_ids_detected() {
        let dir = tempdir().expect("tmp");
        fs::write(dir.path().join("a.json"), sample_valid_manifest()).expect("write");
        fs::write(dir.path().join("b.json"), sample_valid_manifest()).expect("write");
        let r = validate_catalog(dir.path()).expect("scan");
        assert_eq!(r.total_manifests, 2);
        assert_eq!(r.duplicate_ids, vec!["sample".to_string()]);
        assert!(!r.is_clean());
    }

    #[test]
    fn missing_directory_returns_error() {
        let r = validate_catalog(Path::new("/definitely/does/not/exist"));
        assert!(r.is_err());
    }

    #[test]
    fn schema_rejects_unknown_field() {
        let raw = r#"{
            "id": "sample",
            "name": "Sample MCP",
            "description": "Sample for testing",
            "transport": "stdio",
            "command": "npx",
            "args": ["-y"],
            "url": null,
            "tools": ["x"],
            "license": "MIT",
            "category": "testing",
            "maintainer": "scope",
            "upstream": "https://github.com/scope/sample",
            "ghost_field": "unknown"
        }"#;
        let dir = tempdir().expect("tmp");
        let f = dir.path().join("unknown.json");
        fs::write(&f, raw).expect("write");
        let r = validate_file(&f);
        assert!(!r.schema_ok);
    }

    #[test]
    fn clean_report_count_matches() {
        let dir = tempdir().expect("tmp");
        fs::write(dir.path().join("a.json"), sample_valid_manifest()).expect("write");
        let other = sample_valid_manifest().replace("\"sample\"", "\"other\"");
        fs::write(dir.path().join("b.json"), &other).expect("write");
        let r = validate_catalog(dir.path()).expect("scan");
        assert_eq!(r.clean, 2);
        assert_eq!(r.dirty, 0);
        assert!(r.is_clean());
    }
}
