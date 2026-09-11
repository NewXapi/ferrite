//! 用量查询端点 DTO。
//!
//! 与后端 `crates/api/admin-observe` 的真实 wire 形状一一对齐（后端为权威）：
//!
//! - [`UsageLogDto`]     ← `LogView` (GET /api/log, /api/log/self 的 `items` 元素, camelCase)
//! - [`UsageLogPage`]    ← 上述端点的响应体 `{"items": [...], "total": n}`
//! - [`UsageLogQuery`]   ← `LogQuery` (query string 参数, camelCase)
//! - [`UsageStatDto`]    ← `UsageStat` (GET /api/log/stat, /api/log/self/stat)
//! - [`UsageDailyStatDto`] 按天聚合口径（未来 /api/log/daily 端点的契约占位）
//! - [`DashboardSummaryDto`] ← GET /api/dashboard 的 json! 汇总
//!
//! 依赖纪律（crate 级）：契约层只允许纯数据/序列化依赖。时间一律用 `String`
//! 承载 RFC3339（如 `"2026-09-08T17:55:28.579140Z"`），避免 wasm 侧引入
//! chrono 的 parse 成本与 feature 面。

use serde::{Deserialize, Serialize};

/// GET /api/log 与 GET /api/log/self 的查询参数。
///
/// 与后端 `LogQuery` 字段一一对齐（camelCase）。所有字段可选：缺省（wire 上
/// 无该参数）即不过滤。序列化时用 [`serde::skip_serializing_if`] 省略
/// `None` 字段，等价于 URL query 中"不带该参数"，后端 `Option` 解析为
/// `None` 后走默认值（`page` 默认 1、`size` 默认 20 且 clamp 到 1..=100）。
///
/// 错误情况：`start`/`end` 必须为 RFC3339 字符串（如 `2026-09-08T00:00:00Z`），
/// 后端解析失败时返回 4xx 错误信封。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogQuery {
    /// 日志类型过滤。`None` = 不过滤。取值对齐 new-api：
    /// 1=topup, 2=consume, 3=manage, 4=system。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_type: Option<i16>,
    /// 用户名精确匹配。`None` 或空串 = 不过滤（后端会 trim 后过滤空串）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// API 密钥名称精确匹配。`None` 或空串 = 不过滤。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_name: Option<String>,
    /// 模型名精确匹配。`None` 或空串 = 不过滤。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    /// 时间窗起点，RFC3339 字符串，闭区间（`created_at >= start`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// 时间窗终点，RFC3339 字符串，开区间（`created_at < end`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// 1-based 页码。`None` 时后端默认 1。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    /// 每页条数。`None` 时后端默认 20；后端会 clamp 到 1..=100。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
}

/// `{"items": UsageLogDto[], "total": i64}` — /api/log 与 /api/log/self 的响应体。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogPage {
    /// 当前页日志行，按 `id DESC`（新→旧）排序。
    pub items: Vec<UsageLogDto>,
    /// 命中总条数（非当前页长度），i64 与后端 `count(*)` 一致。
    pub total: i64,
}

/// 单条用量日志 wire 行 — 与后端 `LogView` 字段一一对齐（camelCase）。
///
/// 来源：admin-observe `usage_logs` 平表（单机版）。`id` 为数据库 BIGSERIAL
/// 主键，前端直接以字符串展示或做行 key 均可。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogDto {
    /// 记录唯一标识（PostgreSQL BIGSERIAL 主键）。
    pub id: i64,
    /// 日志类型：1=topup, 2=consume, 3=manage, 4=system（对齐 new-api）。
    pub log_type: i16,
    /// 产生该记录的用户 UUID。wire 上为字符串形式（Uuid 序列化产物）。
    pub user_key: String,
    /// 用户名。后端默认空串（`''`），不会缺字段。
    pub username: String,
    /// API 密钥名称。未走密钥的请求为空串。
    pub token_name: String,
    /// 上游渠道名称。未归属渠道时为空串。
    pub channel_name: String,
    /// 模型名。
    pub model_name: String,
    /// 输入 token 数。
    pub prompt_tokens: i32,
    /// 输出 token 数。
    pub completion_tokens: i32,
    /// 内部额度单位消耗量。换算口径：500_000 ≈ $1。
    pub quota: i64,
    /// 请求耗时（毫秒）。
    pub use_time_ms: i32,
    /// 是否流式请求。
    pub is_stream: bool,
    /// 客户端 IP。未知时为空串。
    pub ip: String,
    /// 网关侧请求关联 ID。
    pub request_id: String,
    /// 记录创建时间，RFC3339 字符串（如 `"2026-09-08T17:55:28.579140Z"`）。
    /// 契约层用 String 承载，避免 wasm 侧引入 chrono。
    pub created_at: String,
}

/// 用量统计 — GET /api/log/stat（全量）与 /api/log/self/stat（本人）的响应体。
///
/// 口径：`quota` / `requests` 为**当日**（`date_trunc('day', now())` 起）
/// 累计；`rpm` / `tpm` 为**近 60 秒**滑动窗口。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStatDto {
    /// 当日总消耗（内部额度单位，500_000 ≈ $1）。
    pub quota: i64,
    /// 当日请求总数。
    pub requests: i64,
    /// 近 60 秒请求数。
    pub rpm: i64,
    /// 近 60 秒 token 数（prompt + completion 之和）。
    pub tpm: i64,
}

/// 按天聚合的用量统计。
///
/// 契约占位：后端 admin-observe 的聚合查询尚未落地，本 DTO 先定义 wire 形状
/// 供前端按天图表使用。各字段：
///
/// - `date`：按天分组键，格式 `YYYY-MM-DD`（后端本地日；前端按字符串比较/排序即可）；
/// - `requests`：当日请求总数；
/// - `tokens`：当日 token 总数，口径为 `prompt_tokens + completion_tokens`；
/// - `quota`：当日消耗（内部额度单位，500_000 ≈ $1）。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDailyStatDto {
    /// 按天分组键，`YYYY-MM-DD`。
    pub date: String,
    /// 当日请求总数。
    pub requests: i64,
    /// 当日 token 总数（prompt + completion）。
    pub tokens: i64,
    /// 当日消耗（内部额度单位，500_000 ≈ $1）。
    pub quota: i64,
}

/// `GET /api/log/self/stat/daily` 响应信封 (后端 `json!({ "items": [...] })` 直出)。
/// 一行 = 一天；无数据的中间日期不出现（后端不补零）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDailyStatPage {
    pub items: Vec<UsageDailyStatDto>,
}

/// Dashboard 汇总 — GET /api/dashboard 的响应体（后端 `json!` 直出，camelCase）。
///
/// 口径同 [`UsageStatDto`]：`quota_today` / `requests_today` 为当日累计，
/// `rpm` / `tpm` 为近 60 秒窗口。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummaryDto {
    /// 用户总数。
    pub users: i64,
    /// API 密钥总数。
    pub tokens: i64,
    /// 渠道总数。
    pub channels: i64,
    /// 启用中的渠道数。
    pub channels_enabled: i64,
    /// 分组总数。
    pub groups: i64,
    /// 当日消耗（内部额度单位，500_000 ≈ $1）。
    pub quota_today: i64,
    /// 当日请求总数。
    pub requests_today: i64,
    /// 近 60 秒请求数。
    pub rpm: i64,
    /// 近 60 秒 token 数（prompt + completion 之和）。
    pub tpm: i64,
}
