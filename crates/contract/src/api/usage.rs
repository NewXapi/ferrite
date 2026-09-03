//! 用量查询端点 DTO。
//! 参考: mock::account::UsageLog 形状 (前端已按此渲染) + new-api /api/log/self。

use crate::records::UsageEventRecord;
use serde::{Deserialize, Serialize};

/// GET /api/log/self?start=&end=&model=&p=&page_size= → data: UsageLogPage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogQuery {
    /// Unix 秒; 0 = 不限。
    pub start: i64,
    pub end: i64,
    /// 空字符串 = 不过滤。
    pub model: String,
    /// 1-based 页码。
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogPage {
    pub items: Vec<UsageLogDto>,
    pub total: u64,
}

/// 前端日志行 — 与 mock::account::UsageLog 字段一一对齐。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogDto {
    pub id: String,
    pub model: String,
    pub success: bool,
    /// Unix 秒。
    pub timestamp: i64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cached_tokens: u32,
    pub first_token_ms: u32,
    pub duration_ms: u32,
    /// 前端展示用金额 (美元, 保留 4 位)。
    pub cost: f64,
    pub error: Option<String>,
}

impl From<&UsageEventRecord> for UsageLogDto {
    fn from(e: &UsageEventRecord) -> Self {
        Self {
            id: e.meta.key.clone(),
            model: e.public_model.clone(),
            success: (200..400).contains(&e.status_code),
            timestamp: e.meta.updated_at.timestamp(),
            prompt_tokens: e.prompt_tokens as u32,
            completion_tokens: e.completion_tokens as u32,
            cached_tokens: e.cached_tokens as u32,
            first_token_ms: e.first_token_ms,
            duration_ms: e.duration_ms,
            // 内部计费单位 → 美元: 500_000 = $1
            cost: e.cost as f64 / 500_000.0,
            error: e.error.clone(),
        }
    }
}
