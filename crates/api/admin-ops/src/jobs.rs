//! 系统任务 runner — 认领/心跳/执行/结果。
//!
//! 任务类型 (首批, 对齐 new-api schedule_tasks.go 注册表):
//! channel_probe (探活) / usage_cleanup (observe::retention) /
//! sub_reset (billing::subscriptions) / hourly_rollup / observe_backfill。

use store::StoreError;

/// 认领一批任务 (多实例安全的 SQL 见 crate 文档)。
/// TODO(#800): SKIP LOCKED 认领 + lease 续租 + 过期回收的完整实现。
pub async fn claim(_worker_id: &str, _limit: u32) -> Result<Vec<Job>, StoreError> {
    todo!("TODO(#800)")
}

/// 任务句柄。
pub struct Job {
    pub key: uuid::Uuid,
    pub job_type: String,
    pub payload: serde_json::Value,
}

/// 任务处理器 — 各域注册自己的实现 (observe/billing/probe)。
pub trait JobHandler: Send + Sync {
    fn job_type(&self) -> &'static str;
    fn run(&self, job: &Job) -> impl Future<Output = Result<serde_json::Value, StoreError>> + Send;
}

/// runner 主循环 — apps/console 启动时拉起。
/// 循环: claim → (心跳并发执行) → 成功写 result / 失败 attempts+1 → 重回 pending。
/// TODO(#800): 并发度参数 + attempts 上限 → failed 终态 + 告警 (notify)。
pub async fn run_loop<H: JobHandler>(_handlers: Vec<H>) {
    todo!("TODO(#800): runner 主循环")
}
