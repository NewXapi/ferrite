//! Rust 侧的内部消息 / 请求类型。
//!
//! 这些是「数据」：可序列化（camelCase）、可裁剪、可渲染；不持有 runtime 状态。
//! 不依赖 `harness-tools` 的 `ToolInvocation` 等结构体，因为这里的 tool call 只
//! 保留 snapshot 阶段需要的最小字段（call_id + 工具引用 + 参数 JSON），具体
//! `ToolInvocation` 的强校验由 `harness-runtime` 在拿到完整 `AgentModelRequest` 后做。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::variables::VariableContext;

/// OpenAI 风格 role。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentModelRole {
    /// system 提示；裁剪规则上保留首条
    System,
    /// 用户消息
    User,
    /// 助手消息
    Assistant,
    /// 工具结果
    Tool,
}

impl AgentModelRole {
    /// 是否 system；用于裁剪时的「首条永保留」逻辑
    pub fn is_system(self) -> bool {
        matches!(self, Self::System)
    }
}

/// 一条消息的内容块；支持文本 / 工具调用 / 工具结果 三种。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AgentModelContentPart {
    /// 普通文本
    Text {
        /// 文本内容
        text: String,
    },
    /// 助手发出的工具调用
    ToolCall {
        /// provider / runtime 用于关联 tool result 的 call id
        call_id: String,
        /// 工具标识（`provider:native_name`）
        tool_id: String,
        /// 模型可见的工具名（OpenAI function name）
        model_alias: String,
        /// 参数 JSON（已解析，非字符串）
        arguments: Value,
    },
    /// 工具结果；`call_id` 必须对应同会话内一条 `ToolCall`
    ToolResult {
        /// 对应的 tool call id
        call_id: String,
        /// 工具 id（冗余存储，便于 runtime 不查 transcript 即可路由）
        tool_id: String,
        /// 工具输出文本（结构化结果由 runtime 解析）
        content: String,
        /// 工具调用是否失败
        #[serde(default)]
        is_error: bool,
    },
}

impl AgentModelContentPart {
    /// 估算文本长度（按字符数；token 估算由调用方传入闭包）
    pub fn text_length(&self) -> usize {
        match self {
            Self::Text { text } => text.len(),
            Self::ToolCall { arguments, .. } => arguments.to_string().len(),
            Self::ToolResult { content, .. } => content.len(),
        }
    }

    /// 是否工具调用 / 工具结果；用于裁剪时的成组保留
    pub fn is_tool_payload(&self) -> bool {
        matches!(self, Self::ToolCall { .. } | Self::ToolResult { .. })
    }
}

/// 一条模型消息；可包含多个 `AgentModelContentPart`。
///
/// 简化约定：system / user / tool 消息通常只包含 `Text`；assistant 消息可能
/// 同时包含 `Text` + `ToolCall`；tool 消息通常只包含 `ToolResult`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelMessage {
    /// role
    pub role: AgentModelRole,
    /// 多 part 内容；空数组视为非法，构造时保证至少 1 项
    pub parts: Vec<AgentModelContentPart>,
    /// 可选推理 / provider metadata（不计入 token 估算）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl AgentModelMessage {
    /// 构造 system / user 单文本消息
    pub fn text(role: AgentModelRole, text: impl Into<String>) -> Self {
        Self {
            role,
            parts: vec![AgentModelContentPart::Text { text: text.into() }],
            name: None,
        }
    }

    /// 构造 tool 消息
    pub fn tool_result(
        call_id: impl Into<String>,
        tool_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role: AgentModelRole::Tool,
            parts: vec![AgentModelContentPart::ToolResult {
                call_id: call_id.into(),
                tool_id: tool_id.into(),
                content: content.into(),
                is_error: false,
            }],
            name: None,
        }
    }

    /// 构造 assistant tool-call 消息
    pub fn assistant_tool_call(
        call_id: impl Into<String>,
        tool_id: impl Into<String>,
        model_alias: impl Into<String>,
        arguments: Value,
    ) -> Self {
        Self {
            role: AgentModelRole::Assistant,
            parts: vec![AgentModelContentPart::ToolCall {
                call_id: call_id.into(),
                tool_id: tool_id.into(),
                model_alias: model_alias.into(),
                arguments,
            }],
            name: None,
        }
    }

    /// 拼接全部文本（不含 tool call / result 结构）便于 token 估算
    pub fn text_payload(&self) -> String {
        let mut buf = String::new();
        for part in &self.parts {
            if let AgentModelContentPart::Text { text } = part {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(text);
            }
        }
        buf
    }

    /// 是否包含 tool-call / tool-result；用于裁剪时的成组原子性
    pub fn has_tool_payload(&self) -> bool {
        self.parts
            .iter()
            .any(AgentModelContentPart::is_tool_payload)
    }
}

/// 完整模型请求；可序列化为 OpenAI-compatible JSON（实际编码由 runtime 负责）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelRequest {
    /// 可选 system 提示（在 `messages` 之前注入）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// 历史 / 当前消息
    pub messages: Vec<AgentModelMessage>,
    /// 已编译的工具描述（来自前端 snapshot）；runtime 拿到后转换为 OpenAI tools[]
    #[serde(default)]
    pub tools: Vec<AgentModelToolSpec>,
    /// 是否启用 thinking / reasoning；由 caller 透传，runtime 不解释
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Rust 侧的工具描述（snapshot 阶段）；runtime 把它转成
/// `harness_tools::AgentModelTool`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelToolSpec {
    /// 完整工具 id（`provider:native_name`）
    pub tool_id: String,
    /// 模型可见名（OpenAI `function.name`）
    pub model_alias: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// 工具描述
    pub description: Option<String>,
    /// JSON Schema
    pub input_schema: Value,
}

/// 前端拼好后给 Rust 端的输入；`render` 会把它展开成 `AgentModelRequest`。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptInput {
    /// system 提示（已被前端物化，无 `{{...}}`）
    #[serde(default)]
    pub system: Option<String>,
    /// 角色上下文（仅供 `expand_variables` 兜底用）
    #[serde(default)]
    pub character_name: Option<String>,
    /// 用户上下文（同上）
    #[serde(default)]
    pub user_name: Option<String>,
    /// 已经过前端宏展开的消息历史
    #[serde(default)]
    pub messages: Vec<AgentModelMessage>,
    /// 工具描述
    #[serde(default)]
    pub tools: Vec<AgentModelToolSpec>,
    /// token budget；裁剪时使用
    #[serde(default)]
    pub token_budget: Option<u32>,
}

impl PromptInput {
    /// 构造空输入
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 system
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// 追加消息
    pub fn push_message(mut self, message: AgentModelMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// 设置 token budget
    pub fn with_token_budget(mut self, budget: u32) -> Self {
        self.token_budget = Some(budget);
        self
    }

    /// 取出变量上下文（用于 `expand_variables`）
    pub fn variable_context(&self) -> VariableContext {
        VariableContext {
            character_name: self.character_name.clone().unwrap_or_default(),
            user_name: self.user_name.clone().unwrap_or_default(),
        }
    }
}
