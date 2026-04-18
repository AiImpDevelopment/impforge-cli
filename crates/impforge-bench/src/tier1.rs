// SPDX-License-Identifier: MIT
//! Tier 1 — function-level benchmarks (HumanEval + MBPP subsets).
//!
//! We ship a curated 10-case subset so the runner is fast to execute.
//! Full HumanEval / MBPP can be pulled via `impforge-cli bench --full`
//! in a later iteration.

use crate::report::BenchCase;

pub fn curated_cases() -> Vec<BenchCase> {
    vec![
        case("he-01", "humaneval", "Write a Python function `add(a, b)` that returns the sum of two integers.", "def add"),
        case("he-02", "humaneval", "Write a Python function `is_even(n)` that returns True if n is even.", "def is_even"),
        case("he-03", "humaneval", "Write a Python function `factorial(n)` that returns n! — use recursion.", "def factorial"),
        case("he-04", "humaneval", "Write a Python function `reverse(s)` that returns the reversed string.", "def reverse"),
        case("he-05", "humaneval", "Write a Python function `fib(n)` returning the nth Fibonacci number.", "def fib"),
        case("mbpp-01", "mbpp", "Write a Python function `is_prime(n)` that returns True if n is prime.", "def is_prime"),
        case("mbpp-02", "mbpp", "Write a Python function `max_of(lst)` returning the largest element.", "def max_of"),
        case("mbpp-03", "mbpp", "Write a Python function `unique(lst)` returning the list with duplicates removed.", "def unique"),
        case("mbpp-04", "mbpp", "Write a Python function `gcd(a, b)` using Euclid's algorithm.", "def gcd"),
        case("mbpp-05", "mbpp", "Write a Python function `word_count(s)` returning a dict mapping word → count.", "def word_count"),
    ]
}

fn case(id: &str, cat: &str, prompt: &str, expected: &str) -> BenchCase {
    BenchCase {
        id: format!("tier1-{id}"),
        tier: 1,
        category: cat.to_string(),
        prompt: prompt.to_string(),
        expected_signal: expected.to_string(),
    }
}

/// Pass = response contains the expected signal (function signature).
pub fn grade_response(case: &BenchCase, response: &str) -> bool {
    response.contains(&case.expected_signal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_has_ten_cases() {
        assert_eq!(curated_cases().len(), 10);
    }

    #[test]
    fn every_case_is_tier_1() {
        for c in curated_cases() {
            assert_eq!(c.tier, 1);
            assert!(!c.prompt.is_empty());
            assert!(!c.expected_signal.is_empty());
        }
    }

    #[test]
    fn grade_matches_signature() {
        let cases = curated_cases();
        let c = &cases[0];
        assert!(grade_response(c, "def add(a, b):\n    return a + b"));
        assert!(!grade_response(c, "def sum_two(a, b):\n    return a + b"));
    }

    #[test]
    fn grade_requires_exact_stem() {
        let cases = curated_cases();
        let c = cases.iter().find(|x| x.id.contains("he-02")).expect("case");
        assert!(grade_response(c, "def is_even(n): return n % 2 == 0"));
        assert!(!grade_response(c, "def even(n): return n % 2 == 0"));
    }
}
