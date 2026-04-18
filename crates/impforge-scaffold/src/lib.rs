// SPDX-License-Identifier: MIT
//! Template scaffolding for impforge-cli.
//!
//! Given a template id + a target directory, this crate copies every file
//! from the bundled `templates/<id>` directory into the target while
//! refusing unsafe paths.  The scaffolding engine is deliberately plain:
//! no templating language, no macros, no code execution.  All variation
//! lives inside the source template.

use impforge_core::{CoreError, CoreResult, TemplateManifest};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scaffold_template(
    templates_root: &Path,
    template_id: &str,
    target_dir: &Path,
) -> CoreResult<ScaffoldReport> {
    let source = templates_root.join(template_id);
    if !source.exists() {
        return Err(CoreError::TemplateNotFound(template_id.to_string()));
    }
    ensure_safe_target(target_dir)?;
    fs::create_dir_all(target_dir)?;

    let manifest_path = source.join("template.json");
    if !manifest_path.exists() {
        return Err(CoreError::invalid_manifest(format!(
            "template '{template_id}' has no template.json"
        )));
    }
    let manifest: TemplateManifest =
        serde_json::from_slice(&fs::read(&manifest_path)?)?;
    manifest.validate()?;

    let mut file_count = 0_usize;
    let mut total_bytes = 0_usize;
    let mut running_hash = Sha256::new();

    for entry in WalkDir::new(&source).min_depth(1) {
        let entry = entry.map_err(|e| CoreError::other(e.to_string()))?;
        let rel = entry
            .path()
            .strip_prefix(&source)
            .map_err(|e| CoreError::other(e.to_string()))?;
        reject_unsafe_relative(rel)?;
        let out = target_dir.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&out)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            let bytes = fs::read(entry.path())?;
            running_hash.update(rel.to_string_lossy().as_bytes());
            running_hash.update(&bytes);
            total_bytes += bytes.len();
            file_count += 1;
            fs::write(&out, bytes)?;
        }
    }

    let content_hash = hex::encode(running_hash.finalize());

    Ok(ScaffoldReport {
        template_id: template_id.to_string(),
        target: target_dir.to_path_buf(),
        file_count,
        total_bytes,
        content_hash,
        manifest,
    })
}

fn ensure_safe_target(target: &Path) -> CoreResult<()> {
    if target.as_os_str().is_empty() {
        return Err(CoreError::UnsafePath(
            target.display().to_string(),
            "empty path".to_string(),
        ));
    }
    if target.to_string_lossy().contains("..") {
        return Err(CoreError::UnsafePath(
            target.display().to_string(),
            "contains '..'".to_string(),
        ));
    }
    Ok(())
}

fn reject_unsafe_relative(rel: &Path) -> CoreResult<()> {
    for comp in rel.components() {
        use std::path::Component;
        match comp {
            Component::Normal(_) => {}
            _ => {
                return Err(CoreError::UnsafePath(
                    rel.display().to_string(),
                    "only normal components allowed inside template".to_string(),
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ScaffoldReport {
    pub template_id: String,
    pub target: PathBuf,
    pub file_count: usize,
    pub total_bytes: usize,
    pub content_hash: String,
    pub manifest: TemplateManifest,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_manifest_json() -> String {
        serde_json::json!({
            "id": "demo-web",
            "name": "Demo Web",
            "description": "a demo web template for tests",
            "category": "web",
            "industry": "demo",
            "framework": "next-15",
            "language": "typescript",
            "license": "MIT",
            "compliance": ["GDPR"],
            "tags": ["demo"],
            "complianceRuleCount": 1,
            "safetyClass": "t1_filesystem",
            "previewCommand": "bun run dev",
            "previewReadyUrl": "http://localhost:3000",
            "buildCommand": "bun run build"
        })
        .to_string()
    }

    fn make_template(root: &Path) {
        let t = root.join("demo-web");
        fs::create_dir_all(t.join("src")).expect("mkdir");
        fs::write(t.join("template.json"), sample_manifest_json()).expect("write");
        fs::write(t.join("README.md"), "# Demo").expect("write");
        fs::write(t.join("src/main.ts"), "console.log('hi');").expect("write");
    }

    #[test]
    fn scaffold_copies_all_files() {
        let src = TempDir::new().expect("tempdir");
        let dst = TempDir::new().expect("tempdir");
        make_template(src.path());
        let report = scaffold_template(src.path(), "demo-web", &dst.path().join("out"))
            .expect("scaffold");
        assert_eq!(report.file_count, 3);
        assert!(dst.path().join("out/README.md").exists());
        assert!(dst.path().join("out/src/main.ts").exists());
    }

    #[test]
    fn unknown_template_rejected() {
        let src = TempDir::new().expect("tempdir");
        let dst = TempDir::new().expect("tempdir");
        let err = scaffold_template(src.path(), "nonexistent", dst.path()).expect_err("must fail");
        assert!(matches!(err, CoreError::TemplateNotFound(_)));
    }

    #[test]
    fn dotdot_target_rejected() {
        let src = TempDir::new().expect("tempdir");
        make_template(src.path());
        let err = scaffold_template(
            src.path(),
            "demo-web",
            Path::new("/tmp/../etc/evil"),
        )
        .expect_err("must fail");
        assert!(matches!(err, CoreError::UnsafePath(_, _)));
    }

    #[test]
    fn scaffold_hash_is_deterministic() {
        let src = TempDir::new().expect("tempdir");
        let dst1 = TempDir::new().expect("tempdir");
        let dst2 = TempDir::new().expect("tempdir");
        make_template(src.path());
        let a = scaffold_template(src.path(), "demo-web", &dst1.path().join("a"))
            .expect("a");
        let b = scaffold_template(src.path(), "demo-web", &dst2.path().join("b"))
            .expect("b");
        assert_eq!(a.content_hash, b.content_hash);
    }
}
