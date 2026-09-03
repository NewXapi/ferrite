//! 订阅生命周期 — 分配/到期降级/周期重置。
//!
//! 参考: new-api store_subscription.go + reset_subscription_cycle.go,
//! sub2api user_subscription (窗口语义) + subscription_service.go (L1 缓存)。
//! 我们无 Redis: 订阅校验进 identity 快照 (sync 下发), 到期由 ops::jobs 驱动。

use store::StoreError;

/// 购买/分配生效: 创建 user_subscriptions 行 (窗口起点 = now)。
/// 到期降级规则: 用户有多个活跃升级订阅时, 降级延迟到最后一个过期 (不叠加降级)。
/// TODO(#606): 分配/续期/撤销 + 活跃订阅判定。
pub async fn assign(_user_key: &str, _plan_key: &str) -> Result<(), StoreError> {
    todo!("TODO(#606)")
}

/// 周期重置 (ops::jobs 的 sub_reset 任务调用):
/// 幂等 — pre-consume 记录 (idempotency) 保证同窗口只重置一次。
/// TODO(#607): daily/weekly/monthly 窗口判定 + 重置写。
pub async fn reset_window(_user_key: &str, _window: &str) -> Result<(), StoreError> {
    todo!("TODO(#607)")
}

/// 到期扫描 (ops::jobs 定期): 过期订阅 status → suspended + 降级评估。
pub async fn expire_sweep() -> Result<(), StoreError> {
    todo!("TODO(#607): 批量到期处理")
}
