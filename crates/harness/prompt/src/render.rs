//! `PromptInput` → `AgentModelRequest`。
//!
//! 顺序固定：
//! 1. system（可选，前端已物化；这里再走一次 `expand_variables` 兜底）
//! 2. messages（已物化）
//! 3. tools（snapshot 阶段不变）
//!
//! 这一步**不**做截断 — 调用方先 `render`，再决定是否喂给
//! `truncate_history`（裁剪只对历史消息生效，system / 工具描述按 caller
//! 提供的 budget 处理）。

use thiserror::Error;

use crate::types::{AgentModelMessage, AgentModelRequest, PromptInput};
use crate::variables::expand_variables;

/// `render` 的错误。当前只有「消息为空」一种语义错误（system 缺失不算）。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderError {
    /// messages 为空且没有 system：没有可发给模型的内容
    #[error("render.empty: prompt has no messages and no system prompt")]
    Empty,
}

/// 把 `PromptInput` 渲染成 `AgentModelRequest`。
///
/// `token_budget` 暂不消费 — 截断交给 `truncate_history`，调用方决定何时
/// 触发（render 总是返回完整结构）。
pub fn render(input: &PromptInput) -> Result<AgentModelRequest, RenderError> {
    if input.system.is_none() && input.messages.is_empty() {
        return Err(RenderError::Empty);
    }

    let system = input
        .system
        .as_ref()
        .map(|raw| expand_variables(raw, &input.variable_context()));

    let messages = input
        .messages
        .iter()
        .map(|msg| materialize_message(msg, &input.variable_context()))
        .collect::<Vec<_>>();

    Ok(AgentModelRequest {
        system,
        messages,
        tools: input.tools.clone(),
        metadata: None,
    })
}

fn materialize_message(
    msg: &AgentModelMessage,
    ctx: &crate::variables::VariableContext,
) -> AgentModelMessage {
    let mut out = msg.clone();
    for part in &mut out.parts {
        if let crate::types::AgentModelContentPart::Text { text } = part {
            *text = expand_variables(text, ctx);
        }
    }
    out
}
