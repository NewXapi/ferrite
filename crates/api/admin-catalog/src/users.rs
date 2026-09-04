//! 用户管理 — admin 面板写操作。

use store::StoreError;

/// 余额调整 (admin): delta 正负皆可, 结果写回 quota 并产审计事件。
///
/// 返回调整后的余额。
/// TODO(#423): 审计事件归属 — 复用 usage 域 (type=admin) 还是独立 audit 域?
/// 参考 new-api compose_log_info.go 的 log 类型分类。
pub async fn adjust_quota(_user_key: &str, _delta: i64) -> Result<i64, StoreError> {
    todo!("TODO(#423): 原子调整 + 审计事件")
}

/// 用户启停 (admin): status 1↔2; 启停会经由 identity 快照下发, edge 即时生效。
pub async fn set_status(_user_key: &str, _status: u8) -> Result<(), StoreError> {
    todo!("TODO(#428)")
}
