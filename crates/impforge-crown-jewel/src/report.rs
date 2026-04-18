// SPDX-License-Identifier: MIT
//! Report types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    NoStubs,
    NoSuppression,
    NoLonelyUnwrap,
    TestFirst,
    CrownJewelWiring,
    ParallelEfficiency,
    ErrorRecall,
    /// Dim 8 — every typed-message dispatcher (hub / bus / router) must
    /// inspect the message's `kind` / `MessageKind` / `Direction` before
    /// choosing a recipient.  Blind `for transport in all_transports` fan-out
    /// is a Crown-Jewel violation.
    KindRouting,
}

impl Dimension {
    pub fn as_str(self) -> &'static str {
        match self {
            Dimension::NoStubs => "no_stubs",
            Dimension::NoSuppression => "no_suppression",
            Dimension::NoLonelyUnwrap => "no_lonely_unwrap",
            Dimension::TestFirst => "test_first",
            Dimension::CrownJewelWiring => "crown_jewel_wiring",
            Dimension::ParallelEfficiency => "parallel_efficiency",
            Dimension::ErrorRecall => "error_recall",
            Dimension::KindRouting => "kind_routing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn is_blocking(self) -> bool {
        matches!(self, Severity::High | Severity::Critical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrownJewelFinding {
    pub path: PathBuf,
    pub line: usize,
    pub dimension: Dimension,
    pub severity: Severity,
    pub pattern: String,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrownJewelReport {
    pub root: PathBuf,
    pub files_scanned: usize,
    pub findings: Vec<CrownJewelFinding>,
    pub dimension_totals: DimensionTotals,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionTotals {
    pub no_stubs: usize,
    pub no_suppression: usize,
    pub no_lonely_unwrap: usize,
    pub test_first: usize,
    pub crown_jewel_wiring: usize,
    pub parallel_efficiency: usize,
    pub error_recall: usize,
    pub kind_routing: usize,
}

impl CrownJewelReport {
    pub fn is_clean(&self) -> bool {
        !self.findings.iter().any(|f| f.severity.is_blocking())
    }

    pub fn blocking_count(&self) -> usize {
        self.findings.iter().filter(|f| f.severity.is_blocking()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_is_blocking() {
        assert!(Severity::Critical.is_blocking());
        assert!(Severity::High.is_blocking());
        assert!(!Severity::Medium.is_blocking());
    }

    #[test]
    fn dimension_as_str_stable() {
        assert_eq!(Dimension::NoStubs.as_str(), "no_stubs");
        assert_eq!(Dimension::NoSuppression.as_str(), "no_suppression");
        assert_eq!(Dimension::NoLonelyUnwrap.as_str(), "no_lonely_unwrap");
    }

    #[test]
    fn empty_report_is_clean() {
        let r = CrownJewelReport {
            root: PathBuf::from("/tmp"),
            files_scanned: 0,
            findings: vec![],
            dimension_totals: DimensionTotals::default(),
        };
        assert!(r.is_clean());
        assert_eq!(r.blocking_count(), 0);
    }

    #[test]
    fn report_with_critical_is_not_clean() {
        let r = CrownJewelReport {
            root: PathBuf::from("/tmp"),
            files_scanned: 1,
            findings: vec![CrownJewelFinding {
                path: PathBuf::from("/tmp/a.rs"),
                line: 1,
                dimension: Dimension::NoStubs,
                severity: Severity::Critical,
                pattern: "unimplemented!(".to_string(),
                snippet: "fn foo() { unimplemented!() }".to_string(),
            }],
            dimension_totals: DimensionTotals { no_stubs: 1, ..Default::default() },
        };
        assert!(!r.is_clean());
        assert_eq!(r.blocking_count(), 1);
    }

    #[test]
    fn report_serializes_roundtrip() {
        let r = CrownJewelReport {
            root: PathBuf::from("/tmp"),
            files_scanned: 0,
            findings: vec![],
            dimension_totals: DimensionTotals::default(),
        };
        let j = serde_json::to_string(&r).expect("serialize");
        let back: CrownJewelReport = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(r, back);
    }
}
