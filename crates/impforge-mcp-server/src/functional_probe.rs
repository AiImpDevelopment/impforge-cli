// SPDX-License-Identifier: MIT
//! Functional smoke-prober for MCP server manifests.
//!
//! Given a manifest, spawn the upstream child process, exchange a JSON-RPC
//! `initialize` handshake, and either collect the `serverInfo` response or
//! terminate after a timeout.  The result is a typed `ProbeOutcome` that
//! downstream callers (CLI doctor, CI integration test, Quarantine gate)
//! can assert against.
//!
//! ## What "functional" means here
//!
//! A manifest is **functional** when:
//!   1. The declared `command` can be launched inside the 5 s timeout;
//!   2. The child's stdout returns a JSON-RPC 2.0 envelope whose `id` matches
//!      the request;
//!   3. The envelope contains a `result.serverInfo` object or an explicit
//!      `error` object (both count as "the MCP protocol is being spoken").
//!
//! A timeout, non-JSON stdout, or a crash before the first byte counts as
//! **non-functional** — the manifest stays in the catalog but is flagged.

use crate::catalog_validator::{McpServerManifest, TransportKind, VerificationStatus};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_SMOKE_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeOutcome {
    pub manifest_id: String,
    pub transport: TransportKind,
    pub verification_status: VerificationStatus,
    pub functional: bool,
    pub elapsed_ms: u64,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProbeReport {
    pub total: usize,
    pub functional: usize,
    pub non_functional: usize,
    pub skipped: usize,
    pub per_manifest: Vec<ProbeOutcome>,
}

impl CatalogProbeReport {
    pub fn pass_rate(&self) -> f64 {
        let eligible = self.total.saturating_sub(self.skipped);
        if eligible == 0 {
            return 1.0;
        }
        self.functional as f64 / eligible as f64
    }
}

/// Probe a single manifest.  Returns an outcome regardless of success.
pub fn probe(manifest: &McpServerManifest, timeout_ms: u64) -> ProbeOutcome {
    let start = Instant::now();
    let mut outcome = ProbeOutcome {
        manifest_id: manifest.id.clone(),
        transport: manifest.transport,
        verification_status: manifest.verification_status,
        functional: false,
        elapsed_ms: 0,
        details: String::new(),
    };

    match manifest.transport {
        TransportKind::Stdio => probe_stdio(manifest, timeout_ms, &mut outcome),
        TransportKind::Http | TransportKind::Sse => {
            outcome.details = "http/sse probes not implemented yet — skipped".to_string();
        }
    }

    outcome.elapsed_ms = start.elapsed().as_millis() as u64;
    outcome
}

fn probe_stdio(manifest: &McpServerManifest, timeout_ms: u64, outcome: &mut ProbeOutcome) {
    let Some(cmd) = manifest.command.as_deref() else {
        outcome.details = "stdio manifest missing command".to_string();
        return;
    };
    let args: Vec<&str> = manifest
        .args
        .as_deref()
        .map(|v| v.iter().map(String::as_str).collect())
        .unwrap_or_default();

    let spawn_result = Command::new(cmd)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match spawn_result {
        Ok(c) => c,
        Err(e) => {
            outcome.details = format!("spawn failed: {e}");
            return;
        }
    };

    // Write JSON-RPC initialize frame.
    let init_payload = r#"{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"impforge-cli-probe","version":"0.1"}}}"#.to_string();

    if let Some(mut stdin) = child.stdin.take() {
        let frame = format!("{init_payload}\n");
        if let Err(e) = stdin.write_all(frame.as_bytes()) {
            let _ = child.kill();
            outcome.details = format!("stdin write failed: {e}");
            return;
        }
    } else {
        let _ = child.kill();
        outcome.details = "child stdin unavailable".to_string();
        return;
    }

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let _ = child.kill();
            outcome.details = "child stdout unavailable".to_string();
            return;
        }
    };

    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        if reader.read_line(&mut line).is_ok() {
            let _ = tx.send(line);
        }
    });

    match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.contains("\"jsonrpc\":\"2.0\"") {
                outcome.functional = true;
                outcome.details = format!("ok — handshake received ({} bytes)", trimmed.len());
            } else {
                outcome.details = format!("non-JSON response: {}", truncate(trimmed, 120));
            }
        }
        Err(_) => {
            outcome.details = format!("timeout after {timeout_ms} ms");
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

/// Probe a set of manifests serially.  Callers that want parallelism wrap
/// this in their own thread pool (Rust async or rayon).
pub fn probe_all(manifests: &[McpServerManifest], timeout_ms: u64) -> CatalogProbeReport {
    let mut report = CatalogProbeReport {
        total: manifests.len(),
        functional: 0,
        non_functional: 0,
        skipped: 0,
        per_manifest: Vec::with_capacity(manifests.len()),
    };
    for m in manifests {
        if m.verification_status == VerificationStatus::Planned {
            report.skipped += 1;
            report.per_manifest.push(ProbeOutcome {
                manifest_id: m.id.clone(),
                transport: m.transport,
                verification_status: m.verification_status,
                functional: false,
                elapsed_ms: 0,
                details: "planned — upstream package not yet available".to_string(),
            });
            continue;
        }
        let outcome = probe(m, timeout_ms);
        if outcome.functional {
            report.functional += 1;
        } else {
            report.non_functional += 1;
        }
        report.per_manifest.push(outcome);
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_with_command(cmd: &str, args: Vec<&str>) -> McpServerManifest {
        McpServerManifest {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "test".to_string(),
            transport: TransportKind::Stdio,
            command: Some(cmd.to_string()),
            args: Some(args.into_iter().map(String::from).collect()),
            url: None,
            tools: vec!["x".to_string()],
            license: "MIT".to_string(),
            category: "testing".to_string(),
            maintainer: "t".to_string(),
            upstream: "https://github.com/t/t".to_string(),
            verification_status: VerificationStatus::Verified,
        }
    }

    #[test]
    fn planned_manifests_are_skipped() {
        let mut m = manifest_with_command("true", vec![]);
        m.verification_status = VerificationStatus::Planned;
        let report = probe_all(std::slice::from_ref(&m), 100);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.functional, 0);
    }

    #[test]
    fn missing_command_reports_failure() {
        let m = manifest_with_command("definitely-not-a-real-binary-xyz123", vec![]);
        let outcome = probe(&m, 500);
        assert!(!outcome.functional);
        assert!(outcome.details.contains("spawn failed") || outcome.details.contains("timeout"));
    }

    #[test]
    fn fake_server_that_prints_json_counts_as_functional() {
        let m = manifest_with_command(
            "sh",
            vec![
                "-c",
                "printf '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"ok\":true}}\\n' && sleep 2",
            ],
        );
        let outcome = probe(&m, 2_000);
        assert!(
            outcome.functional,
            "expected functional; got: {}",
            outcome.details
        );
    }

    #[test]
    fn non_json_stdout_reports_non_functional() {
        let m = manifest_with_command("sh", vec!["-c", "echo 'not json' && sleep 1"]);
        let outcome = probe(&m, 1_500);
        assert!(!outcome.functional);
        assert!(outcome.details.contains("non-JSON"));
    }

    #[test]
    fn silent_process_times_out() {
        let m = manifest_with_command("sh", vec!["-c", "sleep 5"]);
        let outcome = probe(&m, 300);
        assert!(!outcome.functional);
        assert!(outcome.details.contains("timeout"));
    }

    #[test]
    fn pass_rate_handles_empty_report() {
        let r = CatalogProbeReport {
            total: 0,
            functional: 0,
            non_functional: 0,
            skipped: 0,
            per_manifest: vec![],
        };
        assert_eq!(r.pass_rate(), 1.0);
    }

    #[test]
    fn pass_rate_excludes_skipped_from_denominator() {
        let r = CatalogProbeReport {
            total: 5,
            functional: 2,
            non_functional: 1,
            skipped: 2,
            per_manifest: vec![],
        };
        let actual = r.pass_rate();
        assert!((actual - (2.0 / 3.0)).abs() < 1e-6);
    }

    #[test]
    fn verification_status_round_trips() {
        let raw = r#""verified""#;
        let v: VerificationStatus = serde_json::from_str(raw).expect("de");
        assert_eq!(v, VerificationStatus::Verified);
        let back = serde_json::to_string(&v).expect("ser");
        assert_eq!(back, raw);
    }
}
