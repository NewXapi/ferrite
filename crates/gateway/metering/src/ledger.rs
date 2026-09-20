//! 内存账本 — 预扣/结算/释放的原子操作面。
//!
//! 参考: new-api open_billing_session.go (请求级计费会话) +
//! resolve_funding_source.go (钱包/订阅双资金源)。
//! V1: 单一钱包余额位; V2: 订阅窗口 (billing 域下发)。

use std::collections::HashMap;

// Shuttle 并发测试要求把同步原语换成 shuttle 的实现（见 tests/ledger_concurrency.rs）。
// 非测试/非 shuttle feature 编译下零影响：仍是 std 原语。
#[cfg(all(feature = "shuttle", test))]
use shuttle::sync::atomic::{AtomicU64, Ordering};
#[cfg(all(feature = "shuttle", test))]
use shuttle::sync::{Arc, Mutex};
#[cfg(not(all(feature = "shuttle", test)))]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(not(all(feature = "shuttle", test)))]
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

/// 预扣记录的值形状: (user_key, token_key, amount)。
type HoldRecord = (String, String, i64);

/// 内存账本实现 — V1: 单一钱包。
///
/// balances 与 holds 同守一把锁：prehold 的「检查→扣减→登记」、
/// settle/release 的「退额→移除 hold」都在单一临界区内完成，不存在
/// 中间态可见窗口（tests/ledger_concurrency.rs 的 Shuttle 测试证成此不变式）。
/// ponytail: 全局单锁够 V1——本账本尚未接生产链路且纯内存无 IO；trait
/// 注释的「DashMap + per-user 锁」是吞吐真成瓶颈时的升级路径。
pub struct MemoryLedger {
    state: Arc<Mutex<LedgerState>>,
    /// 全局 hold_id 计数器。
    next_id: AtomicU64,
}

struct LedgerState {
    /// user_key → token_key → 余额 (内部单位)。
    balances: HashMap<String, HashMap<String, i64>>,
    /// 预扣记录: hold_id → (user_key, token_key, amount)。
    holds: HashMap<u64, HoldRecord>,
}

impl MemoryLedger {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(LedgerState {
                balances: HashMap::new(),
                holds: HashMap::new(),
            })),
            next_id: AtomicU64::new(1),
        }
    }

    /// 设置余额 (测试/初始化用)。
    pub fn set_balance(&self, user_key: &str, token_key: &str, amount: i64) {
        let mut state = self.state.lock().unwrap();
        state
            .balances
            .entry(user_key.to_string())
            .or_default()
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
        let mut state = self.state.lock().unwrap();
        let balance = state
            .balances
            .entry(user_key.to_string())
            .or_default()
            .entry(token_key.to_string())
            .or_insert(0);

        if *balance < estimated {
            return Err(Insufficient {
                need: estimated,
                have: *balance,
            });
        }

        // 扣减与登记 hold 在同一临界区：不存在「已扣未登记」的可见窗口。
        *balance -= estimated;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        state
            .holds
            .insert(id, (user_key.to_string(), token_key.to_string(), estimated));

        Ok(Hold {
            id,
            amount: estimated,
            user_key: user_key.to_string(),
            token_key: token_key.to_string(),
        })
    }

    fn settle(&self, hold: &Hold, actual: i64) -> i64 {
        let mut state = self.state.lock().unwrap();

        // 先占住 hold：已结算/已释放的 hold 这里拿不到，直接 no-op（幂等）。
        if state.holds.remove(&hold.id).is_none() {
            return 0;
        }

        let diff = hold.amount - actual;
        let balance = state
            .balances
            .entry(hold.user_key.clone())
            .or_default()
            .entry(hold.token_key.clone())
            .or_insert(0);
        *balance += diff; // 退回差额 (或补扣)

        diff
    }

    fn release(&self, hold: &Hold) {
        let mut state = self.state.lock().unwrap();

        // 同上：hold 不在 ⇒ 已被 settle 或已释放，重复退额会凭空生钱。
        if state.holds.remove(&hold.id).is_none() {
            return;
        }

        let balance = state
            .balances
            .entry(hold.user_key.clone())
            .or_default()
            .entry(hold.token_key.clone())
            .or_insert(0);
        *balance += hold.amount;
    }
}

impl BalanceLedger for MemoryLedger {
    fn available(&self, user_key: &str, token_key: &str) -> i64 {
        let state = self.state.lock().unwrap();
        state
            .balances
            .get(user_key)
            .and_then(|m| m.get(token_key))
            .copied()
            .unwrap_or(0)
    }
}
