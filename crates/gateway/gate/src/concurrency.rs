//! `concurrency` —— gate 7：post-dispatch 并发槽（每 channel Semaphore）
//!
//! 与其它 gate 不同：本 gate 在 dispatch 之后执行（需要选中的渠道才能占用）。
//! 在 pipeline 中按 post-dispatch 顺序注册。
//!
//! 索引口径是 `ChannelRecord.meta.key`（即 `RouteUnitRecord.channel_key`），
//! 与 catalog 快照、健康表、dispatch 限速表一致；渠道没有数字 id。
//!
//! ## permit 生命周期（RAII）
//!
//! [`Stage::handle`] 用 `try_acquire_owned` 抢槽：成功则把 permit 直接挂进
//! [`RequestCtx::drop_guards`]，本请求结束、ctx drop 时 permit 随之 drop、槽位
//! 归还。没有中间 hold_id 记账——不存在"句柄被丢弃导致槽位永久泄漏"的路径。
//! 槽满时立刻 `Err(StageError::RateLimited)`（映射 429），**从不阻塞排队**。
//!
//! 未注册渠道**不限并发**：直接放行 Continue（每渠道 warn 一次），绝不做
//! `or_insert_with(Semaphore::new(0))`——零容量信号量会让首个请求永久卡死。

use std::sync::Arc;

use async_trait::async_trait;
use dashmap::{DashMap, DashSet};
use gateway_pipeline::{RequestCtx, Stage, StageError, StageOutcome};
use tokio::sync::Semaphore;

/// 跨请求共享的并发槽状态。
#[derive(Default)]
pub struct ConcurrencyState {
    /// channel_key → Semaphore
    pub semaphores: DashMap<String, Arc<Semaphore>>,
}

pub struct ConcurrencyGate {
    state: Arc<ConcurrencyState>,
    /// 未注册渠道的 warn 去重标记：每渠道只提醒一次，避免每请求刷屏。
    warned_unregistered: DashSet<String>,
}

impl ConcurrencyGate {
    pub fn new(state: Arc<ConcurrencyState>) -> Self {
        Self {
            state,
            warned_unregistered: DashSet::new(),
        }
    }

    /// 注册 / 更新 channel 的并发上限。
    pub fn register_channel(&self, channel_key: &str, max_concurrency: u32) {
        self.state
            .semaphores
            .entry(channel_key.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(max_concurrency as usize)));
        // ponytail: 暂不处理"缩小"语义——已签发 permit 不能撤销；新值由下次 register 重建。
    }

    /// 当前可用槽数（`Semaphore::available_permits`）；未注册渠道返回 0。
    pub fn available_permits(&self, channel_key: &str) -> usize {
        self.state
            .semaphores
            .get(channel_key)
            .map(|s| s.available_permits())
            .unwrap_or(0)
    }
}

#[async_trait]
impl Stage for ConcurrencyGate {
    fn name(&self) -> &'static str {
        "concurrency"
    }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        // 1. 必须有 SelectedRoute 才能知道占哪个渠道的槽
        let channel_key = ctx
            .route
            .as_ref()
            .ok_or_else(|| {
                StageError::Internal(anyhow::anyhow!("concurrency gate requires SelectedRoute"))
            })?
            .unit
            .channel_key
            .clone();

        // 2. 未注册渠道：不限并发，直接放行（每渠道 warn 一次）。
        let Some(sem) = self.state.semaphores.get(&channel_key).map(|s| s.clone()) else {
            if self.warned_unregistered.insert(channel_key.clone()) {
                tracing::warn!(
                    channel_key = %channel_key,
                    "concurrency gate: channel not registered, passing through unlimited"
                );
            }
            return Ok(StageOutcome::Continue);
        };

        // 3. 同步抢槽：成功 → permit 作为 RAII 守卫挂进 ctx，随 ctx 在请求结束
        //    时 drop 归还槽位；槽满 → RateLimited（router 映射 429）。
        match sem.try_acquire_owned() {
            Ok(permit) => {
                ctx.drop_guards.push(Box::new(permit));
                Ok(StageOutcome::Continue)
            }
            Err(tokio::sync::TryAcquireError::NoPermits) => Err(StageError::RateLimited),
            // 本模块从不 close() 信号量，Closed 分支理论不可达；兜底报 Internal。
            Err(e) => Err(StageError::Internal(anyhow::anyhow!(
                "semaphore closed: {e}"
            ))),
        }
    }
}
