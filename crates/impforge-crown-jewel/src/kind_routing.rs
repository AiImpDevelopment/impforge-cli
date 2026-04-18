// SPDX-License-Identifier: MIT
//! Dimension 8 — Kind-based Routing.
//!
//! Every dispatcher that accepts typed messages (messenger hub, event bus,
//! pipeline router, pub/sub broker) MUST consult the message's kind tag
//! before choosing a recipient.  Blind fan-out of a `MessageKind::UpgradeCard`
//! to every transport is a privacy + UX violation (rich content gets
//! text-truncated on transports that do not support it, or leaks formatting
//! to transports the subscriber never opted into).
//!
//! This dimension flags:
//!   * functions named `publish` / `dispatch` / `send` / `route` / `fan_out`
//!     that iterate over a transport/subscriber collection without referencing
//!     a `kind` / `MessageKind` / `Direction` / `message_type` field in the loop
//!   * `broadcast_to_all` / `send_to_everyone` helpers
//!   * `for transport in ...` / `for subscriber in ...` loops whose body
//!     contains `.send(` but no `match` / `if let` on a kind field
//!
//! The detector is intentionally conservative: it only fires when the loop
//! is clearly a dispatcher (body contains `.send(` OR `.publish(` OR
//! `.deliver(` OR `.dispatch(`) AND no kind keyword appears within +/-3
//! lines of the call site.

use crate::report::{CrownJewelFinding, Dimension, Severity};
use std::path::Path;

const DISPATCH_FN_NAMES: &[&str] = &[
    "publish",
    "dispatch",
    "send",
    "route",
    "fan_out",
    "broadcast",
    "fanout",
    "deliver",
];

const KIND_KEYWORDS: &[&str] = &[
    "MessageKind",
    "kind",
    "Direction",
    "message_type",
    "msg_type",
    "event_type",
    "MsgKind",
    "EventKind",
    ".kind",
];

const LOOP_TARGET_HINTS: &[&str] = &[
    "transport",
    "subscriber",
    "recipient",
    "listener",
    "channel",
    "endpoint",
    "peer",
];

const DISPATCH_CALL_HINTS: &[&str] = &[
    ".send(",
    ".publish(",
    ".deliver(",
    ".dispatch(",
    ".fan_out(",
    ".emit(",
];

/// Scan a Rust file for dim-8 violations.  Returns findings sorted by line.
pub fn scan_rust_file(path: &Path, content: &str) -> Vec<CrownJewelFinding> {
    let lines: Vec<&str> = content.lines().collect();
    let mut findings = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if !is_dispatcher_loop_header(line) {
            continue;
        }
        let window_start = i.saturating_sub(3);
        let window_end = (i + 12).min(lines.len());
        let window = &lines[window_start..window_end];
        let body_text: String = window.join("\n");

        if !body_contains_dispatch_call(&body_text) {
            continue;
        }
        if body_references_kind(&body_text) {
            continue;
        }

        findings.push(CrownJewelFinding {
            path: path.to_path_buf(),
            line: i + 1,
            dimension: Dimension::KindRouting,
            severity: Severity::High,
            pattern: "blind_dispatch_loop".to_string(),
            snippet: line.trim().to_string(),
        });
    }

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let Some(after_fn) = trimmed.strip_prefix("fn ").or_else(|| trimmed.strip_prefix("pub fn "))
            .or_else(|| trimmed.strip_prefix("pub(crate) fn "))
            .or_else(|| trimmed.strip_prefix("async fn "))
            .or_else(|| trimmed.strip_prefix("pub async fn "))
        else {
            continue;
        };
        let fn_name_token = after_fn.split(['(', '<', ' ']).next().unwrap_or("");
        if !DISPATCH_FN_NAMES.iter().any(|n| fn_name_token == *n) {
            continue;
        }
        let body_end = (i + 25).min(lines.len());
        let body_text: String = lines[i..body_end].join("\n");
        if !body_contains_dispatch_call(&body_text) {
            continue;
        }
        if body_references_kind(&body_text) {
            continue;
        }
        findings.push(CrownJewelFinding {
            path: path.to_path_buf(),
            line: i + 1,
            dimension: Dimension::KindRouting,
            severity: Severity::High,
            pattern: format!("dispatcher_fn_{fn_name_token}_without_kind_check"),
            snippet: trimmed.to_string(),
        });
    }

    findings.sort_by_key(|f| f.line);
    findings.dedup_by_key(|f| f.line);
    // When a dispatcher-named function AND a blind loop inside it both fire,
    // the loop is the more-specific finding — drop the outer function-level
    // finding if a loop-level one exists within +25 lines.
    let line_pattern: Vec<(usize, String)> = findings
        .iter()
        .map(|f| (f.line, f.pattern.clone()))
        .collect();
    findings.retain(|f| {
        if !f.pattern.starts_with("dispatcher_fn_") {
            return true;
        }
        let window_end = f.line + 25;
        !line_pattern
            .iter()
            .any(|(ln, p)| p == "blind_dispatch_loop" && *ln > f.line && *ln <= window_end)
    });
    findings
}

fn is_dispatcher_loop_header(line: &str) -> bool {
    let t = line.trim();
    if !t.starts_with("for ") && !t.contains(".iter()") && !t.contains(".into_iter()") {
        return false;
    }
    LOOP_TARGET_HINTS.iter().any(|h| t.to_ascii_lowercase().contains(h))
}

fn body_contains_dispatch_call(body: &str) -> bool {
    DISPATCH_CALL_HINTS.iter().any(|c| body.contains(c))
}

fn body_references_kind(body: &str) -> bool {
    KIND_KEYWORDS.iter().any(|k| body.contains(k))
        || body.contains("supports_kind")
        || body.contains("match ")
            && (body.contains("kind") || body.contains("MessageKind"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_blind_transport_fan_out() {
        let src = r#"
fn publish(&self, msg: &Message) {
    for transport in self.transports.iter() {
        transport.send(msg);
    }
}
"#;
        let findings = scan_rust_file(&PathBuf::from("test.rs"), src);
        assert!(!findings.is_empty(), "expected blind-fan-out finding");
        assert_eq!(findings[0].dimension, Dimension::KindRouting);
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn allows_kind_filtered_loop() {
        let src = r#"
fn publish(&self, msg: &Message) {
    for transport in self.transports.iter() {
        if transport.supports_kind(msg.kind) {
            transport.send(msg);
        }
    }
}
"#;
        let findings = scan_rust_file(&PathBuf::from("test.rs"), src);
        assert!(findings.is_empty(), "kind-aware dispatcher must pass");
    }

    #[test]
    fn allows_match_on_kind() {
        let src = r#"
fn route(&self, msg: &Message) {
    match msg.kind {
        MessageKind::RichReply => self.rich_transports.iter().for_each(|t| t.send(msg)),
        _ => self.default_transport.send(msg),
    }
}
"#;
        let findings = scan_rust_file(&PathBuf::from("test.rs"), src);
        assert!(findings.is_empty(), "match on kind must pass");
    }

    #[test]
    fn flags_broadcast_without_kind() {
        let src = r#"
pub fn broadcast(&self, text: String) {
    for subscriber in self.subscribers.iter() {
        subscriber.send(text.clone());
    }
}
"#;
        let findings = scan_rust_file(&PathBuf::from("t.rs"), src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 3);
    }

    #[test]
    fn ignores_non_dispatcher_loops() {
        let src = r#"
fn sum(values: &[u32]) -> u32 {
    let mut total = 0;
    for v in values.iter() {
        total += v;
    }
    total
}
"#;
        let findings = scan_rust_file(&PathBuf::from("t.rs"), src);
        assert!(findings.is_empty());
    }

    #[test]
    fn findings_are_sorted_and_deduped() {
        let src = r#"
pub fn publish(&self, msg: &Message) {
    for transport in self.transports.iter() {
        transport.send(msg);
    }
    for subscriber in self.subs.iter() {
        subscriber.send(msg);
    }
}
"#;
        let findings = scan_rust_file(&PathBuf::from("t.rs"), src);
        assert!(findings.len() >= 2, "both loops should fire");
        for w in findings.windows(2) {
            assert!(w[0].line < w[1].line, "findings must be sorted");
        }
    }
}
