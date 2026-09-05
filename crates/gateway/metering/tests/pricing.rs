//! pricing 测试 — 验证 cost 计算。

use metering::pricing::{ModelPrice, price_of};
use metering::scanner::TokenCounts;

#[test]
fn price_of_zero_tokens() {
    let counts = TokenCounts::default();
    let price = ModelPrice {
        input: 15.0,
        output: 60.0,
        cache: 0.0,
        group_multiplier: 1.0,
    };
    assert_eq!(price_of(counts, &price), 0);
}

#[test]
fn price_of_input_only() {
    let counts = TokenCounts {
        prompt: 1_000_000,
        completion: 0,
        cached: 0,
    };
    let price = ModelPrice {
        input: 15.0,
        output: 60.0,
        cache: 0.0,
        group_multiplier: 1.0,
    };
    // 1M * $15/M = $15 → 15 * 500_000 = 7_500_000
    assert_eq!(price_of(counts, &price), 7_500_000);
}

#[test]
fn price_of_output_only() {
    let counts = TokenCounts {
        prompt: 0,
        completion: 500_000,
        cached: 0,
    };
    let price = ModelPrice {
        input: 15.0,
        output: 60.0,
        cache: 0.0,
        group_multiplier: 1.0,
    };
    // 500K * $60/M = $30 → 30 * 500_000 = 15_000_000
    assert_eq!(price_of(counts, &price), 15_000_000);
}

#[test]
fn price_of_with_group_multiplier() {
    let counts = TokenCounts {
        prompt: 1_000_000,
        completion: 1_000_000,
        cached: 0,
    };
    let price = ModelPrice {
        input: 15.0,
        output: 60.0,
        cache: 0.0,
        group_multiplier: 2.0,
    };
    // (1M * 15/M + 1M * 60/M) * 2 = ($15 + $60) * 2 = $150 → 150 * 500_000 = 75_000_000
    assert_eq!(price_of(counts, &price), 75_000_000);
}

#[test]
fn price_of_free_model() {
    let counts = TokenCounts {
        prompt: 1000,
        completion: 500,
        cached: 0,
    };
    let price = ModelPrice {
        input: 0.0,
        output: 0.0,
        cache: 0.0,
        group_multiplier: 1.0,
    };
    assert_eq!(price_of(counts, &price), 0);
}