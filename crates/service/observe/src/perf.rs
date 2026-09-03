//! 性能指标 — TTFT/延迟/成功率的时间桶聚合。
//!
//! 参考: new-api record_perf.go + store_perf_metrics.go (内存/Redis/SQL 三级
//! 在 new-api; 我们只有两级: edge 内存瞬时 → usage_logs → SQL 聚合)。
//! 产出 perf_metrics (bucket, model, group)。

use store::StoreError;

/// 聚合窗口 (源 usage_logs 的 first_token_ms/duration_ms/status_code)。
/// TODO(#703): 窗口任务挂 ops::jobs; 分位数 (p50/p95) 需要预排序桶还是采样?
/// 先记 sum/count + max, 分位二期 (需要 t-digest 或桶直方)。
pub async fn rollup_window(_cutoff: chrono::DateTime<chrono::Utc>) -> Result<(), StoreError> {
    todo!("TODO(#703)")
}

/// 模型卡片数据 (mock::models 的 tokens_24h/success_rate/latency_p50 来源)。
/// TODO(#703): 24h 窗口查询 + p50 近似 (bucket 中值)。
pub async fn model_card(_model: &str) -> Result<serde_json::Value, StoreError> {
    todo!("TODO(#703)")
}
