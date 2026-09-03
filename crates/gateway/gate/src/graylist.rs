//! `graylist` —— gate 6：连续失败封禁（防爆破）

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::atomic::AtomicU32;
use super::chain::{Gate, GateCtx};
use super::error::Rejection;

/// 灰名单状态
pub struct GrayListState {
    pub fail_count: DashMap<[u8; 32], AtomicU32>,
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

pub struct GrayListGate {
    state: Arc<ArcSwap<GrayListState>>,
}

impl GrayListGate {
    pub fn new(state: Arc<ArcSwap<GrayListState>>) -> Self {
        Self { state }
    }

    /// 记录一次结果（由 forward 完成后调用）
    pub fn record(&self, _hash: [u8; 32], _success: bool) {
        // TODO: 计数 + 触发封禁
    }
}

#[async_trait]
impl Gate for GrayListGate {
    fn name(&self) -> &'static str { "graylist" }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let hash = ctx.token.as_ref()
            .map(|t| t.id_hash)
            .or_else(|| ctx.raw_key.as_ref().map(|k| sha256(k)));

        let Some(hash) = hash else { return Ok(()); };

        let state = self.state.load();
        if let Some(until) = state.blocked_until.get(&hash) {
            if Instant::now() < *until.value() {
                return Err(Rejection::Graylisted);
            }
        }
        Ok(())
    }
}

fn sha256(_input: &str) -> [u8; 32] {
    [0u8; 32]
}
