//! 结算 — 扫描结果 + 定价 → UsageEvent (写 WAL)。
//!
//! 调用方: dispatch::retry 的成功/失败出口。结算后事件进 store::UsageStore
//! (edge = 本地 WAL append; 不阻塞转发)。

use contract::SCHEMA_VERSION;
use contract::records::{SyncMeta, UsageEventRecord};

use crate::ledger::Hold;
use crate::pricing::{PriceTable, price_of};
use crate::scanner::TokenCounts;

/// 生成并落盘一条 UsageEvent。
///
/// 幂等性: meta.key = UUIDv7 (edge 生成), center 端 ON CONFLICT DO NOTHING。
pub fn settle_event(
    counts: TokenCounts,
    hold: &Hold,
    price_table: &dyn PriceTable,
    channel_key: &str,
    route_unit_key: &str,
    public_model: &str,
    upstream_model: &str,
    first_token_ms: u32,
    duration_ms: u32,
    status_code: u16,
    error: Option<&str>,
) -> UsageEventRecord {
    // 查找模型价格 (默认 0 = 免费)
    let price = price_table
        .lookup(public_model)
        .unwrap_or(crate::pricing::ModelPrice {
            input: 0.0,
            output: 0.0,
            cache: 0.0,
            group_multiplier: 1.0,
        });

    let cost = price_of(counts, &price);

    UsageEventRecord {
        meta: SyncMeta {
            key: uuid::Uuid::new_v4().to_string(),
            schema_version: SCHEMA_VERSION,
            logical_version: 1,
            origin: "edge".into(),
            updated_at: chrono::Utc::now(),
        },
        token_key: hold.token_key.clone(),
        user_key: hold.user_key.clone(),
        channel_key: channel_key.to_string(),
        route_unit_key: route_unit_key.to_string(),
        public_model: public_model.to_string(),
        upstream_model: upstream_model.to_string(),
        prompt_tokens: counts.prompt,
        completion_tokens: counts.completion,
        cached_tokens: counts.cached,
        first_token_ms: first_token_ms,
        duration_ms: duration_ms,
        cost,
        status_code,
        error: error.map(|s| s.to_string()),
    }
}
