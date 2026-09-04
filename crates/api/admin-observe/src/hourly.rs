//! 小时聚合 — usage_logs → usage_hourly (upsert 幂等)。
//!
//! SQL 形状 (水位线推进, 对齐 sub2api watermark 模式):
//! ```sql
//! INSERT INTO usage_hourly (bucket_start, user_key, model, group_id, channel_key,
//!                           requests, prompt_tokens, completion_tokens, cost)
//! SELECT date_trunc('hour', created_at), user_key, public_model, '-', channel_key,
//!        count(*), sum(prompt_tokens), sum(completion_tokens), sum(cost)
//! FROM usage_logs
//! WHERE created_at > $watermark AND created_at <= $cutoff
//! GROUP BY 1, 2, 3, 4, 5
//! ON CONFLICT (bucket_start, user_key, model, channel_key) DO UPDATE
//! SET requests = usage_hourly.requests + EXCLUDED.requests, ...;
//! ```
//! 水位线只在成功后前进 (失败重放靠幂等 upsert 不翻倍)。

use store::StoreError;

/// 聚合一个窗口 (watermark, cutoff]; 返回新水位线。
/// TODO(#700): 窗口参数 + watermark 存储 (options 表) + 任务挂 ops::jobs。
pub async fn rollup_window(
    _cutoff: chrono::DateTime<chrono::Utc>,
) -> Result<chrono::DateTime<chrono::Utc>, StoreError> {
    todo!("TODO(#700)")
}

/// 面板查询: 用户/管理员的用量曲线 (带 Freshness)。
/// TODO(#700): 时间范围聚合查询 + 分组维度选择。
pub async fn query(
    _query: &contract::api::usage::UsageLogQuery,
) -> Result<serde_json::Value, StoreError> {
    todo!("TODO(#700)")
}
