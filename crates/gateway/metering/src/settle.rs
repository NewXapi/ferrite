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
/// TODO(#335): 定价表定型后接 pricing::price_of; 失败请求 (无 usage) 也产事件
/// (tokens=0, status_code/error 填实), 供健康统计与审计。
pub fn settle_event(
    counts: TokenCounts,
    hold: &Hold,
    _price_table: &dyn PriceTable,
) -> UsageEventRecord {
    let _ = price_of(
        counts,
        &super::pricing::ModelPrice {
            input: 0.0,
            output: 0.0,
            cache: 0.0,
            group_multiplier: 1.0,
        },
    );
    UsageEventRecord {
        meta: SyncMeta {
            key: String::new(), // TODO(#336): uuid::now_v7() 注入 — store 层生成
            schema_version: SCHEMA_VERSION,
            logical_version: 1,
            origin: "edge".into(),
            updated_at: chrono::Utc::now(),
        },
        token_key: hold.token_key.clone(),
        user_key: hold.user_key.clone(),
        channel_key: String::new(),
        route_unit_key: String::new(),
        public_model: String::new(),
        upstream_model: String::new(),
        prompt_tokens: counts.prompt,
        completion_tokens: counts.completion,
        cached_tokens: counts.cached,
        first_token_ms: 0,
        duration_ms: 0,
        cost: 0,
        status_code: 200,
        error: None,
    }
}
