//! 计费纯函数测试 — 定价换算、校验、预扣/结算增量。
//!
//! 这些是钱路径上的纯函数：换算错一位就是账单错一位，必须锁住边界。

use api::billing::{
    ModelPricing, estimate_reserve, settle_delta, tokens_to_quota, validate_pricing,
};

/// 未配置定价 → 1 token 记 1 quota，不做任何缩放。
#[test]
fn unconfigured_is_one_to_one() {
    let quota = tokens_to_quota(100, 50, None);
    assert_eq!(quota, 150);
}

/// multiplier 0.5 对 1k 单价生效：2000 tokens × 1.0/1k × 0.5 = 1。
#[test]
fn configured_half_multiplier() {
    let p = ModelPricing {
        input_per_1k: 1.0,
        output_per_1k: 1.0,
        multiplier: 0.5,
    };
    assert_eq!(tokens_to_quota(1000, 1000, Some(&p)), 1);
    assert_eq!(tokens_to_quota(500, 500, Some(&p)), 1);
}

/// 溢出必须饱和到 u64::MAX，不能回绕成小额账单。
#[test]
fn tokens_to_quota_overflow_saturates() {
    let p = ModelPricing {
        input_per_1k: 1e12,
        output_per_1k: 0.0,
        multiplier: 1.0,
    };
    assert_eq!(tokens_to_quota(u64::MAX, 0, Some(&p)), u64::MAX);
}

/// 零 token 不产生费用。
#[test]
fn zero_tokens_is_zero() {
    let p = ModelPricing {
        input_per_1k: 2.0,
        output_per_1k: 3.0,
        multiplier: 1.0,
    };
    assert_eq!(tokens_to_quota(0, 0, Some(&p)), 0);
}

/// 负单价/负倍率非法；倍率 0 合法（免费模型）。
#[test]
fn validate_pricing_rejects_bad() {
    let mut p = ModelPricing {
        input_per_1k: -1.0,
        output_per_1k: 1.0,
        multiplier: 1.0,
    };
    assert!(validate_pricing(&p).is_err());
    p.input_per_1k = 1.0;
    p.multiplier = 0.0;
    assert!(validate_pricing(&p).is_ok()); // 0 = free model, valid
    p.multiplier = -0.5;
    assert!(validate_pricing(&p).is_err()); // negative = invalid
    p.multiplier = 1.0;
    assert!(validate_pricing(&p).is_ok());
}

/// 定价 DTO 拒绝未知字段，防止 admin 写错字段名却静默生效。
#[test]
fn deny_unknown_fields() {
    let json = r#"{"input_per_1k":1.0,"output_per_1k":1.0,"multiplier":1.0,"extra":1}"#;
    assert!(serde_json::from_str::<ModelPricing>(json).is_err());
}

/// 缺省倍率是 1.0，不是 0（否则默认全免费）。
#[test]
fn default_multiplier_is_one() {
    let p = ModelPricing::default();
    assert_eq!(p.multiplier, 1.0);
}

// ─── F10.2 + F10.3 reserve/settle 纯函数测试 ──────────────────────────

#[test]
fn estimate_reserve_is_fixed_1000() {
    assert_eq!(estimate_reserve(), 1000);
}

/// 实际超预扣 → 正增量（补扣）。
#[test]
fn settle_delta_positive_when_actual_exceeds_reserve() {
    assert_eq!(settle_delta(1500, 1000), 500);
}

/// 实际低于预扣 → 负增量（退回）。
#[test]
fn settle_delta_negative_when_actual_below_reserve() {
    assert_eq!(settle_delta(300, 1000), -700);
}

#[test]
fn settle_delta_zero_when_equal() {
    assert_eq!(settle_delta(1000, 1000), 0);
}

/// 实际 0（上游没返回 usage）→ 全额退回预扣。
#[test]
fn settle_delta_zero_actual() {
    assert_eq!(settle_delta(0, 1000), -1000);
}
