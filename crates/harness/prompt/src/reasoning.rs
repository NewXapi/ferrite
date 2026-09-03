//! Reasoning 回灌：把模型推理回填到下一次 prompt。
//!
//! Ported from SillyTavern `public/scripts/reasoning.js` `ReasoningTemplate` 的
//! 数据部分；只搬运 *模板* / *包装* / *注入* 三件套，不搬任何 UI / 流式相关。
//!
//! ## 模板
//!
//! 模板只承载数据 —— `prefix` / `suffix` / `separator` 三段字符串，没有别的。
//! 默认 Think 风格：`prefix = "<think>"`、`suffix = "</think>"`、`separator = "\n"`。
//!
//! ## 包装
//!
//! `wrap_reasoning(text, template) -> String`：单条推理的纯字符串包装，
//! 形如 `prefix + text + suffix`。空文本照样包装（调用方负责过滤）。
//!
//! ## 注入
//!
//! `inject_reasoning(messages, reasoning_texts, template, max_additions) -> Vec<AgentModelMessage>`：
//! 把回填的若干条推理渲染成额外的 system 消息，**新→旧**、**空跳过**、
//! **不超过 max_additions**，再拼接到原 messages 末尾。**不修改原 vec**。
//!
//! ponytail: 数据层只关心「要插什么、按什么顺序、限几条」。具体塞到
//! `system` 字段还是 messages 中由 caller 决定（保留与 `render` 同样的边界：
//! crate 不假设调用方后续怎么用）。
//!
//! ## 设计边界
//!
//! - 不依赖 runtime / repository / HTTP。
//! - 不假设 `reasoning_texts` 来自哪个 provider（OpenAI reasoning_content、
//!   DeepSeek reasoning、Qwen thinking 等都按字符串传入）。
//! - 不裁剪、不展开宏 —— 调用方在合适时机再调 `truncate_history` / `render`。

use serde::{Deserialize, Serialize};

use crate::types::{AgentModelMessage, AgentModelRole};

/// SillyTavern `ReasoningTemplate` 的数据子集：仅 prefix / suffix / separator。
///
/// 不照搬 `auto_escape` / `use_prefix` / `use_suffix` 等 UI 开关 —— Rust 侧
/// 模板已经是物化数据，开关意义消失。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningTemplate {
    /// 推理块起始标签，例如 `<think>`
    pub prefix: String,
    /// 推理块结束标签，例如 `</think>`
    pub suffix: String,
    /// 多条推理拼接时的分隔符，默认换行
    pub separator: String,
}

impl Default for ReasoningTemplate {
    /// 默认 Think 风格模板。
    fn default() -> Self {
        Self::think_xml()
    }
}

impl ReasoningTemplate {
    /// 构造默认 Think 模板（`<think>...</think>`，多条间用 `\n` 分隔）。
    pub fn think_xml() -> Self {
        Self {
            prefix: "<think>".into(),
            suffix: "</think>".into(),
            separator: "\n".into(),
        }
    }
}

/// 单条推理包装：`prefix + text + suffix`。
///
/// 空 `text` 也照样包装 —— 调用方应先用 `inject_reasoning` 的过滤语义
/// （跳过空项）拿到合法候选，这里只做纯字面拼接。
pub fn wrap_reasoning(text: &str, template: &ReasoningTemplate) -> String {
    let mut out = String::with_capacity(template.prefix.len() + text.len() + template.suffix.len());
    out.push_str(&template.prefix);
    out.push_str(text);
    out.push_str(&template.suffix);
    out
}

/// 把多条 reasoning 注入到 `messages` 末尾，**生成新 vec**、**不改原 vec**。
///
/// 规则：
/// 1. `reasoning_texts` 按出现顺序视为「新→旧」（调用方负责排序）；
/// 2. 每条空字符串（trim 后为空）跳过；
/// 3. 用 `template` 包装成单独一条 `AgentModelMessage`（role=System、text part）；
/// 4. 最多取 `max_additions` 条（**0 表示全部**）；超出的更旧项被截断丢弃；
/// 5. 返回 `messages`（clone）后追加这些 system 消息；`messages` 自身不变。
///
/// 实现细节：
/// - 模板只用于单条包装；多条之间不靠 separator 拼接 —— 每条独占一条
///   system 消息，方便 caller 之后做 token 裁剪时整条按 budget 评估。
///
/// ponytail: 一个 filter + take + map + extend；不发明新概念。
pub fn inject_reasoning(
    messages: &[AgentModelMessage],
    reasoning_texts: &[String],
    template: &ReasoningTemplate,
    max_additions: usize,
) -> Vec<AgentModelMessage> {
    let mut out = messages.to_vec();
    let mut additions: Vec<AgentModelMessage> = Vec::new();
    for raw in reasoning_texts.iter().take(if max_additions == 0 {
        reasoning_texts.len()
    } else {
        max_additions
    }) {
        if raw.trim().is_empty() {
            continue;
        }
        let wrapped = wrap_reasoning(raw, template);
        additions.push(AgentModelMessage::text(AgentModelRole::System, wrapped));
    }
    out.extend(additions);
    out
}
