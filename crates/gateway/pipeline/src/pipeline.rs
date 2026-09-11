//! `pipeline` —— 链式 stage 编排器
//!
//! 使用方式：
//! ```ignore
//! let pipe = Pipeline::new()
//!     .push(AdmissionStage::new(...))
//!     .push(DispatchStage::new(...))
//!     .push(ForwardStage::new(...))
//!     .push(ProtocolBridgeStage::new(...));
//! let resp = pipe.run(ctx).await?;
//! ```

use crate::ctx::{RequestCtx, RouteAttribution};
use crate::stage::{Stage, StageError, StageOutcome};
use axum::body::Body;
use http::Response;
use std::sync::Arc;

/// 链式 stage 容器
pub struct Pipeline {
    stages: Vec<Arc<dyn Stage>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { stages: vec![] }
    }

    /// 链式注册一个 stage
    pub fn push<S: Stage + 'static>(mut self, s: S) -> Self {
        self.stages.push(Arc::new(s));
        self
    }

    pub fn len(&self) -> usize {
        self.stages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// 顺序执行所有 stage
    ///
    /// 遇 `ShortCircuit` / `Stream` / `Err` 立即返回。
    ///
    /// 返回 `Ok` 响应前，若 Dispatch 已选中路由，则把渠道归因打包成
    /// [`RouteAttribution`] 塞进响应 extensions（非流式与流式两个出口都要覆盖，
    /// 见 [`attach_attribution`]）。
    pub async fn run(&self, mut ctx: RequestCtx) -> Result<Response<Body>, StageError> {
        for (i, stage) in self.stages.iter().enumerate() {
            tracing::trace!(
                stage = stage.name(),
                idx = i,
                req_id = %ctx.request.request_id,
                "pipeline::run stage start"
            );
            let outcome = stage.handle(&mut ctx).await?;
            match outcome {
                StageOutcome::Continue => continue,
                StageOutcome::ShortCircuit(resp) => {
                    return Ok(attach_attribution(ctx, resp));
                }
                StageOutcome::Stream(stream) => {
                    return Ok(attach_attribution(ctx, stream.into_response()));
                }
            }
        }
        Err(StageError::Internal(anyhow::anyhow!(
            "pipeline ended without producing a response (stages={})",
            self.stages.len()
        )))
    }
}

/// 把 Dispatch 选中的渠道归因从 ctx 打包进响应 extensions。
///
/// ctx 在 `run` 内被按值消费：流式分支的 body 交给 SsePipe 后 ctx 无法再从
/// handler 拿回（`Pipeline::run` 返回的只有响应），所以打包必须在 run 返回前
/// 完成——这是流式请求也能带上渠道归因的关键。链外中间件（apps/api 的
/// usage_middleware）再从 `response.extensions()` 读走并落库。
///
/// 路由未选中（dispatch 前短路，如 401/配额拒绝）时不插入：此时没有"实际
/// 命中的渠道"，usage 侧按缺失处理。
fn attach_attribution(ctx: RequestCtx, mut resp: Response<Body>) -> Response<Body> {
    if let Some(route) = ctx.route.as_ref() {
        resp.extensions_mut().insert(RouteAttribution {
            // selected_channel_key 由 DispatchStage 与 route 同步写入；万一
            // 缺席（自定义 stage 只写 route），回落到 route 自带的渠道键。
            channel_key: ctx
                .selected_channel_key
                .clone()
                .unwrap_or_else(|| route.unit.channel_key.clone()),
            channel_name: ctx.selected_channel_name.clone().unwrap_or_default(),
            model: ctx.requested_model.clone().unwrap_or_default(),
        });
    }
    resp
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}
