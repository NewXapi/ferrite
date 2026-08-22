//! F10.1 — 计费倍率配置：从 PG kv_store 读写模型定价，token → quota 换算

/// 模型计价配置（存 kv_store `pricing:{model}`）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ModelPricing {
    /// 每千 input token 单价
    pub input_per_1k: f64,
    /// 每千 output token 单价
    pub output_per_1k: f64,
    /// 倍率乘数（默认 1.0）
    pub multiplier: f64,
}

impl Default for ModelPricing {
    fn default() -> Self {
        Self {
            input_per_1k: 0.0,
            output_per_1k: 0.0,
            multiplier: 1.0,
        }
    }
}

/// 从 kv_store 读 `pricing:{model}` 的 value jsonb 反序列化
pub async fn read_pricing(
    pool: &crate::config::PgPool,
    model: &str,
) -> Result<Option<ModelPricing>, sqlx::Error> {
    match sqlx::query_as("SELECT value FROM kv_store WHERE key = $1")
        .bind(format!("pricing:{model}"))
        .fetch_optional(pool)
        .await?
    {
        Some((v,)) => serde_json::from_value(v)
            .map(Some)
            .map_err(|e| sqlx::Error::Decode(Box::new(e))),
        None => Ok(None),
    }
}

/// 写入 kv_store（UPSERT）
pub async fn write_pricing(
    pool: &crate::config::PgPool,
    model: &str,
    pricing: &ModelPricing,
) -> Result<(), sqlx::Error> {
    let value = serde_json::to_value(pricing).expect("ModelPricing is always serializable");
    sqlx::query("INSERT INTO kv_store (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2")
        .bind(format!("pricing:{model}"))
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

/// token → quota 换算：
/// - 未配置 → prompt_tokens + completion_tokens（1:1）
/// - 已配置 → ((prompt*input_per_1k + completion*output_per_1k) * multiplier / 1000.0).ceil() as u64
pub fn tokens_to_quota(
    prompt_tokens: u64,
    completion_tokens: u64,
    pricing: Option<&ModelPricing>,
) -> u64 {
    let Some(p) = pricing else {
        return prompt_tokens + completion_tokens;
    };
    let cost = (prompt_tokens as f64 * p.input_per_1k
        + completion_tokens as f64 * p.output_per_1k)
        * p.multiplier
        / 1000.0;
    cost.ceil() as u64
}

/// 防滥倍率校验：不允许负数单价/非正倍率
pub fn validate_pricing(p: &ModelPricing) -> Result<(), String> {
    if p.input_per_1k < 0.0 || p.output_per_1k < 0.0 {
        return Err("input_per_1k/output_per_1k must be >= 0".into());
    }
    if p.multiplier <= 0.0 {
        return Err("multiplier must be > 0".into());
    }
    Ok(())
}

// ─── F10.2 预扣 + F10.3 结算 ──────────────────────────────────────────────

/// 预扣估算额度（固定值，覆盖大多数请求的实际消耗）
pub const RESERVE_QUOTA: i64 = 1000;

/// 预扣估算函数 — 纯函数，便于单测
pub fn estimate_reserve() -> u64 {
    RESERVE_QUOTA as u64
}

/// 结算 delta：actual - reserve（正数=补扣，负数=退还）
pub fn settle_delta(actual: u64, reserve: u64) -> i64 {
    actual as i64 - reserve as i64
}

/// 预扣：原子增 used_quota，仅当剩余额度 ≥ reserve 时成功
/// 返回 Some(updated_used_quota) 表示成功，None 表示额度不足
pub async fn reserve_quota(
    pool: &crate::config::PgPool,
    token_key: &str,
    reserve: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        r#"UPDATE tokens SET used_quota = used_quota + $1
           WHERE key = $2 AND (quota - used_quota) >= $1
           RETURNING used_quota"#,
    )
    .bind(reserve)
    .bind(token_key)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(u,)| u))
}

/// 结算：将 used_quota 从 reserve 调整为 actual
/// delta = actual - reserve；正数补扣，负数退还
pub async fn settle_quota(
    pool: &crate::config::PgPool,
    token_key: &str,
    reserve: i64,
    actual: u64,
) -> Result<(), sqlx::Error> {
    let delta = settle_delta(actual, reserve as u64);
    sqlx::query("UPDATE tokens SET used_quota = used_quota + $1 WHERE key = $2")
        .bind(delta)
        .bind(token_key)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_is_one_to_one() {
        let quota = tokens_to_quota(100, 50, None);
        assert_eq!(quota, 150);
    }

    #[test]
    fn configured_half_multiplier() {
        // 每 1k token 单价 1.0，倍率 0.5
        let p = ModelPricing {
            input_per_1k: 1.0,
            output_per_1k: 1.0,
            multiplier: 0.5,
        };
        // (1000*1.0 + 1000*1.0) * 0.5 / 1000 = 1.0 → ceil = 1
        assert_eq!(tokens_to_quota(1000, 1000, Some(&p)), 1);
        // (500*1.0 + 500*1.0) * 0.5 / 1000 = 0.5 → ceil = 1
        assert_eq!(tokens_to_quota(500, 500, Some(&p)), 1);
    }

    #[test]
    fn zero_tokens_is_zero() {
        let p = ModelPricing {
            input_per_1k: 2.0,
            output_per_1k: 3.0,
            multiplier: 1.0,
        };
        assert_eq!(tokens_to_quota(0, 0, Some(&p)), 0);
    }

    #[test]
    fn validate_pricing_rejects_bad() {
        let mut p = ModelPricing { input_per_1k: -1.0, output_per_1k: 1.0, multiplier: 1.0 };
        assert!(validate_pricing(&p).is_err());
        p.input_per_1k = 1.0;
        p.multiplier = 0.0;
        assert!(validate_pricing(&p).is_err());
        p.multiplier = -0.5;
        assert!(validate_pricing(&p).is_err());
        p.multiplier = 1.0;
        assert!(validate_pricing(&p).is_ok());
    }

    #[test]
    fn deny_unknown_fields() {
        // 多余字段应被拒绝
        let json = r#"{"input_per_1k":1.0,"output_per_1k":1.0,"multiplier":1.0,"extra":1}"#;
        assert!(serde_json::from_str::<ModelPricing>(json).is_err());
    }

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

    #[test]
    fn settle_delta_positive_when_actual_exceeds_reserve() {
        // 实际 1500，预扣 1000 → 补扣 500
        assert_eq!(settle_delta(1500, 1000), 500);
    }

    #[test]
    fn settle_delta_negative_when_actual_below_reserve() {
        // 实际 300，预扣 1000 → 退还 -700
        assert_eq!(settle_delta(300, 1000), -700);
    }

    #[test]
    fn settle_delta_zero_when_equal() {
        assert_eq!(settle_delta(1000, 1000), 0);
    }

    #[test]
    fn settle_delta_zero_actual() {
        // usage 为空（流式无 usage）→ 全额退还
        assert_eq!(settle_delta(0, 1000), -1000);
    }
}
