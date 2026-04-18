// SPDX-License-Identifier: MIT
//! Directory-tree scanner — drives the per-line checks plus the file-
//! level checks for Dimensions 4 and 5.

use crate::dims;
use crate::report::{CrownJewelReport, Dimension, DimensionTotals};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SCANNABLE_EXTENSIONS: &[&str] =
    &["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "svelte", "vue", "cs", "kt"];

pub fn scan<P: AsRef<Path>>(root: P) -> anyhow::Result<CrownJewelReport> {
    let root_owned = root.as_ref().to_path_buf();
    let mut findings = Vec::new();
    let mut files_scanned = 0_usize;

    for entry in WalkDir::new(&root_owned).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if !SCANNABLE_EXTENSIONS.contains(&ext) {
            continue;
        }
        if is_generated_or_vendored(entry.path()) {
            continue;
        }
        files_scanned += 1;
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for (i, line) in content.lines().enumerate() {
            let in_test = line_is_inside_test_block(&content, i);
            findings.extend(dims::scan_line(entry.path(), i + 1, line, in_test));
        }

        // Dimension 4: test-first — only Rust files carry `pub fn` + `#[test]`.
        if ext == "rs" {
            let pub_items = extract_pub_items(&content);
            let test_names = extract_test_names(&content);
            findings.extend(dims::dim4_test_first(entry.path(), &pub_items, &test_names));
        }
    }

    let dimension_totals = tally(&findings);
    Ok(CrownJewelReport {
        root: root_owned,
        files_scanned,
        findings,
        dimension_totals,
    })
}

/// Convenience — scan a workspace directory AND run Dimension 5 against
/// the workspace root's `Cargo.toml` + the orchestrator bootstrap file.
pub fn scan_workspace<P: AsRef<Path>>(
    workspace_root: P,
    bootstrap_path: Option<&Path>,
) -> anyhow::Result<CrownJewelReport> {
    let mut report = scan(&workspace_root)?;
    let members = workspace_members(workspace_root.as_ref())?;
    let registered = match bootstrap_path {
        Some(p) => registered_modules(p)?,
        None => Vec::new(),
    };
    let bootstrap_anchor = bootstrap_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workspace_root.as_ref().to_path_buf());
    report
        .findings
        .extend(dims::dim5_crown_jewel_wiring(&bootstrap_anchor, &members, &registered));
    report.dimension_totals = tally(&report.findings);
    Ok(report)
}

fn tally(findings: &[crate::report::CrownJewelFinding]) -> DimensionTotals {
    let mut t = DimensionTotals::default();
    for f in findings {
        match f.dimension {
            Dimension::NoStubs => t.no_stubs += 1,
            Dimension::NoSuppression => t.no_suppression += 1,
            Dimension::NoLonelyUnwrap => t.no_lonely_unwrap += 1,
            Dimension::TestFirst => t.test_first += 1,
            Dimension::CrownJewelWiring => t.crown_jewel_wiring += 1,
            Dimension::ParallelEfficiency => t.parallel_efficiency += 1,
            Dimension::ErrorRecall => t.error_recall += 1,
        }
    }
    t
}

/// Behavioral scan (dims 6 + 7) using on-disk telemetry.
pub fn scan_behavior(
    prior_errors: &[crate::behavior::ErrorRecallEntry],
    fresh_errors: &[crate::behavior::ErrorRecallEntry],
) -> anyhow::Result<CrownJewelReport> {
    let trace_path = crate::behavior::agent_trace_path()?;
    let trace = crate::behavior::read_trace(&trace_path).unwrap_or_default();
    let mut findings = crate::behavior::dim6_parallel_efficiency(&trace_path, &trace);
    let recall_path = crate::behavior::error_recall_path()?;
    findings.extend(crate::behavior::dim7_error_recall(&recall_path, prior_errors, fresh_errors));
    let dimension_totals = tally(&findings);
    Ok(CrownJewelReport {
        root: trace_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default(),
        files_scanned: 0,
        findings,
        dimension_totals,
    })
}

fn is_generated_or_vendored(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy().to_lowercase();
        s == "target"
            || s == "node_modules"
            || s == "dist"
            || s == ".next"
            || s == "vendor"
            || s == "build"
    })
}

fn line_is_inside_test_block(content: &str, line_index: usize) -> bool {
    if !content.contains("#[cfg(test)]") {
        return false;
    }
    let mut brace_depth: i32 = 0;
    let mut inside = false;
    for (i, line) in content.lines().enumerate() {
        if line.contains("#[cfg(test)]") {
            inside = true;
            brace_depth = 0;
        }
        if inside {
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
            if i == line_index {
                return true;
            }
            if brace_depth <= 0 && line.contains('}') {
                inside = false;
            }
        }
    }
    false
}

fn extract_pub_items(content: &str) -> Vec<String> {
    let mut items = Vec::new();
    for line in content.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("pub fn ") {
            let name = rest.split(['(', '<', ' ']).next().unwrap_or("").to_string();
            if !name.is_empty() {
                items.push(name);
            }
        } else if let Some(rest) = t.strip_prefix("pub struct ") {
            let name = rest
                .split([' ', '<', '{', '(', ';'])
                .next()
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                items.push(name);
            }
        } else if let Some(rest) = t.strip_prefix("pub enum ") {
            let name = rest
                .split([' ', '<', '{'])
                .next()
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                items.push(name);
            }
        }
    }
    items
}

fn extract_test_names(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if !t.contains("#[test]") {
            continue;
        }
        // Case A: `#[test]` on its own line — scan next 1–4 lines for `fn NAME`.
        if t == "#[test]" {
            let end = (i + 5).min(lines.len());
            for candidate in lines.iter().take(end).skip(i + 1) {
                if let Some(name) = extract_fn_name(candidate) {
                    names.push(name);
                    break;
                }
            }
            continue;
        }
        // Case B: `#[test]` inline before `fn NAME` on the same line.
        if let Some(pos) = line.find("#[test]") {
            let tail = &line[pos + "#[test]".len()..];
            if let Some(name) = extract_fn_name(tail) {
                names.push(name);
            }
        }
    }
    names
}

fn extract_fn_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn workspace_members(root: &Path) -> anyhow::Result<Vec<String>> {
    let cargo = root.join("Cargo.toml");
    if !cargo.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&cargo)?;
    let mut members = Vec::new();
    let mut inside = false;
    for line in raw.lines() {
        let t = line.trim();
        if t.starts_with("members") && t.contains('[') {
            inside = true;
            // Handle `members = ["a", "b"]` single-line case.
            if t.contains(']') {
                if let (Some(start), Some(end)) = (t.find('['), t.rfind(']')) {
                    for token in t[start + 1..end].split(',') {
                        let cleaned = token.trim_matches(|c: char| {
                            c == '"' || c == ',' || c.is_whitespace()
                        });
                        if let Some(name) = cleaned.strip_prefix("crates/") {
                            members.push(name.to_string());
                        }
                    }
                }
                inside = false;
            }
            continue;
        }
        if inside {
            if t.contains(']') {
                inside = false;
                continue;
            }
            let cleaned = t.trim_matches(|c: char| c == '"' || c == ',' || c.is_whitespace());
            if let Some(name) = cleaned.strip_prefix("crates/") {
                members.push(name.to_string());
            }
        }
    }
    Ok(members)
}

fn registered_modules(bootstrap: &Path) -> anyhow::Result<Vec<String>> {
    if !bootstrap.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(bootstrap)?;
    let mut names = Vec::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.contains("orc.register(") && t.contains("Module_") {
            if let Some(start) = t.find("Arc::new(") {
                let tail = &t[start + "Arc::new(".len()..];
                let crate_with_underscore = tail.split("::Module_").next().unwrap_or("");
                let as_crate = crate_with_underscore.replace('_', "-");
                if !as_crate.is_empty() {
                    names.push(as_crate);
                }
            }
        }
    }
    Ok(names)
}

fn _parent_path(path: &Path) -> PathBuf {
    path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn clean_tree_reports_zero_findings() {
        let td = TempDir::new().expect("td");
        fs::write(
            td.path().join("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn add_works() { assert_eq!(add(1,2), 3); }\n}",
        )
        .expect("w");
        let r = scan(td.path()).expect("scan");
        assert!(r.is_clean(), "findings: {:?}", r.findings);
    }

    #[test]
    fn stub_flagged_and_critical_blocks() {
        let td = TempDir::new().expect("td");
        fs::write(td.path().join("a.rs"), "fn foo() { unimplemented!(); }").expect("w");
        let r = scan(td.path()).expect("scan");
        assert!(r.dimension_totals.no_stubs >= 1);
        assert!(!r.is_clean());
        assert!(r.blocking_count() >= 1);
    }

    #[test]
    fn allow_attribute_flagged() {
        let td = TempDir::new().expect("td");
        fs::write(
            td.path().join("a.rs"),
            "#[allow(dead_code)]\npub fn f() {}\n#[cfg(test)]\nmod tests { #[test] fn f_works() {} }",
        )
        .expect("w");
        let r = scan(td.path()).expect("scan");
        assert!(r.dimension_totals.no_suppression >= 1);
    }

    #[test]
    fn untested_pub_fn_flagged() {
        let td = TempDir::new().expect("td");
        fs::write(
            td.path().join("a.rs"),
            "pub fn uncovered() -> i32 { 42 }",
        )
        .expect("w");
        let r = scan(td.path()).expect("scan");
        assert!(r.dimension_totals.test_first >= 1);
    }

    #[test]
    fn tested_pub_fn_passes() {
        let td = TempDir::new().expect("td");
        fs::write(
            td.path().join("a.rs"),
            "pub fn covered() -> i32 { 42 }\n#[cfg(test)]\nmod tests { #[test] fn covered_works() {} }",
        )
        .expect("w");
        let r = scan(td.path()).expect("scan");
        assert_eq!(r.dimension_totals.test_first, 0);
    }

    #[test]
    fn extract_pub_items_finds_fn_struct_enum() {
        let src = "pub fn alpha() {}\npub struct Beta;\npub enum Gamma { X }";
        let items = extract_pub_items(src);
        assert_eq!(items, vec!["alpha".to_string(), "Beta".to_string(), "Gamma".to_string()]);
    }

    #[test]
    fn extract_test_names_grabs_fn_names() {
        let src = "#[test]\nfn a_works() {}\n#[test]\nasync fn b_works() {}";
        let names = extract_test_names(src);
        assert_eq!(names, vec!["a_works".to_string(), "b_works".to_string()]);
    }

    #[test]
    fn workspace_members_reads_cargo() {
        let td = TempDir::new().expect("td");
        fs::write(
            td.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\n  \"crates/a\",\n  \"crates/b\",\n]\n",
        )
        .expect("w");
        let m = workspace_members(td.path()).expect("ok");
        assert_eq!(m, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn registered_modules_parses_bootstrap() {
        let td = TempDir::new().expect("td");
        let f = td.path().join("runtime.rs");
        fs::write(
            &f,
            "orc.register(Arc::new(impforge_scaffold::Module_))?;\norc.register(Arc::new(impforge_models::Module_))?;",
        )
        .expect("w");
        let mods = registered_modules(&f).expect("ok");
        assert_eq!(
            mods,
            vec!["impforge-scaffold".to_string(), "impforge-models".to_string()]
        );
    }

    #[test]
    fn scan_workspace_flags_unregistered_crate() {
        let td = TempDir::new().expect("td");
        fs::write(
            td.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/impforge-scaffold\", \"crates/impforge-orphan\"]\n",
        )
        .expect("w");
        let bootstrap = td.path().join("runtime.rs");
        fs::write(
            &bootstrap,
            "orc.register(Arc::new(impforge_scaffold::Module_))?;",
        )
        .expect("w");
        let r = scan_workspace(td.path(), Some(&bootstrap)).expect("scan");
        assert!(r.dimension_totals.crown_jewel_wiring >= 1);
    }

    #[test]
    fn generated_dirs_excluded() {
        let td = TempDir::new().expect("td");
        let node_modules = td.path().join("node_modules");
        fs::create_dir_all(&node_modules).expect("mkdir");
        fs::write(node_modules.join("bad.rs"), "fn x() { unimplemented!(); }").expect("w");
        let r = scan(td.path()).expect("scan");
        assert!(r.findings.is_empty());
    }
}
