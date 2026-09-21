//! 单位契约回归测试：`estimate_cost` 必须与 `metering::pricing::price_of`
//! 走同一本账（1 内部单位 = $1/500_000）。Bug 复现：旧代码 `as i64` 截断
//! + `* 1_000_000` 膨胀，导致预估值恒为 0，「余额<预估→402」在生产空转。

use gateway_gate::quota::estimate_cost;
use gateway_gate::snapshot::PriceRow;

#[test]
fn estimate_cost_unit_contract_gpt4o_4096() {
    // gpt-4o output $15/M -> 15 * 500_000 = 7_500_000 内部单位/M
    // max_tokens=4096 -> 7_500_000 * 4096 / 1_000_000 = 30_720 内部单位
    let price = PriceRow {
        input_per_m: 0.0,
        output_per_m: 7_500_000.0,
        cache_per_m: 0.0,
    };
    let cost = estimate_cost(&price, 4096);
    assert_eq!(
        cost, 30_720,
        "gpt-4o 4096 output tokens must be 30_720 internal units"
    );
}

#[test]
fn estimate_cost_small_max_tokens_not_zero() {
    // 小 max_tokens 不能因截断变 0（旧代码 as i64 会让 1 token 变 0）
    let price = PriceRow {
        input_per_m: 0.0,
        output_per_m: 7_500_000.0,
        cache_per_m: 0.0,
    };
    // 1 token -> 7_500_000 * 1 / 1_000_000 = 7.5 -> ceil = 8
    let cost = estimate_cost(&price, 1);
    assert!(cost >= 1, "small token count must not round to zero");
}
