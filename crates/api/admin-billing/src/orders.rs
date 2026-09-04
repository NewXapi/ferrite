//! 订单状态机 — pending → paid|failed|refunded, 终态不可变。
//!
//! 参考: new-api topup_api.go + handle_payment_return.go,
//! sub2api payment_order (lifecycle/refund/source 元数据)。

use store::StoreError;

/// 状态迁移 (唯一合法路径):
/// ```text
/// pending --webhook(paid)--> paid  (触发入账: quota 入钱包 / 订阅生效)
/// pending --webhook(fail)--> failed
/// paid    --admin/refund--> refunded (v2, 需要双录)
/// ```
/// 其余迁移 = StoreError::Conflict。
pub fn transition(_current: &str, _next: &str) -> Result<(), StoreError> {
    todo!("TODO(#601): 状态机校验实现")
}

/// 回调入账 (paid 后): 事务内 = 订单终态 + 用户 quota 入账 + (订阅单) 订阅行创建。
/// 幂等由 idempotency_records 保证 (scope="payment-webhook", key=provider_txn_id)。
/// TODO(#604): 入账事务编排 (多表写单事务)。
pub async fn apply_payment(_order_key: &str) -> Result<(), StoreError> {
    todo!("TODO(#604)")
}
