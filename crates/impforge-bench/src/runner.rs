// SPDX-License-Identifier: MIT
//! Pairwise AB runner.  Executes each benchmark case against raw Ollama
//! and against impforge-cli-context variants of the same model.

use crate::report::{BenchCase, BenchResult, ModelComparison, UpliftScore};
use crate::{tier1, tier3, tier4};
use impforge_core::{CoreError, CoreResult};
use impforge_models::ollama;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchConfig {
    pub models: Vec<String>,
    pub tiers: Vec<u8>,
    pub runs_per_case: u32,
    pub system_prompt: Option<String>,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            models: vec!["qwen3-imp:8b".to_string()],
            tiers: vec![1, 3, 4],
            runs_per_case: 3,
            system_prompt: None,
        }
    }
}

/// The impforge context-injected system prompt.  Mirrors what the
/// `generate` command would attach when scaffolding a template.
pub const IMPFORGE_CONTEXT_PROMPT: &str =
    "You are an assistant augmented with impforge-cli.  When a user prompt mentions an industry or a compliance regime, answer using that industry's named template (see impforge-cli template list) and cite the specific compliance regime(s) that apply.  Be concise, name abbreviations you recognise, and prefer production-grade language.";

pub fn collect_cases(tiers: &[u8]) -> Vec<BenchCase> {
    let mut cases = Vec::new();
    if tiers.contains(&1) {
        cases.extend(tier1::curated_cases());
    }
    if tiers.contains(&3) {
        cases.extend(tier3::industry_cases());
        cases.extend(tier3::compliance_cases());
    }
    if tiers.contains(&4) {
        cases.extend(tier4::behavioral_cases());
    }
    cases
}

pub fn grade(case: &BenchCase, response: &str) -> bool {
    match case.tier {
        1 => tier1::grade_response(case, response),
        3 => tier3::grade_response(case, response),
        4 => tier4::grade_response(case, response),
        _ => false,
    }
}

pub fn run_pairwise_ab(config: &BenchConfig) -> CoreResult<Vec<ModelComparison>> {
    let cases = collect_cases(&config.tiers);
    if cases.is_empty() {
        return Err(CoreError::validation("no tiers selected"));
    }
    if !ollama::is_reachable(None) {
        return Err(CoreError::Network(
            "Ollama not reachable at 127.0.0.1:11434 — run `ollama serve`".to_string(),
        ));
    }

    let mut comparisons = Vec::with_capacity(config.models.len());
    for model in &config.models {
        let mut bare_results = Vec::with_capacity(cases.len());
        let mut impf_results = Vec::with_capacity(cases.len());
        for case in &cases {
            let bare = median_of_n(model, case, None, config.runs_per_case)?;
            let impf = median_of_n(
                model,
                case,
                config
                    .system_prompt
                    .as_deref()
                    .or(Some(IMPFORGE_CONTEXT_PROMPT)),
                config.runs_per_case,
            )?;
            bare_results.push(bare);
            impf_results.push(impf);
        }
        let uplift = UpliftScore::compute(&bare_results, &impf_results);
        comparisons.push(ModelComparison {
            model: model.clone(),
            bare_ollama: bare_results,
            impforge_context: impf_results,
            uplift,
        });
    }
    Ok(comparisons)
}

fn median_of_n(
    model: &str,
    case: &BenchCase,
    system_prompt: Option<&str>,
    runs: u32,
) -> CoreResult<BenchResult> {
    let runs = runs.max(1);
    let mut attempts: Vec<BenchResult> = Vec::with_capacity(runs as usize);
    for _ in 0..runs {
        let started = Instant::now();
        let resp = ollama::generate_once(model, &case.prompt, system_prompt, None)?;
        let duration_ms = started.elapsed().as_millis() as u64;
        let passed = grade(case, &resp.response);
        attempts.push(BenchResult {
            case_id: case.id.clone(),
            response_text: resp.response.clone(),
            passed,
            score: if passed { 1.0 } else { 0.0 },
            eval_tokens: resp.eval_count,
            duration_ms,
        });
    }
    attempts.sort_by(|a, b| a.score.total_cmp(&b.score));
    let mid = attempts.len() / 2;
    Ok(attempts.swap_remove(mid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_picks_tiers_1_3_4() {
        let c = BenchConfig::default();
        assert_eq!(c.tiers, vec![1, 3, 4]);
        assert_eq!(c.runs_per_case, 3);
    }

    #[test]
    fn collect_cases_combines_requested_tiers() {
        let cases = collect_cases(&[1]);
        assert_eq!(cases.len(), tier1::curated_cases().len());
        let all = collect_cases(&[1, 3, 4]);
        assert!(all.len() > cases.len());
    }

    #[test]
    fn collect_cases_empty_tiers_returns_empty() {
        assert!(collect_cases(&[]).is_empty());
    }

    #[test]
    fn grade_delegates_to_tier_grader() {
        let tier1_case = &tier1::curated_cases()[0];
        assert!(grade(tier1_case, "def add(a, b): return a + b"));
        let tier3_cases = tier3::industry_cases();
        let tier3_case = &tier3_cases[0];
        // Respond with exactly the regime the tier-3 grader expects.
        let response = format!("uses {}", tier3_case.expected_signal);
        assert!(grade(tier3_case, &response));
    }

    #[test]
    fn grade_unknown_tier_returns_false() {
        let case = BenchCase {
            id: "fake-99".to_string(),
            tier: 99,
            category: "fake".to_string(),
            prompt: "x".to_string(),
            expected_signal: "y".to_string(),
        };
        assert!(!grade(&case, "y"));
    }

    #[test]
    fn context_prompt_non_empty() {
        assert!(!IMPFORGE_CONTEXT_PROMPT.is_empty());
        assert!(IMPFORGE_CONTEXT_PROMPT.contains("impforge-cli"));
    }
}
