//! 内存账本 — 预扣/结算/释放的原子操作面。
//!
//! 参考: new-api open_billing_session.go (请求级计费会话) +
//! resolve_funding_source.go (钱包/订阅双资金源)。
//! V1: 单一钱包余额位; V2: 订阅窗口 (billing 域下发)。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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
    fn prehold(
        &self,
        user_key: &str,
        token_key: &str,
        estimated: i64,
    ) -> Result<Hold, Insufficient>;
    /// 结算: 退回 (estimated - actual) 差额; actual > 预估时补扣。
    /// 返回净差额 (负 = 用户被补扣)。
    fn settle(&self, hold: &Hold, actual: i64) -> i64;
    /// 请求在预扣后、结算前失败 (连接中断等) → 全额释放。幂等。
    fn release(&self, hold: &Hold);
}

/// 余额位查询 — admission::quota::BalanceView 的实现侧。
pub trait BalanceLedger: Ledger {
    /// admission 读: available = quota - used - held。
    fn available(&self, user_key: &str, token_key: &str) -> i64;
}

#[derive(Debug, thiserror::Error)]
#[error("insufficient balance: need {need}, have {have}")]
pub struct Insufficient {
    pub need: i64,
    pub have: i64,
}

/// 内存账本实现 — V1: 单一钱包, 内存 HashMap + per-user Mutex。
pub struct MemoryLedger {
    /// user_key → token_key → 余额 (内部单位)。
    balances: Arc<Mutex<HashMap<String, HashMap<String, i64>>>>,
    /// 预扣记录: hold_id → (user_key, token_key, amount)。
    holds: Arc<Mutex<HashMap<u64, (String, String, i64)>>>,
    /// 全局 hold_id 计数器。
    next_id: AtomicU64,
}

impl MemoryLedger {
    pub fn new() -> Self {
        Self {
            balances: Arc::new(Mutex::new(HashMap::new())),
            holds: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
        }
    }

    /// 设置余额 (测试/初始化用)。
    pub fn set_balance(&self, user_key: &str, token_key: &str, amount: i64) {
        let mut balances = self.balances.lock().unwrap();
        balances
            .entry(user_key.to_string())
            .or_insert_with(HashMap::new)
            .insert(token_key.to_string(), amount);
    }
}

impl Default for MemoryLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger for MemoryLedger {
    fn prehold(
        &self,
        user_key: &str,
        token_key: &str,
        estimated: i64,
    ) -> Result<Hold, Insufficient> {
        let mut balances = self.balances.lock().unwrap();
        let user_balances = balances
            .entry(user_key.to_string())
            .or_insert_with(HashMap::new);
        let balance = user_balances.entry(token_key.to_string()).or_insert(0);

        if *balance < estimated {
            return Err(Insufficient {
                need: estimated,
                have: *balance,
            });
        }

        *balance -= estimated;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let mut holds = self.holds.lock().unwrap();
        holds.insert(id, (user_key.to_string(), token_key.to_string(), estimated));

        Ok(Hold {
            id,
            amount: estimated,
            user_key: user_key.to_string(),
            token_key: token_key.to_string(),
        })
    }

    fn settle(&self, hold: &Hold, actual: i64) -> i64 {
        let mut balances = self.balances.lock().unwrap();
        let user_balances = balances
            .entry(hold.user_key.clone())
            .or_insert_with(HashMap::new);
        let balance = user_balances
            .entry(hold.token_key.clone())
            .or_insert(0);

        let diff = hold.amount - actual;
        *balance += diff; // 退回差额 (或补扣)

        let mut holds = self.holds.lock().unwrap();
        holds.remove(&hold.id);

        diff
    }

    fn release(&self, hold: &Hold) {
        let mut balances = self.balances.lock().unwrap();
        let user_balances = balances
            .entry(hold.user_key.clone())
            .or_insert_with(HashMap::new);
        let balance = user_balances
            .entry(hold.token_key.clone())
            .or_insert(0);

        *balance += hold.amount;

        let mut holds = self.holds.lock().unwrap();
        holds.remove(&hold.id);
    }
}

impl BalanceLedger for MemoryLedger {
    fn available(&self, user_key: &str, token_key: &str) -> i64 {
        let balances = self.balances.lock().unwrap();
        balances
            .get(user_key)
            .and_then(|m| m.get(token_key))
            .copied()
            .unwrap_or(0)
    }
}