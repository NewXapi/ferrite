//! 渠道探活历史与日聚合 — sub2api channel_monitor 语义的移植。
//!
//! 数据流:
//! ```text
//! ops::jobs (channel_probe) 定时探活
//!   → monitor_history (每次探活一行: ok/degraded/failed/error + 延迟)
//!   → monitor 每日聚合 (水位线推进, 只到昨天)
//!   → 保留策略: history 1 天 / daily 30 天 (retention 模块执行)
//! ```
//! 注意: 这是**合成探活** (主动 ping), 与 dispatch 的**真实流量健康观测**
//! (health_observations) 互补 — 前者测"渠道死活", 后者测"当前质量"。

use store::StoreError;

/// 记录一次探活结果 (ops::probe 调用)。
/// TODO(#704): 探活执行 (请求渠道上的测试模型, 短超时) + history 写入。
pub async fn record_probe(_channel_key: &str, _model: &str, _ok: bool, _latency_ms: u32) -> Result<(), StoreError> {
    todo!("TODO(#704)")
}

/// 日聚合 (水位线推进到昨天为止, 幂等 upsert)。
/// TODO(#704): (channel, model, date) 的 ok/degraded/failed 计数 + 延迟和。
pub async fn daily_rollup() -> Result<(), StoreError> {
    todo!("TODO(#704)")
}

/// 可用率查询 (面板: 渠道健康页, 最近 7/15/30 天)。
pub async fn availability(_channel_key: &str, _days: u32) -> Result<serde_json::Value, StoreError> {
    todo!("TODO(#704)")
}
