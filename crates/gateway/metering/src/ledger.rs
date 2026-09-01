//! 内存账本 — 预扣/结算/释放的原子操作面。
//!
//! 参考: new-api open_billing_session.go (请求级计费会话) +
//! resolve_funding_source.go (钱包/订阅双资金源)。
//! V1: 单一钱包余额位; V2: 订阅窗口 (billing 域下发)。

/// 预扣凭据 — admission 返回的 hold_id 即由本模块发放。
#[derive(Debug, Clone)]
pub struct Hold {
    pub id: u64,
    /// 预扣额度 (内部计费单位)。
    pub amount: i64,
    pub user_key: String,
    pub token_key: String,
}

/// 内存账本 trait: prehold/settle/release 必须原子 (DashMap + per-user 锁)。
pub trait Ledger: Send + Sync {
    /// 预扣: 余额位 -= estimated; 不足 → Insufficient。
    fn prehold(&self, user_key: &str, token_key: &str, estimated: i64) -> Result<Hold, Insufficient>;
    /// 结算: 退回 (estimated - actual) 差额; actual > 预估时补扣。
    /// 返回净差额 (负 = 用户被补扣)。
    fn settle(&self, hold: &Hold, actual: i64) -> i64;
    /// 请求在预扣后、结算前失败 (连接中断等) → 全额释放。幂等。
    fn release(&self, hold: &Hold);
}

#[derive(Debug, thiserror::Error)]
#[error("insufficient balance: need {need}, have {have}")]
pub struct Insufficient {
    pub need: i64,
    pub have: i64,
}

/// 余额位查询 — admission::quota::BalanceView 的实现侧。
/// TODO(#330): hold_id 生成 (节点内 u64 计数器) + per-user Mutex 分片设计。
pub trait BalanceLedger: Ledger {
    /// admission 读: available = quota - used - held。
    fn available(&self, user_key: &str, token_key: &str) -> i64;
}
