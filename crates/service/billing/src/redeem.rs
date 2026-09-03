//! 兑换码 — 批量生成 / 单次核销。
//!
//! 参考: one-api redemptions + new-api store_redemption.go (行锁/CAS) +
//! purge_redemptions.go (过期清理 → ops::jobs)。

use store::StoreError;

/// 批量生成: N 条唯一码, 同 batch 标记; 返回明文列表 (仅此一次, 落库前哈希)。
/// TODO(#608): 生成 + 批量 INSERT + 明文导出 (CSV/文本)。
pub async fn generate(_quota: i64, _count: u32) -> Result<Vec<String>, StoreError> {
    todo!("TODO(#608)")
}

/// 核销: 单次有效 (行锁 CAS: 未核销→已核销), 事务内入账用户 quota。
/// 并发核销同一码 → 只有一个成功, 其余 StoreError::Conflict。
/// TODO(#608): UPDATE ... WHERE redeemed_by IS NULL RETURNING 模式。
pub async fn redeem(_code: &str, _user_key: &str) -> Result<i64, StoreError> {
    todo!("TODO(#608)")
}
