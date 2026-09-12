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
    assert_eq!(price_of(counts, &price, 1.0), 0);
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
    assert_eq!(price_of(counts, &price, 1.0), 7_500_000);
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
    assert_eq!(price_of(counts, &price, 1.0), 15_000_000);
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
    assert_eq!(price_of(counts, &price, 1.0), 75_000_000);
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
    assert_eq!(price_of(counts, &price, 1.0), 0);
}

// ---------- group_ratio (请求分组倍率) ----------

/// group_ratio=2 在表内 group_multiplier 之外再乘一档：cost 翻倍。
#[test]
fn price_of_group_ratio_doubles_cost() {
    let counts = TokenCounts {
        prompt: 1_000_000,
        completion: 1_000_000,
        cached: 0,
    };
    let price = ModelPrice {
        input: 15.0,
        output: 60.0,
        cache: 0.0,
        group_multiplier: 1.0,
    };
    // $75 × gm 1.0 × ratio 2 = $150 → 150 * 500_000 = 75_000_000
    assert_eq!(price_of(counts, &price, 2.0), 75_000_000);
}

/// 两级倍率相乘：表内 gm 与请求侧 group_ratio 独立叠乘。
#[test]
fn price_of_group_multiplier_times_group_ratio() {
    let counts = TokenCounts {
        prompt: 1_000_000,
        completion: 0,
        cached: 0,
    };
    let price = ModelPrice {
        input: 15.0,
        output: 60.0,
        cache: 0.0,
        group_multiplier: 3.0,
    };
    // $15 × 3 (gm) × 4 (ratio) = $180 → 90_000_000
    assert_eq!(price_of(counts, &price, 4.0), 90_000_000);
}

/// 下限语义：非零消耗向上取整至少 1 单位（7.5 → 8），零消耗仍为 0。
#[test]
fn price_of_ceil_lower_bound() {
    let counts = TokenCounts {
        prompt: 1,
        completion: 0,
        cached: 0,
    };
    let price = ModelPrice {
        input: 15.0,
        output: 60.0,
        cache: 0.0,
        group_multiplier: 1.0,
    };
    // 1 tok × $15/M = $1.5e-5 → ×500_000 = 7.5 → ceil 8
    assert_eq!(price_of(counts, &price, 1.0), 8);
    // 0 token 不产生任何账单（ceil 不把零抬成非零）
    assert_eq!(price_of(TokenCounts::default(), &price, 2.0), 0);
}

/// cache 列独立计价且同样吃两级倍率。
#[test]
fn price_of_cache_column_with_ratio() {
    let counts = TokenCounts {
        prompt: 0,
        completion: 0,
        cached: 1_000_000,
    };
    let price = ModelPrice {
        input: 15.0,
        output: 60.0,
        cache: 3.75,
        group_multiplier: 2.0,
    };
    // $3.75 × 2 (gm) × 1 (ratio) = $7.5 → 3_750_000
    assert_eq!(price_of(counts, &price, 1.0), 3_750_000);
}

// ---------- ConfigPriceTable::lookup(model, group) ----------

/// 配置表按 model 命中、与 group 无关（组倍率真值走 settle 的 group_ratio，
/// 不经表）；未知 model → None（settle 侧按免费处理）。
#[test]
fn config_table_lookup_hits_by_model_regardless_of_group() {
    use metering::pricing::{ConfigPriceTable, PriceTable};
    use std::collections::HashMap;

    let mut prices = HashMap::new();
    prices.insert(
        "gpt-4o".to_string(),
        ModelPrice {
            input: 15.0,
            output: 60.0,
            cache: 0.0,
            group_multiplier: 1.0,
        },
    );
    let table = ConfigPriceTable::new(prices);

    assert!(table.lookup("gpt-4o", "default").is_some());
    assert!(
        table.lookup("gpt-4o", "vip").is_some(),
        "model 命中即有价, 与 group 无关"
    );
    assert!(table.lookup("unknown-model", "vip").is_none());
}
