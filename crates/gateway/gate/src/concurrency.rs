//! `concurrency` —— gate 7：post-dispatch 并发槽（每 channel Semaphore）
//!
//! 与其它 gate 不同：本 gate 在 dispatch 之后执行（需要 channel_id 才能占用）。
//! 在 pipeline 中按 post-dispatch 顺序注册。

use async_trait::async_trait;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::Semaphore;
use std::sync::atomic::AtomicU64;
use gateway_pipeline::Stage;
use super::error::Rejection;

pub struct ConcurrencyState {
    /// channel_id → Semaphore
    pub semaphores: DashMap<i64, Arc<Semaphore>>,
    /// hold_id 原子自增
    pub next_hold_id: AtomicU64,
}

impl Default for ConcurrencyState {
    fn default() -> Self {
        Self {
            semaphores: DashMap::new(),
            next_hold_id: AtomicU64::new(1),
        }
    }
}

pub struct ConcurrencyGate {
    state: Arc<ConcurrencyState>,
}

impl ConcurrencyGate {
    pub fn new(state: Arc<ConcurrencyState>) -> Self {
        Self { state }
    }

    /// 注册 channel 的并发上限（来自 channel 配置）
    pub fn register_channel(&self, _channel_id: i64, _max_concurrency: u32) {
        // TODO: 创建 / 更新 Semaphore
        unimplemented!("ConcurrencyGate::register_channel")
    }

    /// 释放并发槽
    pub fn release(&self, _hold_id: u64) {
        // TODO: 找 hold 对应 channel + permit
        unimplemented!("ConcurrencyGate::release")
    }
}

#[async_trait]
impl Stage for ConcurrencyGate {
    fn name(&self) -> &'static str { "concurrency" }

    async fn handle(&self, _ctx: &mut gateway_pipeline::RequestCtx) -> Result<gateway_pipeline::StageOutcome, gateway_pipeline::StageError> {
        // TODO: 从 ctx.route.channel_id 拿 semaphore，占用，写 hold_id 到 ctx
        unimplemented!("ConcurrencyGate::handle")
    }
}
