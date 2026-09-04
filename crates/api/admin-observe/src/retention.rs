//! 留存与清理 — 分区 drop 优于 DELETE。
//!
//! 策略 (todo/07 文档):
//! - usage_logs: 按月分区, 保留 N 个月后 DROP PARTITION (瞬时, 不产生死元组);
//! - health_observations: 保留 30 天 (批量 DELETE + vacuum);
//! - idempotency_records: 过期行清理 (billing 幂等窗口);
//! - 探活: history 1 天 / daily 30 天 (monitor 模块)。
//!
//! wildtoken 教训: 清理绝不阻塞转发路径 — 全部挂 ops::jobs 后台批处理。

use store::StoreError;

/// 确保分区存在 (月初由 ops::jobs 预建下月分区)。
/// TODO(#705): 动态 DDL (CREATE TABLE usage_logs_YYYYMM PARTITION OF ...)。
pub async fn ensure_partitions(_up_to: chrono::DateTime<chrono::Utc>) -> Result<(), StoreError> {
    todo!("TODO(#705)")
}

/// 按策略执行清理 (ops::jobs 的 usage_cleanup 任务)。
/// TODO(#705): 各表保留期配置化 (options 表) + 批量删除限速。
pub async fn cleanup() -> Result<CleanupReport, StoreError> {
    todo!("TODO(#705)")
}

#[derive(Debug, Default, serde::Serialize)]
pub struct CleanupReport {
    pub partitions_dropped: u32,
    pub rows_deleted: u64,
}
