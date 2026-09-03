//! `chain` —— `GateChain`：把多个 gate 串成单一 stage
//!
//! 外部看是单一 stage；内部按顺序执行 gates，任一失败立即返回。

use std::sync::Arc;
use async_trait::async_trait;
use gateway_pipeline::{Stage, RequestCtx, StageOutcome};
use crate::error::Rejection;
use crate::{TokenInfo, UserInfo};

/// Gate 检查上下文（gate 间共享单次请求的中间产物）
pub struct GateCtx {
    /// 不可变请求入参
    pub request_meta: gateway_pipeline::RequestMeta,
    /// auth 阶段提取的明文 key
    pub raw_key: Option<String>,
    /// auth 通过后填充
    pub token: Option<TokenInfo>,
    /// state 通过后填充
    pub user: Option<UserInfo>,
    /// 生效分组（token.group 优先，user.group 次之）
    pub group: Option<String>,
    /// quota 阶段计算的成本
    pub estimated_cost: Option<i64>,
    /// model 阶段从请求体解析的 model
    pub requested_model: Option<String>,
    /// model 阶段从请求体解析的 max_tokens
    pub requested_max_tokens: Option<u32>,
}

/// Gate 通过后的最终产出
pub struct Gated {
    pub token: TokenInfo,
    pub user: UserInfo,
    pub group: String,
}

/// Gate trait
#[async_trait]
pub trait Gate: Send + Sync {
    fn name(&self) -> &'static str;
    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection>;
}

/// 多 gate 串行编排
pub struct GateChain {
    gates: Vec<Arc<dyn Gate>>,
}

impl GateChain {
    pub fn new() -> Self {
        Self { gates: vec![] }
    }

    pub fn push<G: Gate + 'static>(mut self, g: G) -> Self {
        self.gates.push(Arc::new(g));
        self
    }

    pub fn len(&self) -> usize {
        self.gates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gates.is_empty()
    }
}

impl Default for GateChain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for GateChain {
    fn name(&self) -> &'static str { "gate" }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, gateway_pipeline::StageError> {
        let mut gate_ctx = GateCtx {
            request_meta: ctx.request.clone(),
            raw_key: None,
            token: None,
            user: None,
            group: None,
            estimated_cost: None,
            requested_model: None,
            requested_max_tokens: None,
        };

        for gate in &self.gates {
            if let Err(rej) = gate.check(&mut gate_ctx).await {
                return Ok(StageOutcome::ShortCircuit(
                    crate::error::rejection_to_response(rej)
                ));
            }
        }

        // 全部通过：把 token/user/group 提升到 RequestCtx
        if let Some(token) = gate_ctx.token {
            ctx.token = Some(gateway_pipeline::TokenInfo {
                id: token.id,
                group: token.group.clone(),
                enabled: token.enabled,
                allowed_models: token.allowed_models.clone(),
                auth_version: token.auth_version,
            });
        }
        Ok(StageOutcome::Continue)
    }
}
