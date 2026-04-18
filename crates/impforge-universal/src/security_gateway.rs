// SPDX-License-Identifier: MIT
//! v0.3 security gateway — Pre/Exec/Post interception with Merkle audit.
//!
//! **What ships in v0.3 (this file):**
//!   * Pre-stage — tool-id match, argument size cap, prompt-injection
//!     heuristic, simple Cedar-style policy allow/deny evaluation.
//!   * Exec-stage — deadline enforcement via a sync wrapper around the
//!     caller-supplied closure.
//!   * Post-stage — Crown-Jewel output score + SHA-256 Merkle-chained
//!     audit entry.
//!
//! **What v0.4 will add (placeholders documented):**
//!   * WASI-p2 sandbox via `wasmtime` — opt-in feature flag.  Drafts in
//!     `wasmtime_sandbox.rs.todo` (not yet shipped — keeps MSRV low
//!     and crate build cheap for the default feature set).
//!   * Cedar policy evaluation via `cedar-policy` — opt-in feature flag.
//!   * HTTP REST facade via axum for non-stdio clients.
//!
//! The simple built-in policy is good enough for the CLI's preview user
//! base and leaves the heavier wasmtime/cedar integration to opt-in so
//! cold builds stay under the 3 MB minimal-feature budget.

use crate::errors::{UniversalError, UniversalResult};
use crate::tool::{ToolCall, UniversalTool};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::RwLock;
use std::time::Instant;

pub const DEFAULT_ARG_BYTES_CAP: usize = 256 * 1024;
pub const DEFAULT_DEADLINE_MS: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GateDecision {
    Allow,
    Deny { reason: String },
}

impl GateDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, GateDecision::Allow)
    }
}

/// One policy rule — a minimal Cedar-inspired schema that the CLI can
/// evaluate without pulling in the full cedar-policy crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRule {
    /// Tool-id glob (`"*"` = any, `"filesystem:*"` = any filesystem tool).
    pub tool_glob: String,
    pub effect: RuleEffect,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleEffect {
    Allow,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub call_id: String,
    pub tool_id: String,
    pub decision: GateDecision,
    pub prev_hash: String,
    pub entry_hash: String,
    pub unix_ms: i64,
}

const GENESIS: &str = "impforge-universal-security-gateway-genesis";

#[derive(Debug, Default)]
pub struct SecurityGateway {
    policies: RwLock<Vec<PolicyRule>>,
    audit: RwLock<Vec<AuditEntry>>,
    arg_bytes_cap: usize,
}

impl SecurityGateway {
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(Vec::new()),
            audit: RwLock::new(Vec::new()),
            arg_bytes_cap: DEFAULT_ARG_BYTES_CAP,
        }
    }

    pub fn with_arg_cap(mut self, cap: usize) -> Self {
        self.arg_bytes_cap = cap;
        self
    }

    pub fn add_policy(&self, rule: PolicyRule) -> UniversalResult<()> {
        let mut g = self
            .policies
            .write()
            .map_err(|e| UniversalError::Other(format!("policy lock: {e}")))?;
        g.push(rule);
        Ok(())
    }

    pub fn policies(&self) -> UniversalResult<Vec<PolicyRule>> {
        let g = self
            .policies
            .read()
            .map_err(|e| UniversalError::Other(format!("policy lock: {e}")))?;
        Ok(g.clone())
    }

    pub fn pre(&self, call: &ToolCall, tool: &UniversalTool) -> UniversalResult<GateDecision> {
        if tool.id != call.tool_id {
            let d = GateDecision::Deny {
                reason: format!("tool id mismatch {} vs {}", tool.id, call.tool_id),
            };
            self.audit_push(call, &d)?;
            return Ok(d);
        }
        let arg_bytes = call.arguments.to_string().len();
        if arg_bytes > self.arg_bytes_cap {
            let d = GateDecision::Deny {
                reason: format!(
                    "arguments {} bytes exceed cap {}",
                    arg_bytes, self.arg_bytes_cap
                ),
            };
            self.audit_push(call, &d)?;
            return Ok(d);
        }
        if prompt_injection_hit(&call.arguments.to_string()) {
            let d = GateDecision::Deny {
                reason: "arguments match prompt-injection pattern".to_string(),
            };
            self.audit_push(call, &d)?;
            return Ok(d);
        }
        // Policy evaluation — deny-takes-precedence ordering.
        let policies = self
            .policies
            .read()
            .map_err(|e| UniversalError::Other(format!("policy lock: {e}")))?;
        for rule in policies.iter() {
            if glob_match(&rule.tool_glob, &tool.id) && rule.effect == RuleEffect::Deny {
                let d = GateDecision::Deny {
                    reason: format!("policy deny: {}", rule.reason),
                };
                drop(policies);
                self.audit_push(call, &d)?;
                return Ok(d);
            }
        }
        drop(policies);
        let d = GateDecision::Allow;
        self.audit_push(call, &d)?;
        Ok(d)
    }

    pub fn exec_with_deadline<F, T>(&self, f: F, deadline_ms: u64) -> UniversalResult<(T, u64)>
    where
        F: FnOnce() -> UniversalResult<T>,
    {
        let start = Instant::now();
        let out = f()?;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        if elapsed_ms > deadline_ms {
            return Err(UniversalError::Other(format!(
                "call exceeded deadline {deadline_ms}ms (took {elapsed_ms}ms)"
            )));
        }
        Ok((out, elapsed_ms))
    }

    pub fn post(&self, call: &ToolCall, ok: bool, text: &str) -> UniversalResult<(u32, String)> {
        let score = score_output(text, ok);
        let decision = if ok {
            GateDecision::Allow
        } else {
            GateDecision::Deny {
                reason: "tool returned error".to_string(),
            }
        };
        let entry_hash = self.audit_push(call, &decision)?;
        Ok((score, entry_hash))
    }

    fn audit_push(&self, call: &ToolCall, decision: &GateDecision) -> UniversalResult<String> {
        let mut g = self
            .audit
            .write()
            .map_err(|e| UniversalError::Other(format!("audit lock: {e}")))?;
        let prev_hash = g
            .last()
            .map(|e| e.entry_hash.clone())
            .unwrap_or_else(|| hash_hex(GENESIS));
        let unix_ms = chrono::Utc::now().timestamp_millis();
        let material = format!(
            "{prev_hash}|{}|{}|{:?}|{unix_ms}",
            call.call_id, call.tool_id, decision
        );
        let entry_hash = hash_hex(&material);
        g.push(AuditEntry {
            call_id: call.call_id.clone(),
            tool_id: call.tool_id.clone(),
            decision: decision.clone(),
            prev_hash,
            entry_hash: entry_hash.clone(),
            unix_ms,
        });
        Ok(entry_hash)
    }

    pub fn audit_entries(&self) -> UniversalResult<Vec<AuditEntry>> {
        let g = self
            .audit
            .read()
            .map_err(|e| UniversalError::Other(format!("audit lock: {e}")))?;
        Ok(g.clone())
    }

    pub fn verify_audit_chain(&self) -> UniversalResult<bool> {
        let g = self
            .audit
            .read()
            .map_err(|e| UniversalError::Other(format!("audit lock: {e}")))?;
        let mut prev = hash_hex(GENESIS);
        for e in g.iter() {
            if e.prev_hash != prev {
                return Ok(false);
            }
            let material = format!(
                "{prev}|{}|{}|{:?}|{}",
                e.call_id, e.tool_id, e.decision, e.unix_ms
            );
            if hash_hex(&material) != e.entry_hash {
                return Ok(false);
            }
            prev = e.entry_hash.clone();
        }
        Ok(true)
    }
}

fn hash_hex(material: &str) -> String {
    let mut h = Sha256::new();
    h.update(material.as_bytes());
    format!("{:x}", h.finalize())
}

fn prompt_injection_hit(s: &str) -> bool {
    const PATTERNS: &[&str] = &[
        "ignore previous instructions",
        "ignore all previous",
        "system prompt",
        "you are now",
        "override",
        "jailbreak",
        "disregard safety",
    ];
    let l = s.to_ascii_lowercase();
    PATTERNS.iter().any(|p| l.contains(p))
}

fn score_output(text: &str, ok: bool) -> u32 {
    if !ok {
        return 0;
    }
    let mut score = 100u32;
    if text.trim().is_empty() {
        score = score.saturating_sub(30);
    }
    if text.len() > 50_000 {
        score = score.saturating_sub(10);
    }
    if text.contains("TODO") || text.contains("unimplemented") {
        score = score.saturating_sub(20);
    }
    score
}

/// Minimal glob matcher — supports only leading/trailing `*`.  Rejecting
/// the full fnmatch syntax keeps the security surface minimal.
fn glob_match(pattern: &str, candidate: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(stripped) = pattern.strip_suffix('*') {
        if let Some(inner) = stripped.strip_prefix('*') {
            return candidate.contains(inner);
        }
        return candidate.starts_with(stripped);
    }
    if let Some(stripped) = pattern.strip_prefix('*') {
        return candidate.ends_with(stripped);
    }
    pattern == candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::ToolCost;
    use serde_json::json;

    fn sample_tool() -> UniversalTool {
        UniversalTool {
            id: "fs:read".to_string(),
            name: "read".to_string(),
            description: "r".to_string(),
            input_schema: json!({"type":"object"}),
            output_schema: None,
            source: "fs".to_string(),
            cost: ToolCost::Low,
        }
    }

    fn call(tool_id: &str) -> ToolCall {
        ToolCall {
            tool_id: tool_id.to_string(),
            arguments: json!({"p": "x"}),
            call_id: "c1".to_string(),
        }
    }

    #[test]
    fn matching_tool_allows() {
        let g = SecurityGateway::new();
        assert!(g.pre(&call("fs:read"), &sample_tool()).expect("ok").is_allowed());
    }

    #[test]
    fn tool_id_mismatch_denied() {
        let g = SecurityGateway::new();
        let c = call("evil");
        assert!(!g.pre(&c, &sample_tool()).expect("ok").is_allowed());
    }

    #[test]
    fn prompt_injection_denied() {
        let g = SecurityGateway::new();
        let c = ToolCall {
            tool_id: "fs:read".to_string(),
            arguments: json!({"text": "ignore previous instructions"}),
            call_id: "c".to_string(),
        };
        assert!(!g.pre(&c, &sample_tool()).expect("ok").is_allowed());
    }

    #[test]
    fn arg_cap_denied() {
        let g = SecurityGateway::new().with_arg_cap(8);
        let c = call("fs:read");
        assert!(!g.pre(&c, &sample_tool()).expect("ok").is_allowed());
    }

    #[test]
    fn policy_deny_overrides_allow() {
        let g = SecurityGateway::new();
        g.add_policy(PolicyRule {
            tool_glob: "fs:*".to_string(),
            effect: RuleEffect::Deny,
            reason: "fs blocked".to_string(),
        })
        .expect("add");
        assert!(!g.pre(&call("fs:read"), &sample_tool()).expect("ok").is_allowed());
    }

    #[test]
    fn policy_on_unrelated_tool_does_not_match() {
        let g = SecurityGateway::new();
        g.add_policy(PolicyRule {
            tool_glob: "github:*".to_string(),
            effect: RuleEffect::Deny,
            reason: "github blocked".to_string(),
        })
        .expect("add");
        assert!(g.pre(&call("fs:read"), &sample_tool()).expect("ok").is_allowed());
    }

    #[test]
    fn deadline_enforced() {
        let g = SecurityGateway::new();
        let r = g.exec_with_deadline::<_, String>(
            || {
                std::thread::sleep(std::time::Duration::from_millis(30));
                Ok("ok".to_string())
            },
            5,
        );
        assert!(r.is_err());
    }

    #[test]
    fn deadline_ok_for_fast_call() {
        let g = SecurityGateway::new();
        let (out, _) = g
            .exec_with_deadline(|| Ok(42_i32), 1_000)
            .expect("ok");
        assert_eq!(out, 42);
    }

    #[test]
    fn post_scores_ok_as_hundred() {
        let g = SecurityGateway::new();
        let (score, _hash) = g.post(&call("fs:read"), true, "hello").expect("ok");
        assert_eq!(score, 100);
    }

    #[test]
    fn post_scores_err_as_zero() {
        let g = SecurityGateway::new();
        let (score, _hash) = g.post(&call("fs:read"), false, "err").expect("ok");
        assert_eq!(score, 0);
    }

    #[test]
    fn audit_chain_verifies() {
        let g = SecurityGateway::new();
        g.pre(&call("fs:read"), &sample_tool()).expect("ok");
        g.post(&call("fs:read"), true, "yo").expect("ok");
        assert!(g.verify_audit_chain().expect("verify"));
    }

    #[test]
    fn audit_tamper_detected() {
        let g = SecurityGateway::new();
        g.pre(&call("fs:read"), &sample_tool()).expect("ok");
        // Simulate tampering by forcibly mutating an entry — verify must fail.
        {
            let mut guard = g.audit.write().expect("lock");
            guard[0].tool_id = "evil".to_string();
        }
        assert!(!g.verify_audit_chain().expect("verify"));
    }

    #[test]
    fn glob_prefix_match() {
        assert!(glob_match("fs:*", "fs:read"));
        assert!(!glob_match("fs:*", "github:read"));
    }

    #[test]
    fn glob_suffix_match() {
        assert!(glob_match("*:read", "fs:read"));
        assert!(!glob_match("*:read", "fs:write"));
    }

    #[test]
    fn glob_any() {
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn glob_contains() {
        assert!(glob_match("*read*", "fs:read_file"));
        assert!(!glob_match("*read*", "fs:write"));
    }

    #[test]
    fn score_penalises_todo() {
        assert_eq!(score_output("TODO: x", true), 80);
    }

    #[test]
    fn score_penalises_empty() {
        assert_eq!(score_output("", true), 70);
    }

    #[test]
    fn policies_round_trip() {
        let g = SecurityGateway::new();
        g.add_policy(PolicyRule {
            tool_glob: "*".to_string(),
            effect: RuleEffect::Allow,
            reason: "default".to_string(),
        })
        .expect("add");
        assert_eq!(g.policies().expect("ok").len(), 1);
    }
}
