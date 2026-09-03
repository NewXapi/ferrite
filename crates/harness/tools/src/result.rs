//! Agent 侧工具结果与模型侧工具描述。
//!
//! 整文件照抄自 TauriTavern `tt-domain/src/models/agent/mod.rs:248-270`。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::spec::ToolId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelTool {
    pub tool_id: ToolId,
    pub model_alias: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolResult {
    pub call_id: String,
    pub tool_id: ToolId,
    pub content: String,
    #[serde(default)]
    pub structured: Value,
    #[serde(default)]
    pub is_error: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default)]
    pub resource_refs: Vec<String>,
}
