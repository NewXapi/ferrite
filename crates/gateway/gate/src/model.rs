//! `model` —— gate 5：模型白名单（`token.allowed_models`） + 请求体 model/max_tokens 解析
//!
//! 另含 [`GroupModelGate`]：组级模型白名单（查 [`crate::snapshot::GroupSnapshot`]），
//! 与 token 级 [`ModelGate`] 平行的附加闸门。

use async_trait::async_trait;
use gateway_pipeline::ctx::BodySource;
use serde::Deserialize;

use super::chain::{Gate, GateCtx};
use super::error::Rejection;
use super::snapshot::SharedGroupSnapshot;

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

/// 组级模型门禁：按 `ctx.group` 查 [`GroupSnapshot`](crate::snapshot::GroupSnapshot)
/// 的组白名单做匹配。
///
/// 与 token 级 [`ModelGate`] 平行的**附加**闸门（挂载顺序：建议排在 ModelGate 之后，
/// 依赖其已把 `requested_model` 解析进 ctx，本 gate 自身不解析请求体）。
///
/// fail-open 策略——"未配置"不拦：
/// - `ctx.group` 缺失 / 组不在快照里 / 白名单为空 → 放行；
/// - 仅当组存在且白名单非空、且没有任何 pattern（含 `gpt-4*` 通配）命中时，
///   返回 [`Rejection::ModelNotAllowedForGroup`]。
pub struct GroupModelGate {
    groups: SharedGroupSnapshot,
}

impl GroupModelGate {
    pub fn new(groups: SharedGroupSnapshot) -> Self {
        Self { groups }
    }
}

#[async_trait]
impl Gate for GroupModelGate {
    fn name(&self) -> &'static str {
        "group_model"
    }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let model = ctx
            .requested_model
            .as_deref()
            .ok_or(Rejection::ModelNotSpecified)?;

        // 未带组 / 空组名 = 未配置组级门禁 → fail-open。
        let Some(group) = ctx.group.as_deref().filter(|g| !g.is_empty()) else {
            return Ok(());
        };
        let snapshot = self.groups.load();
        // 组不存在（未配置）→ fail-open；存在且空白名单同样不限模型。
        let Some(allowed) = snapshot.allowed_models(group) else {
            return Ok(());
        };
        if allowed.is_empty() || allowed.iter().any(|p| match_model(p, model)) {
            return Ok(());
        }
        Err(Rejection::ModelNotAllowedForGroup {
            model: model.into(),
            group: group.into(),
        })
    }
}
