// SPDX-License-Identifier: MIT
//! End-to-end test: every shipped MCP manifest must pass the Crown-Jewel
//! catalog validator.  A single dirty manifest fails CI.

use impforge_mcp_server::catalog_validator::validate_catalog;
use std::path::PathBuf;

fn find_catalog_root() -> PathBuf {
    let env = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("cwd"));
    let mut probe = env.clone();
    for _ in 0..6 {
        let candidate = probe.join("mcp-manifests").join("servers");
        if candidate.is_dir() {
            return candidate;
        }
        if !probe.pop() {
            break;
        }
    }
    panic!("could not locate mcp-manifests/servers/ starting from {}", env.display());
}

#[test]
fn every_shipped_manifest_is_crown_jewel() {
    let root = find_catalog_root();
    let report = validate_catalog(&root).expect("validate catalog");
    if !report.is_clean() {
        for r in report.per_manifest.iter().filter(|r| !r.is_clean()) {
            eprintln!(
                "BAD: {} ({:?})",
                r.file.display(),
                r.manifest_id
            );
            for issue in &r.issues {
                eprintln!("  - {issue}");
            }
        }
        panic!(
            "{} of {} manifests are dirty; duplicate ids: {:?}",
            report.dirty, report.total_manifests, report.duplicate_ids
        );
    }
    assert!(
        report.total_manifests >= 50,
        "expected >= 50 manifests, got {}",
        report.total_manifests
    );
}

#[test]
fn no_duplicate_ids() {
    let root = find_catalog_root();
    let report = validate_catalog(&root).expect("validate catalog");
    assert!(
        report.duplicate_ids.is_empty(),
        "duplicate ids: {:?}",
        report.duplicate_ids
    );
}

#[test]
fn total_count_is_reasonable() {
    let root = find_catalog_root();
    let report = validate_catalog(&root).expect("validate catalog");
    assert!(report.total_manifests >= 50);
    assert!(report.total_manifests <= 200);
}
