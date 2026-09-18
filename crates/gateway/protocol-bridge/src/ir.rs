//! Provider-neutral 中间表示（IR）—— 两跳转换的中枢。
//!
//! 所有格式 codec 只与本模块互转：`decode(某格式 JSON) → IR → encode(另一格式 JSON)`，
//! N 种格式互转只要 N 个 codec 而非 N×M。
//!
//! ## 逃逸口（round-trip 无损）
//!
//! - 未知 block 类型经 [`ContentBlock::Unknown`] 原样保留（自定义 serde：
//!   反序列化时未知 `type` 整体落入 `raw`，序列化时 `raw` 原样吐出，不重新打 tag）；
//! - 未知顶层/嵌套字段经 [`LlmRequest::extra`] / [`ToolDef::extra`] / [`SamplingParams::extra`]
//!   （`#[serde(flatten)]`）保留。
//!
//! 多模态（image/audio/file）不建模，一律落 [`ContentBlock::Unknown`]。

use serde::de;
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeMap};
use serde_json::{Map, Value};

/// 对话角色（非 tagged，序列化为 `"user"` / `"assistant"` / `"tool"`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    Tool,
}

/// 系统提示文本块。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
}

/// 内容块：按 `type` 标签区分的多态枚举。
///
/// 已知变体序列化为 `{"type":"text","text":"..."}` 形态；未知类型经 [`Self::Unknown`]
/// 原样保留（见模块级文档）。
#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
    Thinking {
        text: String,
    },
    Refusal {
        text: String,
    },
    /// 未知 block 类型的逃逸口：`raw` 保留原始 JSON 整体，round-trip 无损。
    Unknown {
        raw: Value,
    },
}

impl Serialize for ContentBlock {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            // 已知变体走 tagged 形态：{"type":"<snake_case>", ...}
            ContentBlock::Text { text } => {
                let mut m = serializer.serialize_map(None)?;
                m.serialize_entry("type", "text")?;
                m.serialize_entry("text", text)?;
                m.end()
            }
            ContentBlock::ToolUse { id, name, input } => {
                let mut m = serializer.serialize_map(None)?;
                m.serialize_entry("type", "tool_use")?;
                m.serialize_entry("id", id)?;
                m.serialize_entry("name", name)?;
                m.serialize_entry("input", input)?;
                m.end()
            }
            ContentBlock::ToolResult {
                tool_use_id,
                content,
            } => {
                let mut m = serializer.serialize_map(None)?;
                m.serialize_entry("type", "tool_result")?;
                m.serialize_entry("tool_use_id", tool_use_id)?;
                m.serialize_entry("content", content)?;
                m.end()
            }
            ContentBlock::Thinking { text } => {
                let mut m = serializer.serialize_map(None)?;
                m.serialize_entry("type", "thinking")?;
                m.serialize_entry("text", text)?;
                m.end()
            }
            ContentBlock::Refusal { text } => {
                let mut m = serializer.serialize_map(None)?;
                m.serialize_entry("type", "refusal")?;
                m.serialize_entry("text", text)?;
                m.end()
            }
            // 逃逸口：原样吐出，不重新打 tag —— 保证未知类型 round-trip 无损。
            ContentBlock::Unknown { raw } => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ContentBlock {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// 仅用于已知变体的 tagged 反序列化；未知类型在外层兜底进 `Unknown`。
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        enum Known {
            Text {
                text: String,
            },
            ToolUse {
                id: String,
                name: String,
                input: Value,
            },
            ToolResult {
                tool_use_id: String,
                content: String,
            },
            Thinking {
                text: String,
            },
            Refusal {
                text: String,
            },
        }

        impl From<Known> for ContentBlock {
            fn from(k: Known) -> Self {
                match k {
                    Known::Text { text } => ContentBlock::Text { text },
                    Known::ToolUse { id, name, input } => ContentBlock::ToolUse { id, name, input },
                    Known::ToolResult {
                        tool_use_id,
                        content,
                    } => ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                    },
                    Known::Thinking { text } => ContentBlock::Thinking { text },
                    Known::Refusal { text } => ContentBlock::Refusal { text },
                }
            }
        }

        // ponytail: 先吃成 Value 再分派，比手写 Visitor 短得多；
        // 未知 type 直接整体落 Unknown，零拷贝转交。
        let value = Value::deserialize(deserializer)?;
        match value.get("type").and_then(Value::as_str) {
            Some("text" | "tool_use" | "tool_result" | "thinking" | "refusal") => {
                Known::deserialize(value)
                    .map(Into::into)
                    .map_err(de::Error::custom)
            }
            _ => Ok(ContentBlock::Unknown { raw: value }),
        }
    }
}

/// 单条消息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ContentBlock>,
}

/// 工具定义。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
    #[serde(default, flatten, skip_serializing_if = "Map::is_empty")]
    pub extra: Map<String, Value>,
}

/// 工具选择策略。`Raw` 是未知形状（如厂商特有结构）的逃逸口。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    /// 指定单个工具。
    Specific(String),
    /// 未知形状原样保留。
    Raw(Value),
}

/// 采样参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplingParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(default, flatten, skip_serializing_if = "Map::is_empty")]
    pub extra: Map<String, Value>,
}

/// provider-neutral LLM 请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system: Vec<TextBlock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingParams>,
    #[serde(default)]
    pub stream: bool,
    /// 未知顶层字段的逃逸口，`flatten` 直收直放。
    #[serde(default, flatten, skip_serializing_if = "Map::is_empty")]
    pub extra: Map<String, Value>,
}

/// 非流式响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmResponse {
    pub id: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<ContentBlock>,
    pub usage: Usage,
    pub stop_reason: StopReason,
}

/// 停止原因（非 tagged，序列化为 `"stop"` / `"length"` / ...）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Stop,
    Length,
    ContentFilter,
    ToolUse,
    Error,
    /// 未归类的厂商原因原样保留。
    Other(String),
}

/// token 用量。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u64>,
}

/// 流式事件（tagged：`{"type":"text_delta", ...}`）。
///
/// 内容类 delta 全部带 `index`——这是 G1（tool 碎片装配）修复的关键：
/// codec 按 index 归属同属一个 block 的分片，不再靠猜。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart {
        id: String,
        model: String,
    },
    TextDelta {
        index: u32,
        text: String,
    },
    ToolCallStart {
        index: u32,
        id: String,
        name: String,
    },
    ToolCallDelta {
        index: u32,
        arguments_delta: String,
    },
    ThinkingDelta {
        index: u32,
        text: String,
    },
    MessageDelta {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<Usage>,
    },
    MessageStop {
        reason: StopReason,
    },
    Error {
        message: String,
    },
}
