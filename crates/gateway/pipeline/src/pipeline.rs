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

use std::sync::Arc;
use http::Response;
use axum::body::Body;
use crate::ctx::{RequestCtx, StageOutcome};
use crate::stage::{Stage, StageError};

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
                StageOutcome::ShortCircuit(resp) => return Ok(resp),
                StageOutcome::Stream(stream) => return Ok(stream.into_response()),
            }
        }
        Err(StageError::Internal(anyhow::anyhow!(
            "pipeline ended without producing a response (stages={})",
            self.stages.len()
        )))
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}
