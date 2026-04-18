// SPDX-License-Identifier: MIT
//! # impforge-bench — Scientific uplift measurement.
//!
//! Proves with numbers that impforge-cli materially improves any local
//! model (qwen3-imp, qwen2.5-coder, llama-3.3, deepseek-coder, …) over
//! raw Ollama.
//!
//! ## Four tiers
//!
//! | Tier | What it measures | Source |
//! |------|------------------|--------|
//! | **1** | Function-level (saturated baseline) | HumanEval-subset + MBPP-subset |
//! | **2** | Real-world engineering | SWE-Bench-Verified stub + LiveCodeBench seeds |
//! | **3** | impforge-specific uplift (our moat) | 78 industry prompts + 2 600 compliance Q&A |
//! | **4** | Behavioural (dims 6+7 Crown-Jewel) | agent-trace + error-recall store |
//!
//! ## Methodology
//!
//! * Pairwise AB: identical prompts to raw Ollama and to
//!   `impforge-cli generate` with the same base model.
//! * Seeds pinned, temperature fixed, 3 runs per prompt, median reported.
//! * Per-model breakdown: qwen3-imp:8b · qwen2.5-coder:7b · llama3.3:8b · …
//! * Ed25519-signed JSON reports published at `impforge.com/benchmarks`.

pub mod module;
pub mod report;
pub mod runner;
pub mod tier1;
pub mod tier3;
pub mod tier4;

pub use module::Module_;
pub use report::{BenchCase, BenchReport, BenchResult, ModelComparison, UpliftScore};
pub use runner::{run_pairwise_ab, BenchConfig};
