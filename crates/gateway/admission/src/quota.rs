//! 闸② 配额检查 — 余额 vs 预估成本。
//!
//! 参考: new-api billing (read_user_quota + pre-consume 语义),
//! sub2api user_platform_quota (每平台额度 — 中转站场景退化为单池)。
//!
//! 注意: 这里是**判断**不是**扣减** — 真正的预扣在 metering::Ledger (热路径
//! 旁路内存账本)。二者读的是同一份本地余额位; 分开的原因:
//! admission 回答"能不能过", metering 回答"实际花了多少"。

use crate::error::Rejection;
use contract::records::{TokenRecord, UserRecord};

/// 余额源 — admission/metering 共用的本地余额位视图。
///
/// 实现: apps/gateway 的内存账本 (结算后由 sync 上报 center 收敛)。
/// TODO(#304): token 级与 user 级余额的合并读 — unlimited_quota 跳过;
/// 两者都有限时取小者。
pub trait BalanceView: Send + Sync {
    fn available(&self, user_key: &str, token_key: &str) -> i64;
}

/// 闸②: 预估成本 vs 余额。
///
/// `estimated` 由 metering 的估算器产出 (请求体预扫), admission 只比较。
/// 预扣本身发生在通过后 (metering::Ledger::prehold), 失败需回滚由调用方保证。
pub fn check_quota(
    balance: &dyn BalanceView,
    token: &TokenRecord,
    user: &UserRecord,
    estimated: i64,
) -> Result<(), Rejection> {
    let available = balance.available(&user.meta.key, &token.meta.key);
    if !token.unlimited_quota && available < estimated {
        return Err(Rejection::InsufficientQuota { estimated, available });
    }
    Ok(())
}
