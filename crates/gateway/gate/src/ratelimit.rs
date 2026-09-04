//! `ratelimit` —— gate 4：速率限流（per-key / per-channel RPM 滑动窗口）
//!
//! 算法：每个 `(scope, key)` 一条 token bucket（按秒粒度分桶），窗口内累计超过
//! 限额则拒绝。ponytail: 内存单进程版，不分片；后续接多实例时改为 token bucket 中心化。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use parking_lot::Mutex;

use super::TokenInfo;
use super::chain::{Gate, GateCtx};
use super::error::Rejection;

/// 限流维度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimitScope {
    PerKey,     // 按 token id
    PerChannel, // 按 channel id（post-dispatch 时用）
    PerGroup,   // 按用户组
}

/// 速率限流器（sliding window by second buckets）。
///
/// 内部每个 `(scope, key)` 维护一个 [`TokenBucket`]。
pub struct RateLimiter {
    inner: Mutex<HashMap<(LimitScope, i64), TokenBucket>>,
    /// 默认上限：N 次请求 / window_secs 秒。简单起见目前所有 scope 共用同一对值。
    pub max_requests: u32,
    pub window_secs: u32,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u32) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max_requests,
            window_secs,
        }
    }

    /// 尝试获取一次配额。true = 允许，false = 超限。
    pub fn try_acquire(&self, scope: LimitScope, key: i64) -> bool {
        let now = now_secs();
        let mut map = self.inner.lock();
        let bucket = map.entry((scope, key)).or_default();
        bucket.try_acquire(now, self.max_requests, self.window_secs)
    }
}

/// 单个 (scope, key) 的滑动窗口。
///
/// 算法：以"当前秒"为最新桶，往前扫 window_secs 个秒桶；累计 > max_requests 拒绝。
/// ponytail: HashMap<sec, count>，每次 acquire 顺手清理过期桶，O(window_secs)。
#[derive(Default)]
struct TokenBucket {
    /// second → 该秒内的请求数
    buckets: HashMap<u64, u32>,
}

impl TokenBucket {
    fn try_acquire(&mut self, now: u64, max: u32, window: u32) -> bool {
        // 清理 [now - window, now) 之外的桶
        let cutoff = now.saturating_sub(window as u64);
        self.buckets.retain(|&sec, _| sec >= cutoff);

        let count: u32 = self.buckets.values().copied().sum();
        if count >= max {
            return false;
        }
        *self.buckets.entry(now).or_insert(0) += 1;
        true
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct RateLimitGate {
    limiter: Arc<RateLimiter>,
}

impl RateLimitGate {
    pub fn new(limiter: Arc<RateLimiter>) -> Self {
        Self { limiter }
    }
}

#[async_trait]
impl Gate for RateLimitGate {
    fn name(&self) -> &'static str {
        "ratelimit"
    }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let token: &TokenInfo = ctx.token.as_ref().ok_or(Rejection::AuthSkipped)?;

        // per-key 限流
        if !self.limiter.try_acquire(LimitScope::PerKey, token.id) {
            return Err(Rejection::RateLimited);
        }

        // 可选：per-group（用 hash(user.group) -> i64）
        if let Some(user) = &ctx.user
            && !self
                .limiter
                .try_acquire(LimitScope::PerGroup, str_hash(&user.group))
        {
            return Err(Rejection::RateLimited);
        }

        Ok(())
    }
}

/// 稳定字符串 → i64（用于 group 名当 rate-limit key）。
fn str_hash(s: &str) -> i64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() as i64
}
