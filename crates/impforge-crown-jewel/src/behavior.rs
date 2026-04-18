// SPDX-License-Identifier: MIT
//! Dimensions 6 + 7 — behavioral checks driven by persistent telemetry.
//!
//! Dim 6: parallel-work efficiency.  Tracks agent tool-call timeline and
//! flags idle-waiting windows that exceed backend-specific thresholds.
//!
//! Dim 7: error recall.  Fingerprints every error seen in a cargo / clippy
//! / pnpm run; flags any error whose fingerprint re-appears on a later
//! run without an intervening fix.

use crate::report::{CrownJewelFinding, Dimension, Severity};
use impforge_core::{paths, CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTraceEntry {
    pub at_unix_ms: i64,
    pub kind: AgentTraceKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTraceKind {
    BackgroundStart,
    ToolCall,
    BackgroundFinish,
    IdleTick,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorRecallEntry {
    pub fingerprint: String,
    pub first_seen_unix: i64,
    pub last_seen_unix: i64,
    pub occurrences: u32,
    pub path: String,
    pub line: u32,
    pub code: String,
    pub message_stem: String,
}

pub const CARGO_IDLE_THRESHOLD_MS: i64 = 5_000;
pub const GIT_PUSH_IDLE_THRESHOLD_MS: i64 = 30_000;
pub const WEB_SEARCH_IDLE_THRESHOLD_MS: i64 = 60_000;

pub fn agent_trace_path() -> CoreResult<PathBuf> {
    Ok(paths::config_dir()?.join("agent-trace.ndjson"))
}

pub fn error_recall_path() -> CoreResult<PathBuf> {
    Ok(paths::config_dir()?.join("error-recall.json"))
}

pub fn append_trace(entry: &AgentTraceEntry) -> CoreResult<()> {
    let path = agent_trace_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut raw = serde_json::to_string(entry)?;
    raw.push('\n');
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    f.write_all(raw.as_bytes())?;
    Ok(())
}

pub fn read_trace(path: &Path) -> CoreResult<Vec<AgentTraceEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path)?;
    let mut out = Vec::with_capacity(raw.lines().count());
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: AgentTraceEntry = serde_json::from_str(line)
            .map_err(CoreError::Json)?;
        out.push(entry);
    }
    Ok(out)
}

/// Dimension 6 scan — walks a list of trace entries and emits findings
/// for every background-start → next-tool-call gap that exceeds the
/// backend-specific threshold AND contains no intervening background
/// launch.
pub fn dim6_parallel_efficiency(
    trace_path: &Path,
    trace: &[AgentTraceEntry],
) -> Vec<CrownJewelFinding> {
    let mut findings = Vec::new();
    let mut open_background: Option<&AgentTraceEntry> = None;
    for entry in trace {
        match entry.kind {
            AgentTraceKind::BackgroundStart => {
                // Evaluate any previously-open background.
                if let Some(prev) = open_background {
                    let threshold = threshold_for(&prev.detail);
                    let gap = entry.at_unix_ms - prev.at_unix_ms;
                    if gap > threshold {
                        findings.push(make_dim6_finding(trace_path, prev, gap, threshold));
                    }
                }
                open_background = Some(entry);
            }
            AgentTraceKind::ToolCall => {
                if let Some(prev) = open_background {
                    // A foreground ToolCall during a background task is
                    // parallel work — good.  Reset the window start to
                    // "now" so further gaps are measured from here.
                    let gap = entry.at_unix_ms - prev.at_unix_ms;
                    let threshold = threshold_for(&prev.detail);
                    if gap > threshold {
                        findings.push(make_dim6_finding(trace_path, prev, gap, threshold));
                    }
                    open_background = None;
                }
            }
            AgentTraceKind::BackgroundFinish => {
                open_background = None;
            }
            AgentTraceKind::IdleTick => {
                // Presence of an idle tick during an open background is a
                // direct violation regardless of threshold.
                if let Some(prev) = open_background {
                    findings.push(make_dim6_finding(trace_path, prev, 0, 0));
                    open_background = None;
                }
            }
        }
    }
    findings
}

fn threshold_for(background_detail: &str) -> i64 {
    let lower = background_detail.to_lowercase();
    if lower.contains("cargo") || lower.contains("clippy") || lower.contains("test") || lower.contains("pnpm") {
        CARGO_IDLE_THRESHOLD_MS
    } else if lower.contains("git push") {
        GIT_PUSH_IDLE_THRESHOLD_MS
    } else if lower.contains("websearch") || lower.contains("web_search") {
        WEB_SEARCH_IDLE_THRESHOLD_MS
    } else {
        CARGO_IDLE_THRESHOLD_MS
    }
}

fn make_dim6_finding(
    trace_path: &Path,
    prev: &AgentTraceEntry,
    gap_ms: i64,
    threshold: i64,
) -> CrownJewelFinding {
    CrownJewelFinding {
        path: trace_path.to_path_buf(),
        line: 0,
        dimension: Dimension::ParallelEfficiency,
        severity: if gap_ms > threshold * 3 {
            Severity::High
        } else {
            Severity::Medium
        },
        pattern: format!("idle-wait {gap_ms} ms > threshold {threshold} ms"),
        snippet: format!("background: {}", prev.detail),
    }
}

/// Fingerprint a raw cargo / clippy error line.  Stable across runs: the
/// error code + the file basename + the message stem (truncated).
pub fn fingerprint_error(error_code: &str, path: &str, message: &str) -> String {
    let basename = path.rsplit('/').next().unwrap_or(path);
    let stem: String = message
        .chars()
        .filter(|c| !c.is_ascii_digit() && *c != '\'' && *c != '"')
        .take(80)
        .collect();
    format!("{error_code}|{basename}|{stem}")
}

/// Dimension 7 scan — given a fresh error set and the persistent recall
/// store, emit findings for every fingerprint that was seen before.
pub fn dim7_error_recall(
    recall_path: &Path,
    prior: &[ErrorRecallEntry],
    fresh: &[ErrorRecallEntry],
) -> Vec<CrownJewelFinding> {
    let mut findings = Vec::new();
    for f in fresh {
        let was_seen = prior.iter().any(|p| p.fingerprint == f.fingerprint);
        if was_seen {
            findings.push(CrownJewelFinding {
                path: PathBuf::from(&f.path),
                line: f.line as usize,
                dimension: Dimension::ErrorRecall,
                severity: Severity::High,
                pattern: format!("regression: {}", f.code),
                snippet: format!(
                    "previously seen (recall store: {}) — {}",
                    recall_path.display(),
                    f.message_stem
                ),
            });
        }
    }
    findings
}

pub fn load_recall_store(path: &Path) -> CoreResult<Vec<ErrorRecallEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save_recall_store(path: &Path, entries: &[ErrorRecallEntry]) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(entries)?;
    fs::write(path, raw)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn trace_at(kind: AgentTraceKind, at: i64, detail: &str) -> AgentTraceEntry {
        AgentTraceEntry {
            at_unix_ms: at,
            kind,
            detail: detail.to_string(),
        }
    }

    #[test]
    fn dim6_flags_long_idle_wait() {
        let trace = vec![
            trace_at(AgentTraceKind::BackgroundStart, 0, "cargo check"),
            trace_at(AgentTraceKind::IdleTick, 10_000, "sleep"),
        ];
        let findings = dim6_parallel_efficiency(Path::new("/tmp/trace.ndjson"), &trace);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].dimension, Dimension::ParallelEfficiency);
    }

    #[test]
    fn dim6_accepts_fast_next_tool_call() {
        let trace = vec![
            trace_at(AgentTraceKind::BackgroundStart, 0, "cargo check"),
            trace_at(AgentTraceKind::ToolCall, 1_000, "Write file"),
        ];
        let findings = dim6_parallel_efficiency(Path::new("/tmp/trace.ndjson"), &trace);
        assert!(findings.is_empty());
    }

    #[test]
    fn dim6_git_push_has_higher_threshold() {
        let trace = vec![
            trace_at(AgentTraceKind::BackgroundStart, 0, "git push origin main"),
            trace_at(AgentTraceKind::ToolCall, 20_000, "Write file"),
        ];
        let findings = dim6_parallel_efficiency(Path::new("/tmp/trace.ndjson"), &trace);
        assert!(findings.is_empty(), "20s < 30s git-push threshold");
    }

    #[test]
    fn dim7_flags_recurring_fingerprint() {
        let prior = vec![ErrorRecallEntry {
            fingerprint: "E0277|memory.rs|the trait Eq is not satisfied".to_string(),
            first_seen_unix: 1,
            last_seen_unix: 1,
            occurrences: 1,
            path: "crates/impforge-emergence/src/memory.rs".to_string(),
            line: 20,
            code: "E0277".to_string(),
            message_stem: "the trait Eq is not satisfied".to_string(),
        }];
        let fresh = vec![ErrorRecallEntry {
            fingerprint: "E0277|memory.rs|the trait Eq is not satisfied".to_string(),
            first_seen_unix: 2,
            last_seen_unix: 2,
            occurrences: 1,
            path: "crates/impforge-emergence/src/memory.rs".to_string(),
            line: 20,
            code: "E0277".to_string(),
            message_stem: "the trait Eq is not satisfied".to_string(),
        }];
        let findings = dim7_error_recall(Path::new("/tmp/recall.json"), &prior, &fresh);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].dimension, Dimension::ErrorRecall);
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn dim7_new_error_not_flagged() {
        let fresh = vec![ErrorRecallEntry {
            fingerprint: "E9999|new.rs|fresh error".to_string(),
            first_seen_unix: 2,
            last_seen_unix: 2,
            occurrences: 1,
            path: "crates/new.rs".to_string(),
            line: 1,
            code: "E9999".to_string(),
            message_stem: "fresh error".to_string(),
        }];
        let findings = dim7_error_recall(Path::new("/tmp/recall.json"), &[], &fresh);
        assert!(findings.is_empty());
    }

    #[test]
    fn fingerprint_stable_across_runs() {
        let a = fingerprint_error("E0277", "crates/a/src/b.rs", "the trait `Eq` is not satisfied");
        let b = fingerprint_error("E0277", "crates/a/src/b.rs", "the trait `Eq` is not satisfied");
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_strips_line_numbers() {
        let a = fingerprint_error("E0277", "a.rs", "error at line 20 in foo");
        let b = fingerprint_error("E0277", "a.rs", "error at line 99 in foo");
        assert_eq!(a, b, "line numbers should not affect fingerprint");
    }

    #[test]
    fn trace_round_trips_through_disk() {
        let td = TempDir::new().expect("td");
        let path = td.path().join("trace.ndjson");
        let entries = vec![
            trace_at(AgentTraceKind::BackgroundStart, 100, "cargo check"),
            trace_at(AgentTraceKind::ToolCall, 200, "Write file"),
        ];
        fs::write(
            &path,
            entries
                .iter()
                .map(|e| serde_json::to_string(e).expect("ser"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .expect("w");
        let back = read_trace(&path).expect("read");
        assert_eq!(back, entries);
    }

    #[test]
    fn recall_store_round_trips() {
        let td = TempDir::new().expect("td");
        let path = td.path().join("recall.json");
        let entries = vec![ErrorRecallEntry {
            fingerprint: "x".to_string(),
            first_seen_unix: 1,
            last_seen_unix: 2,
            occurrences: 3,
            path: "a.rs".to_string(),
            line: 1,
            code: "E0277".to_string(),
            message_stem: "stem".to_string(),
        }];
        save_recall_store(&path, &entries).expect("save");
        let back = load_recall_store(&path).expect("load");
        assert_eq!(back, entries);
    }
}
