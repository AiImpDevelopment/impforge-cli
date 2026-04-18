// SPDX-License-Identifier: MIT
//! Generate a ready-to-paste GitHub PR body from a contribution.

use impforge_core::TemplateManifest;

pub fn build_pr_body(manifest: &TemplateManifest) -> String {
    format!(
        r#"## New Template: `{id}`

### Summary

{description}

### Metadata

- **Industry**: `{industry}`
- **Category**: `{category}`
- **Framework**: `{framework}`
- **Language**: `{language}`
- **License**: `{license}`
- **Safety class**: `{safety_class}`
- **Compliance regimes**: {compliance}
- **Tags**: {tags}

### Quality checklist (completed locally)

- [x] `template.json` v1 schema validated
- [x] Compliance regimes match `^[A-Z0-9-]+$`
- [x] Security scan passed (no dangerous APIs, no secrets)
- [x] Prompt-injection scrubber passed
- [x] Preview command runs locally
- [x] Build command produces a clean artefact
- [ ] Reviewed by maintainer

🤖 Generated with [`impforge-cli contribute`](https://github.com/AiImpDevelopment/impforge-cli)
"#,
        id = manifest.id,
        description = manifest.description,
        industry = manifest.industry,
        category = manifest.category,
        framework = manifest.framework,
        language = manifest.language,
        license = manifest.license,
        safety_class = manifest.safety_class,
        compliance = manifest
            .compliance
            .iter()
            .map(|r| format!("`{r}`"))
            .collect::<Vec<_>>()
            .join(" · "),
        tags = manifest
            .tags
            .iter()
            .map(|t| format!("`{t}`"))
            .collect::<Vec<_>>()
            .join(" · "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pr_body_contains_manifest_id() {
        let m = TemplateManifest {
            id: "demo-saas".to_string(),
            name: "Demo SaaS".to_string(),
            description: "demo description".to_string(),
            category: "saas".to_string(),
            industry: "demo".to_string(),
            framework: "next-15".to_string(),
            language: "typescript".to_string(),
            license: "MIT".to_string(),
            compliance: vec!["GDPR".to_string()],
            tags: vec!["demo".to_string()],
            compliance_rule_count: 1,
            safety_class: "t1_filesystem".to_string(),
            preview_command: "bun run dev".to_string(),
            preview_ready_url: "http://localhost:3000".to_string(),
            build_command: "bun run build".to_string(),
        };
        let body = build_pr_body(&m);
        assert!(body.contains("demo-saas"));
        assert!(body.contains("GDPR"));
        assert!(body.contains("impforge-cli contribute"));
    }
}
