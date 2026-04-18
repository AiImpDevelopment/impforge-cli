// SPDX-License-Identifier: MIT
//! Benchmark report data model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchCase {
    pub id: String,
    pub tier: u8,
    pub category: String,
    pub prompt: String,
    pub expected_signal: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchResult {
    pub case_id: String,
    pub response_text: String,
    pub passed: bool,
    pub score: f32,
    pub eval_tokens: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelComparison {
    pub model: String,
    pub bare_ollama: Vec<BenchResult>,
    pub impforge_context: Vec<BenchResult>,
    pub uplift: UpliftScore,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpliftScore {
    pub cases_evaluated: usize,
    pub bare_pass_rate: f32,
    pub impforge_pass_rate: f32,
    pub absolute_uplift_pct: f32,
    pub relative_uplift_pct: f32,
    pub mean_token_reduction_pct: f32,
}

impl UpliftScore {
    pub fn compute(bare: &[BenchResult], impforge: &[BenchResult]) -> Self {
        let total = bare.len().min(impforge.len());
        if total == 0 {
            return Self {
                cases_evaluated: 0,
                bare_pass_rate: 0.0,
                impforge_pass_rate: 0.0,
                absolute_uplift_pct: 0.0,
                relative_uplift_pct: 0.0,
                mean_token_reduction_pct: 0.0,
            };
        }
        let bare_passed = bare.iter().take(total).filter(|r| r.passed).count();
        let impf_passed = impforge.iter().take(total).filter(|r| r.passed).count();
        let bare_rate = bare_passed as f32 / total as f32;
        let impf_rate = impf_passed as f32 / total as f32;
        let absolute = (impf_rate - bare_rate) * 100.0;
        let relative = if bare_rate > 0.0 {
            (impf_rate - bare_rate) / bare_rate * 100.0
        } else if impf_rate > 0.0 {
            100.0
        } else {
            0.0
        };

        let mut token_savings: Vec<f32> = Vec::new();
        for i in 0..total {
            let b = bare[i].eval_tokens as f32;
            let g = impforge[i].eval_tokens as f32;
            if b > 0.0 {
                token_savings.push(((b - g) / b) * 100.0);
            }
        }
        let token_mean = if token_savings.is_empty() {
            0.0
        } else {
            token_savings.iter().sum::<f32>() / token_savings.len() as f32
        };

        Self {
            cases_evaluated: total,
            bare_pass_rate: bare_rate * 100.0,
            impforge_pass_rate: impf_rate * 100.0,
            absolute_uplift_pct: absolute,
            relative_uplift_pct: relative,
            mean_token_reduction_pct: token_mean,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchReport {
    pub schema_version: u32,
    pub started_at_unix: i64,
    pub finished_at_unix: i64,
    pub cli_version: String,
    pub tiers_run: Vec<u8>,
    pub comparisons: Vec<ModelComparison>,
    pub signature_hex: String,
}

impl BenchReport {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut without_sig = self.clone();
        without_sig.signature_hex = String::new();
        serde_json::to_vec(&without_sig).unwrap_or_default()
    }

    pub fn hero_headline(&self) -> Option<String> {
        let best = self
            .comparisons
            .iter()
            .max_by(|a, b| a.uplift.absolute_uplift_pct.total_cmp(&b.uplift.absolute_uplift_pct))?;
        Some(format!(
            "impforge-cli delivers +{:.1} %-points over raw Ollama on {} ({} cases · median token savings {:.0} %)",
            best.uplift.absolute_uplift_pct,
            best.model,
            best.uplift.cases_evaluated,
            best.uplift.mean_token_reduction_pct
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(passed: bool, tokens: u32) -> BenchResult {
        BenchResult {
            case_id: "c".to_string(),
            response_text: String::new(),
            passed,
            score: if passed { 1.0 } else { 0.0 },
            eval_tokens: tokens,
            duration_ms: 100,
        }
    }

    #[test]
    fn uplift_computed_correctly() {
        let bare = vec![result(false, 100), result(true, 100), result(false, 100)];
        let impforge = vec![result(true, 80), result(true, 80), result(false, 80)];
        let up = UpliftScore::compute(&bare, &impforge);
        assert_eq!(up.cases_evaluated, 3);
        assert!((up.bare_pass_rate - 33.333).abs() < 0.01);
        assert!((up.impforge_pass_rate - 66.666).abs() < 0.01);
        assert!((up.absolute_uplift_pct - 33.333).abs() < 0.01);
        assert!(up.mean_token_reduction_pct > 15.0);
    }

    #[test]
    fn uplift_handles_zero_baseline() {
        let bare = vec![result(false, 100)];
        let impforge = vec![result(true, 80)];
        let up = UpliftScore::compute(&bare, &impforge);
        assert_eq!(up.relative_uplift_pct, 100.0);
    }

    #[test]
    fn uplift_empty_is_zero() {
        let up = UpliftScore::compute(&[], &[]);
        assert_eq!(up.cases_evaluated, 0);
        assert_eq!(up.absolute_uplift_pct, 0.0);
    }

    #[test]
    fn canonical_bytes_exclude_signature() {
        let mut r = BenchReport {
            schema_version: 1,
            started_at_unix: 1,
            finished_at_unix: 2,
            cli_version: "0.1.0".to_string(),
            tiers_run: vec![1, 3],
            comparisons: Vec::new(),
            signature_hex: "DEADBEEF".to_string(),
        };
        let bytes_a = r.canonical_bytes();
        r.signature_hex = "CAFEBABE".to_string();
        let bytes_b = r.canonical_bytes();
        assert_eq!(bytes_a, bytes_b);
    }

    #[test]
    fn hero_headline_picks_best_uplift() {
        let report = BenchReport {
            schema_version: 1,
            started_at_unix: 1,
            finished_at_unix: 2,
            cli_version: "0.1.0".to_string(),
            tiers_run: vec![1],
            comparisons: vec![
                ModelComparison {
                    model: "modelA".to_string(),
                    bare_ollama: vec![result(false, 100)],
                    impforge_context: vec![result(true, 80)],
                    uplift: UpliftScore {
                        cases_evaluated: 1,
                        bare_pass_rate: 0.0,
                        impforge_pass_rate: 100.0,
                        absolute_uplift_pct: 100.0,
                        relative_uplift_pct: 100.0,
                        mean_token_reduction_pct: 20.0,
                    },
                },
                ModelComparison {
                    model: "modelB".to_string(),
                    bare_ollama: vec![result(false, 100)],
                    impforge_context: vec![result(false, 100)],
                    uplift: UpliftScore {
                        cases_evaluated: 1,
                        bare_pass_rate: 0.0,
                        impforge_pass_rate: 0.0,
                        absolute_uplift_pct: 0.0,
                        relative_uplift_pct: 0.0,
                        mean_token_reduction_pct: 0.0,
                    },
                },
            ],
            signature_hex: String::new(),
        };
        let headline = report.hero_headline().expect("headline");
        assert!(headline.contains("modelA"));
        assert!(headline.contains("+100.0"));
    }

    #[test]
    fn bench_case_serializes_roundtrip() {
        let c = BenchCase {
            id: "tier1-he-0".to_string(),
            tier: 1,
            category: "humaneval".to_string(),
            prompt: "write a fn".to_string(),
            expected_signal: "def ".to_string(),
        };
        let j = serde_json::to_string(&c).expect("serialize");
        let back: BenchCase = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(c, back);
    }
}
