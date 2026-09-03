//! 运行时事件级别 + 事件结构。
//!
//! 整段搬运自 `tt-domain/src/models/agent/mod.rs:223-244`。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunEvent {
    pub seq: u64,
    pub id: String,
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub level: AgentRunEventLevel,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunEventLevel {
    Debug,
    Info,
    Warn,
    Error,
}
