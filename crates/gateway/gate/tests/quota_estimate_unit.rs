//! 单位契约回归测试：`estimate_cost` 必须与 `metering::pricing::price_of`
//! 走同一本账。Bug 复现：旧代码 `as i64` 截断 + `* 1_000_000` 膨胀
//! 导致预估恒为 0 或 1e6 倍。

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
    assert_eq!(cost, 30_720, "gpt-4o 4096 output tokens must be 30_720 internal units");
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

#[test]
fn estimate_cost_aligns_with_price_of_semantics() {
    // 这条测试用字面量锚定：metering::pricing::price_of 对 4096 completion token
    // 与 estimate_cost(内部单位形态, 4096) 必须相等（两条路径同一本账）。
    //
    // metering::pricing::price_of 逻辑（参考 crates/gateway/metering/src/pricing.rs）：
    // let total_dollars = output_dollars * multipliers; // $15 * 1 = $15
    // let raw = total_dollars * 500_000.0; // $15 * 500_000 = 7_500_000 内部单位/M
    // 对 4096 token: 7_500_000 * 4096 / 1_000_000 = 30_720
    //
    // 因 metering crate 非 gate 的 dev-dependency，这里用等值字面量断言。
    let price = PriceRow {
        input_per_m: 0.0,
        output_per_m: 7_500_000.0,
        cache_per_m: 0.0,
    };
    let cost_estimate = estimate_cost(&price, 4096);

    // metering 同值计算（去耦实现，仅锚定契约数值）
    let total_dollars = 15.0 * 1.0; // output $15/M, multiplier 1.0
    let raw = total_dollars * 500_000.0; // 7_500_000 内部单位/M
    let cost_price_of = (raw * 4096.0 / 1_000_000.0).ceil() as i64;

    assert_eq!(
        cost_estimate, cost_price_of,
        "estimate_cost must match price_of semantics for same inputs"
    );
}