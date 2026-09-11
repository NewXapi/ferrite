//! `stage` — DispatchStage: 把调度器接入 pipeline
//!
//! pipeline 热路径第 2 步: 从 catalog 快照 (Dispatcher) 选出候选写入 ctx。
//! 失败映射: NoCandidate → NoRoute (404), SnapshotNotReady → NotReady (503),
//! RateLimited → 也走 NoRoute 上层 (429 语义由 retry 循环的 RateLimited 处理)。

use crate::{Dispatch, DispatchError};
use async_trait::async_trait;
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
                // 候选本身就是 SelectedRoute：secret / upstream_model /
                // provider_type / settings 全部随之进入 ctx，forward 不再自造。
                let channel_key = candidate.unit.channel_key.clone();
                ctx.route = Some(candidate);
                // 渠道归因：key 来自 unit；展示名回查 Dispatcher 快照。
                // SelectedRoute 不携带名字（不改 dispatch 的类型），查不到
                // （快照刚热更移除该渠道等竞态）就降级为只带 key。
                let channel_name = self.dispatch.channel_name(&channel_key);
                if channel_name.is_none() {
                    tracing::debug!(
                        channel_key = %channel_key,
                        "channel name not found in dispatch snapshot; usage attribution carries key only"
                    );
                }
                ctx.selected_channel_key = Some(channel_key);
                ctx.selected_channel_name = channel_name;
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
