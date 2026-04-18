// SPDX-License-Identifier: MIT
//! Tier 3 — impforge-specific uplift cases.  This is our moat: no other
//! CLI has 78 industry templates + 2 600 compliance rules on disk, so a
//! model given that context naturally out-performs the same model given
//! only the raw prompt.

use crate::report::BenchCase;

pub fn industry_cases() -> Vec<BenchCase> {
    let seeds = [
        ("fintech-saas",  "Scaffold a FinTech multi-tenant SaaS with BSA, PCI-DSS, GDPR, and SOC2 awareness.  Name the framework and the three most important compliance regimes."),
        ("healthcare-saas", "Scaffold a HIPAA-compliant healthcare EMR SaaS.  Name the four most important compliance regimes and the tenancy model."),
        ("legal-saas", "Scaffold a legal SaaS with ABA-MRPC, FRCP, and GoBD awareness.  Explain why SchemaPerTenant beats SharedRls here."),
        ("retail-saas",  "Scaffold a retail SaaS with PCI-DSS and CCPA — name the auth provider and the billing provider."),
        ("education-saas", "Scaffold an education SaaS with FERPA + COPPA awareness.  What tenancy model makes sense and why?"),
        ("insurance-saas", "Scaffold an insurance SaaS with Solvency-II and IFRS-17 awareness.  Name the framework."),
        ("construction-saas", "Scaffold a construction SaaS with OSHA and ISO-45001 awareness.  What are the top three compliance regimes?"),
        ("manufacturing-saas", "Scaffold a manufacturing SaaS with UFLPA and ITAR awareness.  Name the framework and the auth provider."),
        ("telecom-saas", "Scaffold a telecom SaaS with CPNI + CALEA awareness.  Which framework is the default?"),
        ("pharma-saas", "Scaffold a pharma SaaS with FDA-21-CFR-Part-11 and ICH awareness.  What tenancy model is the default?"),
        ("mining-saas", "Scaffold a mining operator SaaS with MSHA and ICMM awareness.  Name the framework."),
        ("maritime-saas", "Scaffold a maritime SaaS with IMO-SOLAS and MARPOL awareness.  Name the three most important regimes."),
        ("government-saas", "Scaffold a government SaaS with FOIA, FedRAMP, and NIST-800-171 awareness.  What tenancy model is required?"),
        ("proptech-saas", "Scaffold a proptech SaaS with Fair-Housing and FCRA awareness.  Which backend tables should be scaffolded?"),
    ];
    seeds
        .iter()
        .map(|(id, prompt)| BenchCase {
            id: format!("tier3-{id}"),
            tier: 3,
            category: "industry".to_string(),
            prompt: prompt.to_string(),
            expected_signal: first_regime_in_prompt(prompt).to_string(),
        })
        .collect()
}

fn first_regime_in_prompt(prompt: &str) -> &str {
    let candidates = [
        "BSA", "PCI-DSS", "GDPR", "SOC2", "HIPAA", "ABA-MRPC", "FRCP", "GoBD",
        "CCPA", "FERPA", "COPPA", "Solvency-II", "IFRS-17", "OSHA", "ISO-45001",
        "UFLPA", "ITAR", "CPNI", "CALEA", "FDA-21-CFR-Part-11", "ICH", "MSHA",
        "ICMM", "IMO-SOLAS", "MARPOL", "FOIA", "FedRAMP", "NIST-800-171",
        "Fair-Housing", "FCRA",
    ];
    for c in candidates {
        if prompt.contains(c) {
            return c;
        }
    }
    ""
}

/// Grade: response must name at least one of the regimes referenced in the
/// prompt.  Case-insensitive.
pub fn grade_response(case: &BenchCase, response: &str) -> bool {
    if case.expected_signal.is_empty() {
        return false;
    }
    response.to_lowercase().contains(&case.expected_signal.to_lowercase())
}

pub fn compliance_cases() -> Vec<BenchCase> {
    vec![
        BenchCase {
            id: "tier3-comp-01".to_string(),
            tier: 3,
            category: "compliance".to_string(),
            prompt: "Is PCI-DSS v4.0 applicable to a SaaS that stores primary account numbers?  Answer yes or no and cite one rule.".to_string(),
            expected_signal: "PCI-DSS".to_string(),
        },
        BenchCase {
            id: "tier3-comp-02".to_string(),
            tier: 3,
            category: "compliance".to_string(),
            prompt: "Which GDPR article governs data subject access requests?  Name the article number.".to_string(),
            expected_signal: "Art".to_string(),
        },
        BenchCase {
            id: "tier3-comp-03".to_string(),
            tier: 3,
            category: "compliance".to_string(),
            prompt: "What does 'CPNI' stand for in US telecom law?  Expand the abbreviation.".to_string(),
            expected_signal: "Customer Proprietary Network".to_string(),
        },
        BenchCase {
            id: "tier3-comp-04".to_string(),
            tier: 3,
            category: "compliance".to_string(),
            prompt: "Name the FDA regulation that governs electronic records and electronic signatures in clinical trials.".to_string(),
            expected_signal: "21 CFR Part 11".to_string(),
        },
        BenchCase {
            id: "tier3-comp-05".to_string(),
            tier: 3,
            category: "compliance".to_string(),
            prompt: "Which SEC regulation requires quarterly 10-Q filings?  Name the form and the time window.".to_string(),
            expected_signal: "10-Q".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn industry_has_fourteen_seeds() {
        assert_eq!(industry_cases().len(), 14);
    }

    #[test]
    fn industry_cases_are_tier3() {
        for c in industry_cases() {
            assert_eq!(c.tier, 3);
            assert!(!c.expected_signal.is_empty(), "missing signal: {}", c.id);
        }
    }

    #[test]
    fn compliance_cases_have_expected_signal() {
        for c in compliance_cases() {
            assert!(!c.expected_signal.is_empty());
        }
    }

    #[test]
    fn grade_is_case_insensitive() {
        let c = &industry_cases()[0];
        assert!(grade_response(c, "uses BSA and PCI-DSS"));
        assert!(grade_response(c, "uses bsa"));
        assert!(!grade_response(c, "no relevant regimes"));
    }

    #[test]
    fn first_regime_detector_stable() {
        assert_eq!(first_regime_in_prompt("needs PCI-DSS and GDPR"), "PCI-DSS");
        assert_eq!(first_regime_in_prompt("just GDPR"), "GDPR");
    }
}
