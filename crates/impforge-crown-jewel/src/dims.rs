// SPDX-License-Identifier: MIT
//! Per-dimension line-level detection.  Every function here takes a
//! single line of source code and returns a list of findings.

use crate::report::{CrownJewelFinding, Dimension, Severity};
use std::path::Path;

pub fn scan_line(
    path: &Path,
    line_number: usize,
    line: &str,
    in_test_block: bool,
) -> Vec<CrownJewelFinding> {
    let mut out = Vec::new();
    if in_test_block {
        return out;
    }
    out.extend(dim1_stubs(path, line_number, line));
    out.extend(dim2_suppression(path, line_number, line));
    out.extend(dim3_lonely_unwrap(path, line_number, line));
    out
}

/// Dimension 1 — No stubs / placeholders.
pub fn dim1_stubs(
    path: &Path,
    line_number: usize,
    line: &str,
) -> Vec<CrownJewelFinding> {
    struct Pat {
        needle: &'static str,
        severity: Severity,
    }
    const PATTERNS: &[Pat] = &[
        Pat { needle: "stub OK", severity: Severity::High },
        Pat { needle: "stub ok", severity: Severity::High },
        Pat { needle: "next iteration", severity: Severity::High },
        Pat { needle: "not yet implemented", severity: Severity::High },
        Pat { needle: "// TODO", severity: Severity::Medium },
        Pat { needle: "// FIXME", severity: Severity::Medium },
        Pat { needle: "unimplemented!(", severity: Severity::Critical },
        Pat { needle: "todo!(", severity: Severity::Critical },
        Pat { needle: "will be wired", severity: Severity::High },
        Pat { needle: "will land in", severity: Severity::High },
        Pat { needle: "deferred to next", severity: Severity::High },
    ];

    let mut out = Vec::new();
    for p in PATTERNS {
        if line.contains(p.needle) {
            out.push(CrownJewelFinding {
                path: path.to_path_buf(),
                line: line_number,
                dimension: Dimension::NoStubs,
                severity: p.severity,
                pattern: p.needle.to_string(),
                snippet: line.trim().to_string(),
            });
        }
    }
    out
}

/// Dimension 2 — No suppression attributes.
pub fn dim2_suppression(
    path: &Path,
    line_number: usize,
    line: &str,
) -> Vec<CrownJewelFinding> {
    let trimmed = line.trim_start();
    let patterns: &[(&str, Severity)] = &[
        ("#[allow(", Severity::High),
        ("#![allow(", Severity::High),
        ("#[alloy(", Severity::Critical),
        ("#[allo(", Severity::Critical),
        ("clippy::allow", Severity::Medium),
        ("#[rustfmt::skip", Severity::Medium),
        ("#![rustfmt::skip", Severity::Medium),
        ("// deny(", Severity::Info),
    ];
    let mut out = Vec::new();
    for (needle, sev) in patterns {
        if trimmed.starts_with(needle) || line.contains(needle) {
            out.push(CrownJewelFinding {
                path: path.to_path_buf(),
                line: line_number,
                dimension: Dimension::NoSuppression,
                severity: *sev,
                pattern: (*needle).to_string(),
                snippet: line.trim().to_string(),
            });
        }
    }
    out
}

/// Dimension 3 — No lonely `.unwrap()`.  Flags `.unwrap()` as Medium (the
/// test-coverage check happens at file scope in `scanner::scan`).
pub fn dim3_lonely_unwrap(
    path: &Path,
    line_number: usize,
    line: &str,
) -> Vec<CrownJewelFinding> {
    if line.contains(".unwrap()") {
        return vec![CrownJewelFinding {
            path: path.to_path_buf(),
            line: line_number,
            dimension: Dimension::NoLonelyUnwrap,
            severity: Severity::Medium,
            pattern: ".unwrap()".to_string(),
            snippet: line.trim().to_string(),
        }];
    }
    Vec::new()
}

/// Dimension 4 — Test-first.  Given the parsed `pub fn` names in a file
/// and the test names, returns a finding for each `pub fn` that has no
/// matching test.
pub fn dim4_test_first(
    path: &Path,
    pub_items: &[String],
    test_names: &[String],
) -> Vec<CrownJewelFinding> {
    let mut out = Vec::new();
    for item in pub_items {
        let needle_lower = item.to_lowercase();
        let covered = test_names
            .iter()
            .any(|t| t.to_lowercase().contains(&needle_lower));
        if !covered {
            out.push(CrownJewelFinding {
                path: path.to_path_buf(),
                line: 1,
                dimension: Dimension::TestFirst,
                severity: Severity::Medium,
                pattern: format!("pub item '{item}' has no matching #[test]"),
                snippet: item.to_string(),
            });
        }
    }
    out
}

/// Dimension 5 — Crown-Jewel wiring.  Given the set of workspace members
/// and the set of modules registered in `bootstrap_orchestrator()`, flag
/// any workspace member that's missing.
pub fn dim5_crown_jewel_wiring(
    path: &Path,
    workspace_members: &[String],
    registered_modules: &[String],
) -> Vec<CrownJewelFinding> {
    let mut out = Vec::new();
    for member in workspace_members {
        // Non-module crates (cli binary, updater, core, emergence, crown-jewel)
        // are orchestrator-drivers, not orchestrator-participants.
        if matches!(
            member.as_str(),
            "impforge-cli"
                | "impforge-core"
                | "impforge-emergence"
                | "impforge-aiimp-updater"
                | "impforge-crown-jewel"
        ) {
            continue;
        }
        if !registered_modules.iter().any(|r| r == member) {
            out.push(CrownJewelFinding {
                path: path.to_path_buf(),
                line: 1,
                dimension: Dimension::CrownJewelWiring,
                severity: Severity::High,
                pattern: format!("crate '{member}' not registered in orchestrator"),
                snippet: format!("missing: orc.register(Arc::new({member}::Module_))?;"),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> &'static Path {
        Path::new("/tmp/x.rs")
    }

    #[test]
    fn dim1_stub_ok_flagged_high() {
        let f = dim1_stubs(p(), 1, "println!(\"stub OK next\");");
        assert!(!f.is_empty());
        assert_eq!(f[0].severity, Severity::High);
    }

    #[test]
    fn dim1_unimplemented_flagged_critical() {
        let f = dim1_stubs(p(), 1, "fn foo() { unimplemented!(); }");
        assert!(f.iter().any(|x| x.severity == Severity::Critical));
    }

    #[test]
    fn dim2_allow_flagged_high() {
        let f = dim2_suppression(p(), 1, "#[allow(dead_code)]");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::High);
    }

    #[test]
    fn dim2_alloy_typo_flagged_critical() {
        let f = dim2_suppression(p(), 1, "#[alloy(unused)]");
        assert_eq!(f[0].severity, Severity::Critical);
    }

    #[test]
    fn dim3_unwrap_flagged_medium() {
        let f = dim3_lonely_unwrap(p(), 1, "let x = foo.unwrap();");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Medium);
    }

    #[test]
    fn dim4_missing_test_flagged() {
        let f = dim4_test_first(
            p(),
            &["compute_thing".to_string()],
            &["test_unrelated".to_string()],
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].dimension, Dimension::TestFirst);
    }

    #[test]
    fn dim4_covered_pub_fn_passes() {
        let f = dim4_test_first(
            p(),
            &["compute_thing".to_string()],
            &["compute_thing_works".to_string()],
        );
        assert!(f.is_empty());
    }

    #[test]
    fn dim5_unregistered_crate_flagged() {
        let f = dim5_crown_jewel_wiring(
            p(),
            &["impforge-scaffold".to_string(), "impforge-new-thing".to_string()],
            &["impforge-scaffold".to_string()],
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::High);
    }

    #[test]
    fn dim5_driver_crates_excluded() {
        let f = dim5_crown_jewel_wiring(
            p(),
            &["impforge-cli".to_string(), "impforge-core".to_string()],
            &[],
        );
        assert!(f.is_empty());
    }

    #[test]
    fn scan_line_skips_tests() {
        let r = scan_line(p(), 1, "fn foo() { unimplemented!(); }", true);
        assert!(r.is_empty());
    }
}
