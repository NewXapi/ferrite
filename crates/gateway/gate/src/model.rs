//! `model` —— gate 5：模型白名单（`token.allowed_models`） + 请求体 model/max_tokens 解析

use async_trait::async_trait;
use gateway_pipeline::ctx::BodySource;
use serde::Deserialize;

use super::chain::{Gate, GateCtx};
use super::error::Rejection;

pub struct ModelGate;

#[async_trait]
impl Gate for ModelGate {
    fn name(&self) -> &'static str {
        "model"
    }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        // 0. 从请求体解析 model + max_tokens（首次进入 gate 时）
        if ctx.requested_model.is_none() {
            let body = ctx.request_meta.body.clone();
            parse_body_model_into(&body, ctx)?;
        }

        let token = ctx.token.as_ref().ok_or(Rejection::AuthSkipped)?;
        let model = ctx
            .requested_model
            .as_deref()
            .ok_or(Rejection::ModelNotSpecified)?;

        // 1. 白名单匹配
        if let Some(allowed) = &token.allowed_models
            && !allowed.is_empty()
            && !allowed.iter().any(|m| match_model(m, model))
        {
            return Err(Rejection::ModelForbidden {
                model: model.into(),
            });
        }
        Ok(())
    }
}

/// 解析 body 的 model 字段 + max_tokens。OpenAI / Anthropic 通用字段名。
#[derive(Debug, Deserialize)]
pub struct BodyModel {
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    /// Anthropic 用 max_completion_tokens 等变体
    #[serde(default)]
    pub max_completion_tokens: Option<u32>,
}

/// 共享 body 解析入口（quota / model 都用）。
pub fn parse_body_model_into(body: &BodySource, ctx: &mut GateCtx) -> Result<(), Rejection> {
    let bytes = match body {
        BodySource::InMemory(b) => b.as_ref(),
        BodySource::OnDisk { .. } => return Ok(()), // 暂不解析落盘 body
    };
    let bm: BodyModel = match serde_json::from_slice(bytes) {
        Ok(b) => b,
        Err(_) => return Ok(()), // 非 JSON / 解析失败 → 跳过
    };
    if let Some(m) = bm.model {
        ctx.requested_model = Some(m);
    }
    ctx.requested_max_tokens = bm.max_tokens.or(bm.max_completion_tokens);
    Ok(())
}

/// 模型名匹配：精确 / 通配符 `*`（如 `gpt-4*` 匹配 `gpt-4o`）。
pub fn match_model(pattern: &str, model: &str) -> bool {
    if pattern == model {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return model.starts_with(prefix);
    }
    false
}
