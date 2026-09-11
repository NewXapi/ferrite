//! # observe — 观测聚合域
//!
//! 单机平表架构下直接消费 `usage_logs` 与渠道探活记录，产出查询视图:
//!
//! | 模块 | 聚合物 | 参考 |
//! |------|--------|------|
//! | [`logs`]    | 请求用量日志查询、今日统计、消耗排行与时间桶趋势 | new-api store_usedata.go |
//! | [`monitor`] | 探活历史 + 日聚合 (monitor_rollup) | sub2api channel_monitor |
//!
//! 查询响应统一携带 as_of 新鲜度标记, 不伪造实时 (设计文档原则 7)。

pub mod logs;
pub mod monitor;

/// 查询响应统一携带新鲜度 (原则 7)。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Freshness {
    /// 数据截止时刻。
    pub as_of: chrono::DateTime<chrono::Utc>,
    /// 覆盖的节点/分区 (partial 提示)。
    pub partial: bool,
}
