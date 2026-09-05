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

/// 防滥倍率校验：不允许负数单价/负倍率 (0 = 免费 model 合法)
pub fn validate_pricing(p: &ModelPricing) -> Result<(), String> {
    if p.input_per_1k < 0.0 || p.output_per_1k < 0.0 {
        return Err("input_per_1k/output_per_1k must be >= 0".into());
    }
    if p.multiplier < 0.0 {
        return Err("multiplier must be >= 0 (0 = free model)".into());
    }
    Ok(())
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
    validate_pricing(pricing).map_err(sqlx::Error::Protocol)?;
    let value = serde_json::to_value(pricing).expect("ModelPricing is always serializable");
    sqlx::query("INSERT INTO kv_store (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2")
        .bind(format!("pricing:{model}"))
        .bind(value)
        .execute(pool)
        .await?;
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
    let cost = (prompt_tokens as f64 * p.input_per_1k + completion_tokens as f64 * p.output_per_1k)
        * p.multiplier
        / 1000.0;
    // 浮点上限护栏：f64 最大 ~1.8e19；超过视作饱和，防 panick/overflow
    // ponytail: 1e15 已够用, 不换 u128
    if !cost.is_finite() || cost >= 1e15 {
        return u64::MAX;
    }
    cost.ceil() as u64
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
/// 负 delta 时 GREATEST clamp 到 0，防 used_quota 被减成负数
pub async fn settle_quota(
    pool: &crate::config::PgPool,
    token_key: &str,
    reserve: i64,
    actual: u64,
) -> Result<(), sqlx::Error> {
    let delta = settle_delta(actual, reserve as u64);
    let applied = sqlx::query_scalar::<_, i64>(
        r#"UPDATE tokens SET used_quota = GREATEST(used_quota + $1, 0)
           WHERE key = $2 RETURNING used_quota"#,
    )
    .bind(delta)
    .bind(token_key)
    .fetch_optional(pool)
    .await?;
    // None = token 被删；admin 行为，不是错误
    if applied.is_none() {
        tracing::info!(token = %token_key, "settle_quota: token row missing, skip");
    }
    Ok(())
}
