//! `graylist` —— gate 6：连续失败封禁（防爆破）

use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use dashmap::DashMap;
use sha2::{Digest, Sha256};

use super::chain::{Gate, GateCtx};
use super::error::Rejection;

/// 灰名单状态：每个 key 一个失败计数 + 一个临时封禁到期时间。
pub struct GrayListState {
    /// 失败次数 + 首次失败时间
    pub fail_count: DashMap<[u8; 32], FailEntry>,
    /// 当前是否处于封禁期
    pub blocked_until: DashMap<[u8; 32], Instant>,
}

impl Default for GrayListState {
    fn default() -> Self {
        Self {
            fail_count: DashMap::new(),
            blocked_until: DashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FailEntry {
    pub streak: u32,
    /// streak 起始时刻；超过此窗口未失败则清零
    pub window_start: Instant,
}

/// 触发封禁的连续失败阈值。
pub const FAIL_STREAK_THRESHOLD: u32 = 5;
/// 单次封禁时长。
pub const BLOCK_DURATION: Duration = Duration::from_secs(60);
/// 失败计数窗口（超此时间未失败则重置 streak）。
pub const STREAK_WINDOW: Duration = Duration::from_secs(300);

pub struct GrayListGate {
    state: Arc<ArcSwap<GrayListState>>,
}

impl GrayListGate {
    pub fn new(state: Arc<ArcSwap<GrayListState>>) -> Self {
        Self { state }
    }

    /// 记录一次结果（由 forward 完成后调用）。
    ///
    /// 成功：清零 streak。
    /// 失败：streak +1；超阈值则封禁 BLOCK_DURATION 秒。
    pub fn record(&self, hash: [u8; 32], success: bool) {
        let state = self.state.load();
        if success {
            state.fail_count.remove(&hash);
            state.blocked_until.remove(&hash);
            return;
        }
        let now = Instant::now();
        let mut entry = state.fail_count.entry(hash).or_insert(FailEntry {
            streak: 0,
            window_start: now,
        });
        if now.duration_since(entry.window_start) > STREAK_WINDOW {
            entry.streak = 0;
            entry.window_start = now;
        }
        entry.streak = entry.streak.saturating_add(1);
        if entry.streak >= FAIL_STREAK_THRESHOLD {
            state.blocked_until.insert(hash, now + BLOCK_DURATION);
            entry.streak = 0;
        }
    }
}

#[async_trait]
impl Gate for GrayListGate {
    fn name(&self) -> &'static str {
        "graylist"
    }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let hash = match ctx.token.as_ref() {
            Some(t) => t.id_hash,
            None => match ctx.raw_key.as_ref() {
                Some(k) => sha256(k),
                None => return Ok(()),
            },
        };

        let state = self.state.load();
        if let Some(until) = state.blocked_until.get(&hash)
            && Instant::now() < *until.value()
        {
            return Err(Rejection::Graylisted);
        }
        Ok(())
    }
}

fn sha256(input: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let out = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}
