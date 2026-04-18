// SPDX-License-Identifier: MIT
//! Integration test — asserts that `brain/Modelfile` (the runtime artefact
//! shipped to end users) carries refusal coverage for all eight baseline
//! safety categories, leaks no private-repo internals, and stays inside the
//! 300-word system-prompt budget recommended for Qwen3-family models.
//!
//! The Modelfile is located via `CARGO_MANIFEST_DIR` so the test runs from
//! any working directory.

use std::fs;
use std::path::PathBuf;

use impforge_crown_jewel::{prompt_mentions_refusal, SafetyCategory};

/// Load the shipped `brain/Modelfile` as a string.
fn load_modelfile() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // crates/impforge-crown-jewel -> brain/Modelfile
    let path: PathBuf = PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .join("brain")
        .join("Modelfile");
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e))
}

/// Extract the contents between the first `SYSTEM """` and the matching
/// closing `"""` on its own line.  Panics if the block is malformed — the
/// test suite depends on that block existing.
fn extract_system_block(modelfile: &str) -> String {
    let start_marker = "SYSTEM \"\"\"";
    let start = modelfile
        .find(start_marker)
        .expect("Modelfile must contain a SYSTEM \"\"\"...\"\"\" block");
    let after = &modelfile[start + start_marker.len()..];
    let end_rel = after
        .find("\n\"\"\"")
        .expect("Modelfile SYSTEM block must terminate with a closing \"\"\" on its own line");
    after[..end_rel].to_string()
}

#[test]
fn modelfile_covers_all_eight_safety_categories() {
    let modelfile = load_modelfile();
    let system_block = extract_system_block(&modelfile);

    assert!(
        prompt_mentions_refusal(&system_block),
        "SYSTEM block must mention refusal language for every SafetyCategory.\n\
         Block was:\n{}",
        system_block
    );

    // Per-category spot checks — if this fails, the error message tells the
    // developer exactly which category regressed.
    let lower = system_block.to_lowercase();
    for cat in SafetyCategory::all() {
        let hit = cat.keywords().iter().any(|kw| lower.contains(kw));
        assert!(
            hit,
            "Modelfile SYSTEM block is missing coverage for {:?}.\n\
             At least one of these keywords must appear: {:?}",
            cat,
            cat.keywords()
        );
    }
}

#[test]
fn modelfile_has_no_private_repo_internals() {
    let modelfile = load_modelfile();

    // Forbidden substrings — all MIT-incompatible or Pro-internal.
    // Each assertion carries its own message so the regression is obvious.
    let forbidden: &[&str] = &[
        "ImpForge Pro",
        "SafetyCategory",
        "selective_alignment",
        "digu_privacy",
        "Elastic-2.0",
    ];

    for needle in forbidden {
        assert!(
            !modelfile.contains(needle),
            "Modelfile must not mention {:?} (Pro-internal / non-MIT leak)",
            needle
        );
    }
}

#[test]
fn system_block_stays_under_three_hundred_words() {
    // Qwen3 degrades measurably past ~300 words of system prompt
    // (Zhou et al., arXiv 2310.03693).  Keep it tight.
    let modelfile = load_modelfile();
    let system_block = extract_system_block(&modelfile);
    let word_count = system_block.split_whitespace().count();
    assert!(
        word_count < 300,
        "SYSTEM block is {} words — must stay under 300 to preserve Qwen3 quality.",
        word_count
    );
}

