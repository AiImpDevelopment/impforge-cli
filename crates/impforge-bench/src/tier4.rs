// SPDX-License-Identifier: MIT
//! Tier 4 — behavioural benchmarks that measure the agent, not the model.
//!
//! Dim 6 (parallel-work efficiency) and Dim 7 (error recall) from the
//! Crown-Jewel Guardian both map cleanly into a benchmark: give the agent
//! a sequence of long-running tasks + a deliberate error from a previous
//! run, measure whether it idle-waits and whether it fixes the known
//! regression.

use crate::report::BenchCase;

pub fn behavioral_cases() -> Vec<BenchCase> {
    vec![
        BenchCase {
            id: "tier4-par-01".to_string(),
            tier: 4,
            category: "parallel_efficiency".to_string(),
            prompt: "Kick off `cargo check --workspace` in the background.  While it runs, write a new file called `foo.rs` with a hello-world function.  Report the wall-clock time between the two operations.".to_string(),
            expected_signal: "foo.rs".to_string(),
        },
        BenchCase {
            id: "tier4-par-02".to_string(),
            tier: 4,
            category: "parallel_efficiency".to_string(),
            prompt: "Start `git push origin main` in the background.  Immediately after, refresh the README by adding one bullet.  Do not poll the push output until after the README write completes.".to_string(),
            expected_signal: "README".to_string(),
        },
        BenchCase {
            id: "tier4-par-03".to_string(),
            tier: 4,
            category: "parallel_efficiency".to_string(),
            prompt: "Trigger a WebSearch for `Rust 1.95 release notes`.  While it runs, draft a Cargo.toml snippet that sets `rust-version = \"1.95\"`.  Do not wait for the search to return before writing the snippet.".to_string(),
            expected_signal: "rust-version".to_string(),
        },
        BenchCase {
            id: "tier4-recall-01".to_string(),
            tier: 4,
            category: "error_recall".to_string(),
            prompt: "The following error just appeared: `error[E0277]: the trait bound `f32: Eq` is not satisfied`.  You saw this error 3 commits ago.  What structural fix should be applied so the `Eq` derive can be removed safely, and what regression test do you add?".to_string(),
            expected_signal: "Eq".to_string(),
        },
        BenchCase {
            id: "tier4-recall-02".to_string(),
            tier: 4,
            category: "error_recall".to_string(),
            prompt: "The compiler warns `unused import: `HealthState``.  You saw this in three earlier runs.  Instead of reapplying the same fix, name the refactoring that prevents the import drift long-term.".to_string(),
            expected_signal: "HealthState".to_string(),
        },
    ]
}

pub fn grade_response(case: &BenchCase, response: &str) -> bool {
    if case.expected_signal.is_empty() {
        return false;
    }
    response.to_lowercase().contains(&case.expected_signal.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavioral_has_five_cases() {
        assert_eq!(behavioral_cases().len(), 5);
    }

    #[test]
    fn every_case_is_tier4() {
        for c in behavioral_cases() {
            assert_eq!(c.tier, 4);
        }
    }

    #[test]
    fn parallel_cases_present() {
        let parallel = behavioral_cases()
            .into_iter()
            .filter(|c| c.category == "parallel_efficiency")
            .count();
        assert!(parallel >= 3);
    }

    #[test]
    fn recall_cases_present() {
        let recall = behavioral_cases()
            .into_iter()
            .filter(|c| c.category == "error_recall")
            .count();
        assert!(recall >= 2);
    }

    #[test]
    fn grade_matches_signal() {
        let cases = behavioral_cases();
        let c = &cases[0];
        assert!(grade_response(c, "I wrote foo.rs in parallel"));
        assert!(!grade_response(c, "I wrote something else"));
    }
}
