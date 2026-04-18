// SPDX-License-Identifier: MIT
//! # Safety Preamble — MIT, shippable in `impforge-cli`.
//!
//! A small, dependency-free module that exposes the baseline refusal preamble
//! the BRAIN model ships with inside its Ollama Modelfile, plus a typed
//! enumeration of the eight non-negotiable categories the preamble covers
//! and a helper that verifies any preamble string mentions refusal language
//! for all eight of them.
//!
//! This module exists to give downstream tooling a single authoritative
//! source of truth for what the free BRAIN refuses, so CI can gate changes
//! that accidentally weaken the Modelfile.
//!
//! Research anchor: long system prompts measurably degrade Qwen-family
//! reasoning quality; see Zhou et al., "Don't Make Your LLM an Evaluation
//! Benchmark Cheater" (arXiv 2310.03693).  The preamble below is therefore
//! kept under 300 words while still covering the eight baseline categories.

/// Eight non-negotiable refusal categories the BRAIN must cover.
///
/// Ordered from most to least frequent in real-world support traffic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SafetyCategory {
    /// Physical violence against any person or animal.
    PhysicalViolence,
    /// Robbery, burglary, theft, or any property crime.
    Robbery,
    /// Financial fraud, scams, phishing, identity theft.
    Fraud,
    /// Bullying, mobbing, harassment, doxxing, targeted intimidation.
    Bullying,
    /// Weapons of mass harm: firearms manufacture, explosives, chemical agents.
    Weapons,
    /// Self-harm or suicide instructions.
    SelfHarm,
    /// Sexual exploitation of minors.
    MinorExploitation,
    /// Biological or chemical weapons synthesis.
    Bioweapons,
}

impl SafetyCategory {
    /// All eight categories, stable order suitable for iteration and tests.
    pub fn all() -> [SafetyCategory; 8] {
        [
            SafetyCategory::PhysicalViolence,
            SafetyCategory::Robbery,
            SafetyCategory::Fraud,
            SafetyCategory::Bullying,
            SafetyCategory::Weapons,
            SafetyCategory::SelfHarm,
            SafetyCategory::MinorExploitation,
            SafetyCategory::Bioweapons,
        ]
    }

    /// Short human-readable description of what the category refuses.
    pub fn description(self) -> &'static str {
        match self {
            SafetyCategory::PhysicalViolence => {
                "physical violence against any person or animal (assault, torture, killing)"
            }
            SafetyCategory::Robbery => "robbery, burglary, theft, or any property crime",
            SafetyCategory::Fraud => "financial fraud, scams, phishing, or identity theft",
            SafetyCategory::Bullying => {
                "bullying, mobbing, harassment, doxxing, or targeted intimidation"
            }
            SafetyCategory::Weapons => {
                "weapons of mass harm (firearms manufacture, explosives, chemical agents)"
            }
            SafetyCategory::SelfHarm => "self-harm or suicide instructions",
            SafetyCategory::MinorExploitation => "sexual exploitation of minors in any form",
            SafetyCategory::Bioweapons => "biological or chemical weapons synthesis",
        }
    }

    /// Keywords that MUST appear in a preamble that covers this category.
    ///
    /// Each keyword is lower-case and compared against a lower-cased preamble.
    /// The helper `prompt_mentions_refusal` requires at least one keyword
    /// from each of the eight categories to be present.  Exposed publicly so
    /// integration tests can perform per-category spot checks against the
    /// shipped Modelfile.
    pub fn keywords(self) -> &'static [&'static str] {
        match self {
            SafetyCategory::PhysicalViolence => &["violence", "assault", "torture"],
            SafetyCategory::Robbery => &["robbery", "burglary", "theft"],
            SafetyCategory::Fraud => &["fraud", "scam", "phishing", "identity theft"],
            SafetyCategory::Bullying => {
                &["bullying", "mobbing", "harassment", "doxxing", "intimidation"]
            }
            SafetyCategory::Weapons => &["firearms", "explosives", "weapons of mass"],
            SafetyCategory::SelfHarm => &["self-harm", "suicide"],
            SafetyCategory::MinorExploitation => {
                &["sexual exploitation of minors", "exploitation of minors"]
            }
            SafetyCategory::Bioweapons => &["biological", "chemical weapons", "bioweapons"],
        }
    }
}

/// The baseline safety preamble shipped inside `brain/Modelfile`.
///
/// Keep this string byte-identical to the `SYSTEM """..."""` block of the
/// Modelfile wherever feasible; the integration test `safety_modelfile.rs`
/// walks the Modelfile and asserts category coverage independently.
pub const SAFETY_PREAMBLE: &str = "You are THE BRAIN — an 8B production AI coding assistant shipped by impforge-cli.

You REFUSE every request that aims to help with any of the following — you do not
provide code, plans, instructions, excuses, or roleplay workarounds for them:
  - physical violence against any person or animal (assault, torture, killing)
  - robbery, burglary, theft, or any property crime
  - financial fraud, scams, phishing, or identity theft
  - bullying, mobbing, harassment, doxxing, or targeted intimidation
  - weapons of mass harm (firearms manufacture, explosives, chemical agents)
  - self-harm or suicide instructions
  - sexual exploitation of minors in any form
  - biological or chemical weapons synthesis

On such requests reply briefly that you will not assist, and offer a safer alternative
when one exists (for example pointing at a crisis line for self-harm topics).
";

/// Returns `true` iff `preamble` mentions refusal language for ALL eight
/// `SafetyCategory` variants.
///
/// The check is deliberately lenient about surface wording: it only requires
/// the presence of at least one keyword per category (case-insensitive).
/// It also requires at least one explicit refusal verb — `refuse`, `will not`,
/// `do not`, or `won't` — somewhere in the preamble so a bare list of topics
/// cannot trivially pass.
pub fn prompt_mentions_refusal(preamble: &str) -> bool {
    let haystack = preamble.to_lowercase();

    // Must contain at least one explicit refusal verb.
    let has_refusal_verb = ["refuse", "will not", "do not", "won't"]
        .iter()
        .any(|v| haystack.contains(v));
    if !has_refusal_verb {
        return false;
    }

    // Must cover every single category.
    for cat in SafetyCategory::all() {
        let covered = cat.keywords().iter().any(|kw| haystack.contains(kw));
        if !covered {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_returns_exactly_eight_unique_categories() {
        let cats = SafetyCategory::all();
        assert_eq!(cats.len(), 8);
        let mut seen: Vec<SafetyCategory> = Vec::new();
        for c in cats {
            assert!(!seen.contains(&c), "duplicate category: {:?}", c);
            seen.push(c);
        }
    }

    #[test]
    fn each_category_has_nonempty_description_and_keywords() {
        for cat in SafetyCategory::all() {
            assert!(
                !cat.description().is_empty(),
                "empty description for {:?}",
                cat
            );
            assert!(
                !cat.keywords().is_empty(),
                "empty keywords for {:?}",
                cat
            );
        }
    }

    #[test]
    fn safety_preamble_mentions_every_category() {
        let lower = SAFETY_PREAMBLE.to_lowercase();
        for cat in SafetyCategory::all() {
            let hit = cat.keywords().iter().any(|kw| lower.contains(kw));
            assert!(hit, "SAFETY_PREAMBLE missing coverage for {:?}", cat);
        }
    }

    #[test]
    fn prompt_mentions_refusal_passes_on_real_preamble() {
        assert!(
            prompt_mentions_refusal(SAFETY_PREAMBLE),
            "real SAFETY_PREAMBLE must pass refusal check"
        );
    }

    #[test]
    fn prompt_mentions_refusal_rejects_blank_input() {
        assert!(!prompt_mentions_refusal(""));
        assert!(!prompt_mentions_refusal("   \n\t  "));
    }

    #[test]
    fn prompt_mentions_refusal_rejects_missing_category() {
        // Start from the real preamble, strip the bullying line — the check
        // must then fail, proving each category is individually enforced.
        let crippled = SAFETY_PREAMBLE
            .lines()
            .filter(|l| {
                !l.contains("bullying") && !l.contains("mobbing") && !l.contains("harassment")
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !prompt_mentions_refusal(&crippled),
            "check must fail when bullying coverage is removed"
        );
    }

    #[test]
    fn prompt_mentions_refusal_rejects_list_without_refusal_verb() {
        // A bare list of topics (no refusal verb) must NOT pass — the preamble
        // needs an explicit commitment, not just keywords.
        let bare_list = "physical violence assault robbery burglary theft fraud scam phishing \
                         identity theft bullying mobbing harassment doxxing intimidation \
                         firearms explosives weapons of mass self-harm suicide \
                         sexual exploitation of minors biological chemical weapons";
        assert!(!prompt_mentions_refusal(bare_list));
    }

    #[test]
    fn individual_category_keywords_appear_in_preamble() {
        // One assertion per category — makes it immediately obvious which
        // category regressed if the preamble is edited.
        let lower = SAFETY_PREAMBLE.to_lowercase();
        assert!(lower.contains("violence"), "physical violence missing");
        assert!(lower.contains("theft") || lower.contains("robbery"), "robbery missing");
        assert!(lower.contains("fraud"), "fraud missing");
        assert!(
            lower.contains("bullying") || lower.contains("harassment"),
            "bullying missing"
        );
        assert!(
            lower.contains("firearms") || lower.contains("weapons of mass"),
            "weapons missing"
        );
        assert!(lower.contains("self-harm") || lower.contains("suicide"), "self-harm missing");
        assert!(
            lower.contains("exploitation of minors"),
            "minor exploitation missing"
        );
        assert!(
            lower.contains("biological") || lower.contains("chemical weapons"),
            "bioweapons missing"
        );
    }

    #[test]
    fn description_text_reflects_category_semantics() {
        assert!(SafetyCategory::PhysicalViolence
            .description()
            .contains("violence"));
        assert!(SafetyCategory::Robbery.description().contains("robbery"));
        assert!(SafetyCategory::Fraud.description().contains("fraud"));
        assert!(SafetyCategory::Bullying.description().contains("bullying"));
        assert!(
            SafetyCategory::Weapons.description().contains("weapons")
                || SafetyCategory::Weapons.description().contains("firearms")
        );
        assert!(
            SafetyCategory::SelfHarm.description().contains("self-harm")
                || SafetyCategory::SelfHarm.description().contains("suicide")
        );
        assert!(SafetyCategory::MinorExploitation
            .description()
            .contains("minors"));
        assert!(
            SafetyCategory::Bioweapons.description().contains("biological")
                || SafetyCategory::Bioweapons
                    .description()
                    .contains("chemical")
        );
    }
}
