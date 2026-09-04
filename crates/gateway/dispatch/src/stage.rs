//! `stage` — DispatchStage: 把调度器接入 pipeline
//!
//! pipeline 热路径第 2 步: 从 catalog 快照 (Dispatcher) 选出候选写入 ctx。
//! 失败映射: NoCandidate → NoRoute (404), SnapshotNotReady → NotReady (503),
//! RateLimited → 也走 NoRoute 上层 (429 语义由 retry 循环的 RateLimited 处理)。

use crate::{Dispatch, DispatchError};
use async_trait::async_trait;
use gateway_pipeline::ctx::SelectedRoute;
use gateway_pipeline::{RequestCtx, Stage, StageError, StageOutcome};
use std::sync::Arc;

/// 调度 stage — 输入已准入请求 (RequestCtx), 输出选中路由写入 ctx.route。
pub struct DispatchStage {
    dispatch: Arc<dyn Dispatch>,
}

impl DispatchStage {
    pub fn new(dispatch: Arc<dyn Dispatch>) -> Self {
        Self { dispatch }
    }
}

#[async_trait]
impl Stage for DispatchStage {
    fn name(&self) -> &'static str {
        "dispatch"
    }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        let group = ctx
            .token
            .as_ref()
            .map(|t| t.group.clone())
            .unwrap_or_default();
        // 模型名由 gate::model 从请求体解析后经 GateChain 提升到 ctx。
        let public_model = ctx.requested_model.clone().unwrap_or_default();
        match self.dispatch.select(&group, &public_model, &[]) {
            Ok(candidate) => {
                ctx.route = Some(SelectedRoute {
                    channel_id: 0, // forward 组装时从 candidate 取, 这里仅标记已选
                    api_type: 0,
                    base_url: candidate.base_url.clone(),
                });
                Ok(StageOutcome::Continue)
            }
            Err(DispatchError::SnapshotNotReady) => Err(StageError::NotReady),
            Err(
                DispatchError::NoCandidate { .. }
                | DispatchError::RateLimited { .. }
                | DispatchError::RetriesExhausted { .. },
            ) => Err(StageError::NoRoute),
        }
    }
}
