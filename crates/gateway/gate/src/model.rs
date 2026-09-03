//! `model` —— gate 5：模型白名单（`token.allowed_models`）

use async_trait::async_trait;
use super::chain::{Gate, GateCtx};
use super::error::Rejection;

pub struct ModelGate;

#[async_trait]
impl Gate for ModelGate {
    fn name(&self) -> &'static str { "model" }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let token = ctx.token.as_ref().ok_or(Rejection::AuthSkipped)?;

        // 1. 必须有 model 字段（由调用方在调用本 gate 前解析）
        let model = ctx.requested_model.as_deref()
            .ok_or(Rejection::ModelNotSpecified)?;

        // 2. 查白名单
        if let Some(allowed) = &token.allowed_models {
            if !allowed.iter().any(|m| match_model(m, model)) {
                return Err(Rejection::ModelForbidden { model: model.into() });
            }
        }
        Ok(())
    }
}

/// 模型名匹配规则：
/// - 精确匹配
/// - 通配符 `*`（如 `gpt-4*` 匹配 `gpt-4o` / `gpt-4-turbo`）
/// - 别名映射（在配置层处理，本函数只做通配符）
fn match_model(pattern: &str, model: &str) -> bool {
    if pattern == model { return true; }
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        return model.starts_with(prefix);
    }
    false
}
