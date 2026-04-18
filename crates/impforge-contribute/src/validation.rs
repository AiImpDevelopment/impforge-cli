// SPDX-License-Identifier: MIT
//! Local validation before a community contribution is submitted.

use impforge_core::{CoreError, CoreResult, TemplateManifest};

pub fn validate_template_submission(manifest: &TemplateManifest) -> CoreResult<()> {
    manifest.validate()?;
    if manifest.description.len() < 30 {
        return Err(CoreError::validation(
            "description must be at least 30 characters — help other users",
        ));
    }
    if manifest.tags.is_empty() {
        return Err(CoreError::validation("at least one tag is required"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpl() -> TemplateManifest {
        TemplateManifest {
            id: "demo".to_string(),
            name: "Demo".to_string(),
            description: "A sufficiently long description for the demo".to_string(),
            category: "web".to_string(),
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
        }
    }

    #[test]
    fn valid_submission_passes() {
        assert!(validate_template_submission(&tmpl()).is_ok());
    }

    #[test]
    fn short_description_rejected() {
        let mut t = tmpl();
        t.description = "too short".to_string();
        assert!(validate_template_submission(&t).is_err());
    }

    #[test]
    fn missing_tags_rejected() {
        let mut t = tmpl();
        t.tags.clear();
        assert!(validate_template_submission(&t).is_err());
    }
}
