//! 结算 — 扫描结果 + 定价 → UsageEvent (写 usage_logs 平表)。
//!
//! 调用方: dispatch::retry 的成功/失败出口。结算后事件落 usage_logs，
//! admin-observe 直接查表聚合，不阻塞转发。

use contract::SCHEMA_VERSION;
use contract::records::{SyncMeta, UsageEventRecord};

use crate::ledger::Hold;
use crate::pricing::{PriceTable, price_of};
use crate::scanner::TokenCounts;

/// 生成并落盘一条 UsageEvent。
///
/// `group` 随 model 一起进 [`PriceTable::lookup`]（实现可按组给价）；
/// `group_ratio` 是请求分组倍率（GroupRecord.rate_multiplier），乘进
/// [`price_of`]；库层调用方拿不到组倍率真值时传 1.0。
/// 幂等性: meta.key = UUIDv7 (edge 生成), center 端 ON CONFLICT DO NOTHING。
// ponytail: 参数多都是一条 usage 事件的独立字段，包成 struct 只是把参数搬个地方。
#[allow(clippy::too_many_arguments)]
pub fn settle_event(
    counts: TokenCounts,
    group: &str,
    group_ratio: f64,
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
    // 查找模型价格 (默认 0 = 免费)；默认价 group_multiplier 固定 1.0，
    // 组倍率完全靠调用方传入的 group_ratio。
    let price = price_table
        .lookup(public_model, group)
        .unwrap_or(crate::pricing::ModelPrice {
            input: 0.0,
            output: 0.0,
            cache: 0.0,
            group_multiplier: 1.0,
        });

    let cost = price_of(counts, &price, group_ratio);

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
        first_token_ms,
        duration_ms,
        cost,
        status_code,
        error: error.map(|s| s.to_string()),
    }
}

/// 从**非流式**响应体 (完整 JSON) 解析 usage 计数。
///
/// 兼容两种形状：OpenAI (`prompt_tokens`/`completion_tokens`，cache 在
/// `prompt_tokens_details.cached_tokens` 或顶层 `cached_tokens`) 与
/// Claude (`input_tokens`/`output_tokens`，cache 在 `cache_read_input_tokens`)。
/// 体不是合法 JSON、缺 `usage` 字段或全零无 usage → 返回 `None`，
/// 由调用方走估算兜底。
pub fn extract_usage(body: &[u8]) -> Option<TokenCounts> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    let usage = v.get("usage")?;
    let get = |keys: [&str; 2]| {
        keys.iter()
            .find_map(|k| usage.get(*k).and_then(|x| x.as_u64()))
            .unwrap_or(0)
    };
    let counts = TokenCounts {
        prompt: get(["prompt_tokens", "input_tokens"]),
        completion: get(["completion_tokens", "output_tokens"]),
        cached: usage
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .or_else(|| usage.get("cached_tokens"))
            .or_else(|| usage.get("cache_read_input_tokens"))
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
    };
    // 全零 = 上游没给真实 usage，视作无 usage（让调用方估算兜底）。
    if counts.prompt == 0 && counts.completion == 0 {
        None
    } else {
        Some(counts)
    }
}
