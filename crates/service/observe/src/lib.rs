//! # observe — 观测聚合域 (center)
//!
//! 原则: **edge 产生原始事件, center 只做聚合**。
//! usage_logs (分区原始表) 由 store 域持有; 本 crate 消费它产出四类视图:
//!
//! | 模块 | 聚合物 | 参考 |
//! |------|--------|------|
//! | [`hourly`]   | usage_hourly: (user, model, group, channel) × 小时 | new-api store_usedata.go |
//! | [`rankings`] | model_rankings: 周期份额 + 环比 | new-api rank_usage.go |
//! | [`perf`]     | perf_metrics: 请求数/成功率/TTFT/延迟 | new-api record_perf.go |
//! | [`monitor`]  | 探活历史 + 日聚合 (monitor_rollup) | sub2api channel_monitor |
//! | [`retention`]| 分区 drop / 批量清理 / vacuum | wildtoken (丢日志优于阻塞) |
//!
//! 聚合纪律: upsert 幂等 (重复上报不翻倍); 查询响应必须带 as_of 标记,
//! 不伪造实时 (设计文档原则 7)。

pub mod hourly;
pub mod monitor;
pub mod perf;
pub mod rankings;
pub mod retention;

/// 查询响应统一携带新鲜度 (原则 7)。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Freshness {
    /// 数据截止时刻。
    pub as_of: chrono::DateTime<chrono::Utc>,
    /// 覆盖的节点/分区 (partial 提示)。
    pub partial: bool,
}
