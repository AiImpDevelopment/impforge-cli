// SPDX-License-Identifier: MIT
//! Canonical manifest types for templates, skills, and MCP manifests.
//!
//! These are the v1-spec types enforced everywhere in the workspace.

use crate::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const ALLOWED_CATEGORIES: &[&str] = &["web", "saas", "backend", "mobile", "desktop", "agent"];
pub const ALLOWED_SAFETY_CLASSES: &[&str] =
    &["t0_pure", "t1_filesystem", "t2_network", "t3_system"];
pub const ALLOWED_LICENSES: &[&str] = &["MIT", "Apache-2.0", "BSD-3-Clause"];

/// v1 spec `template.json` manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub industry: String,
    pub framework: String,
    pub language: String,
    pub license: String,
    #[serde(default)]
    pub compliance: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub compliance_rule_count: usize,
    pub safety_class: String,
    pub preview_command: String,
    pub preview_ready_url: String,
    pub build_command: String,
}

impl TemplateManifest {
    pub fn validate(&self) -> CoreResult<()> {
        if self.id.is_empty() {
            return Err(CoreError::invalid_manifest("id is empty"));
        }
        if !ALLOWED_CATEGORIES.contains(&self.category.as_str()) {
            return Err(CoreError::invalid_manifest(format!(
                "category '{}' not in {ALLOWED_CATEGORIES:?}",
                self.category
            )));
        }
        if !ALLOWED_LICENSES.contains(&self.license.as_str()) {
            return Err(CoreError::invalid_manifest(format!(
                "license '{}' not in {ALLOWED_LICENSES:?}",
                self.license
            )));
        }
        if !ALLOWED_SAFETY_CLASSES.contains(&self.safety_class.as_str()) {
            return Err(CoreError::invalid_manifest(format!(
                "safety_class '{}' not in {ALLOWED_SAFETY_CLASSES:?}",
                self.safety_class
            )));
        }
        let mut seen = HashSet::new();
        for regime in &self.compliance {
            if regime.is_empty() {
                return Err(CoreError::invalid_manifest("empty compliance regime entry"));
            }
            if !regime
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
            {
                return Err(CoreError::invalid_manifest(format!(
                    "compliance regime '{regime}' must match ^[A-Z0-9-]+$"
                )));
            }
            if !seen.insert(regime.as_str()) {
                return Err(CoreError::invalid_manifest(format!(
                    "duplicate compliance regime '{regime}'"
                )));
            }
        }
        Ok(())
    }
}

/// v1 spec `skill.md` manifest (front-matter only — the body is free-form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub activation_cue: Option<String>,
}

impl SkillManifest {
    pub fn validate(&self) -> CoreResult<()> {
        if self.id.is_empty() {
            return Err(CoreError::invalid_manifest("skill id is empty"));
        }
        if !ALLOWED_LICENSES.contains(&self.license.as_str()) {
            return Err(CoreError::invalid_manifest(format!(
                "skill license '{}' not in {ALLOWED_LICENSES:?}",
                self.license
            )));
        }
        Ok(())
    }
}

/// Individual compliance rule — matches the JSON shape in every template's
/// `compliance_rules.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceRule {
    pub id: String,
    pub regime: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub enforcement: String,
    pub citation: String,
}

/// v1 spec MCP manifest describing a registered MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub transport: McpTransport,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    pub license: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    HttpSse,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_template() -> TemplateManifest {
        TemplateManifest {
            id: "fintech-saas".to_string(),
            name: "FinTech SaaS".to_string(),
            description: "description".to_string(),
            category: "saas".to_string(),
            industry: "fintech".to_string(),
            framework: "next-15".to_string(),
            language: "typescript".to_string(),
            license: "MIT".to_string(),
            compliance: vec!["PCI-DSS".to_string(), "GDPR".to_string()],
            tags: vec!["fintech".to_string()],
            compliance_rule_count: 100,
            safety_class: "t1_filesystem".to_string(),
            preview_command: "bun run dev".to_string(),
            preview_ready_url: "http://localhost:3000".to_string(),
            build_command: "bun run build".to_string(),
        }
    }

    #[test]
    fn valid_template_passes() {
        assert!(sample_template().validate().is_ok());
    }

    #[test]
    fn bad_category_rejected() {
        let mut t = sample_template();
        t.category = "quantum".to_string();
        assert!(t.validate().is_err());
    }

    #[test]
    fn lowercase_regime_rejected() {
        let mut t = sample_template();
        t.compliance = vec!["Pci-Dss".to_string()];
        assert!(t.validate().is_err());
    }

    #[test]
    fn duplicate_regime_rejected() {
        let mut t = sample_template();
        t.compliance = vec!["GDPR".to_string(), "GDPR".to_string()];
        assert!(t.validate().is_err());
    }

    #[test]
    fn bad_license_rejected() {
        let mut t = sample_template();
        t.license = "GPL-3.0".to_string();
        assert!(t.validate().is_err());
    }

    #[test]
    fn skill_missing_license_rejected() {
        let s = SkillManifest {
            id: "threat-modeling".to_string(),
            name: "Threat Modeling".to_string(),
            description: "desc".to_string(),
            license: "EULA".to_string(),
            tags: vec![],
            activation_cue: None,
        };
        assert!(s.validate().is_err());
    }
}
