//! 用量查询端点 DTO。
//! 日志行对齐后端 `admin-observe::logs::LogView` (camelCase,2026-09-09 curl
//! 实测);统计与 Dashboard 对齐 `/api/log/stat` 与 `/api/dashboard`。

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

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogPage {
    pub items: Vec<UsageLogDto>,
    pub total: u64,
}

/// 日志行 — 对齐后端 LogView。
/// `success` / `cost` / `cachedTokens` / `firstTokenMs` 后端暂无对应列,
/// 由前端从 `quota` / `logType` 派生或显示占位 (见 account 页)。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogDto {
    pub id: i64,
    /// logType 1=充值 2=消费 (后端 UsageEvent::consume 置 2)。
    pub log_type: i16,
    pub user_key: String,
    pub username: String,
    pub token_name: String,
    pub channel_name: String,
    pub model_name: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    /// 内部计费单位 (500_000 = ¥1),后端未换算。
    pub quota: i64,
    pub use_time_ms: i32,
    pub is_stream: bool,
    pub ip: String,
    pub request_id: String,
    /// RFC3339 (后端 chrono DateTime<Utc> 序列化)。
    pub created_at: String,
}

impl From<&UsageEventRecord> for UsageLogDto {
    fn from(e: &UsageEventRecord) -> Self {
        Self {
            id: 0,
            log_type: 2,
            user_key: e.user_key.clone(),
            username: String::new(),
            token_name: String::new(),
            channel_name: e.channel_key.clone(),
            model_name: e.public_model.clone(),
            prompt_tokens: e.prompt_tokens as i32,
            completion_tokens: e.completion_tokens as i32,
            quota: e.cost,
            use_time_ms: e.duration_ms as i32,
            is_stream: false,
            ip: String::new(),
            request_id: String::new(),
            created_at: e.meta.updated_at.to_rfc3339(),
        }
    }
}

/// 用量统计 DTO (/api/log/stat, /api/log/self/stat)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStatDto {
    pub quota: i64,
    pub requests: i64,
    pub rpm: i64,
    pub tpm: i64,
}

/// Dashboard 汇总 DTO (/api/dashboard)
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummaryDto {
    pub users: i64,
    pub tokens: i64,
    pub channels: i64,
    pub channels_enabled: i64,
    pub groups: i64,
    pub quota_today: i64,
    pub requests_today: i64,
    pub rpm: i64,
    pub tpm: i64,
}
