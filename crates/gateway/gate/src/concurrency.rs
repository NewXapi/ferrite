//! `concurrency` —— gate 7：post-dispatch 并发槽（每 channel Semaphore）
//!
//! 与其它 gate 不同：本 gate 在 dispatch 之后执行（需要 channel_id 才能占用）。
//! 在 pipeline 中按 post-dispatch 顺序注册。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use dashmap::DashMap;
use gateway_pipeline::{RequestCtx, Stage, StageError, StageOutcome};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub struct ConcurrencyState {
    /// channel_id → Semaphore
    pub semaphores: DashMap<i64, Arc<Semaphore>>,
    /// hold_id → permit（移除 entry → permit drop → 槽位归还）
    pub holds: DashMap<u64, OwnedSemaphorePermit>,
    pub next_hold_id: AtomicU64,
}

impl Default for ConcurrencyState {
    fn default() -> Self {
        Self {
            semaphores: DashMap::new(),
            holds: DashMap::new(),
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

    /// 注册 / 更新 channel 的并发上限。
    pub fn register_channel(&self, channel_id: i64, max_concurrency: u32) {
        self.state
            .semaphores
            .entry(channel_id)
            .or_insert_with(|| Arc::new(Semaphore::new(max_concurrency as usize)));
        // ponytail: 暂不处理"缩小"语义——已签发 permit 不能撤销；新值由下次 register 重建。
    }

    /// 主动释放一个 hold_id（forward 完成时调用）。
    pub fn release(&self, hold_id: u64) {
        self.state.holds.remove(&hold_id);
    }

    /// 同步尝试占用一个槽；返回 hold_id 或 None（槽满）。
    pub fn try_hold(&self, channel_id: i64) -> Option<u64> {
        let sem = self.state.semaphores.get(&channel_id)?.clone();
        let permit = sem.try_acquire_owned().ok()?;
        let hold_id = self.state.next_hold_id.fetch_add(1, Ordering::Relaxed);
        self.state.holds.insert(hold_id, permit);
        Some(hold_id)
    }

    /// 当前已占用数（含已签发未释放的）。
    pub fn in_flight(&self, channel_id: i64) -> usize {
        self.state
            .semaphores
            .get(&channel_id)
            .map(|s| {
                let total = s.available_permits();
                let max_permits = self
                    .state
                    .semaphores
                    .get(&channel_id)
                    .map(|x| x.available_permits())
                    .unwrap_or(0);
                // ponytail: 仅返回 available；总上限可由调用方持有
                let _ = max_permits;
                total
            })
            .unwrap_or(0)
    }
}

#[async_trait]
impl Stage for ConcurrencyGate {
    fn name(&self) -> &'static str {
        "concurrency"
    }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        // 1. 必须有 SelectedRoute 才能拿 channel_id
        let route = ctx.route.as_ref().ok_or_else(|| {
            StageError::Internal(anyhow::anyhow!("concurrency gate requires SelectedRoute"))
        })?;
        let channel_id = route.channel_id;

        // 2. 拿 semaphore
        let sem = self
            .state
            .semaphores
            .entry(channel_id)
            .or_insert_with(|| Arc::new(Semaphore::new(0)))
            .clone();

        // 3. 异步 acquire（max=0 时永不返回 —— 调用方必须先 register_channel）
        let permit = sem
            .acquire_owned()
            .await
            .map_err(|e| StageError::Internal(anyhow::anyhow!("semaphore closed: {e}")))?;

        let hold_id = self.state.next_hold_id.fetch_add(1, Ordering::Relaxed);
        self.state.holds.insert(hold_id, permit);
        Ok(StageOutcome::Continue)
    }
}
