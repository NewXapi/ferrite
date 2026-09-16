//! 用量日志 + dashboard — 单机版平表实现 (usage_logs)。
//!
//! 替代原 hourly/perf/rankings 骨架 (聚合优化推迟，先落原始表 + 查询)。
//! 网关侧后续调 `record()` 写消费记录; admin 面板走查询路由。
//!
//! log_type: 1=topup 2=consume 3=manage 4=system (对齐 new-api)。

use std::collections::HashMap;

use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::Freshness;
use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

/// 充值流水：`usage_logs.log_type = 1`（对齐 new-api，见 `db/migrations/0002_usage_logs.sql`）。
pub const LOG_TYPE_TOPUP: i16 = 1;

/// 消费流水：`usage_logs.log_type = 2`。网关每转发一次请求落一条，
/// [`LogService::top_usage`] 与 [`LogService::trend`] 只聚合这一类。
///
/// 写侧（网关中间件构造 [`UsageEvent`]）与读侧（聚合查询的 `WHERE log_type = ...`）
/// 必须引用同一个常量。历史事故：网关手写 `log_type: 1`（= 充值）而查询过滤 `= 2`，
/// 真实消费全部被当成充值，从 `/api/log/top` 与 `/api/log/trend` 里整体消失。
pub const LOG_TYPE_CONSUME: i16 = 2;

/// 错误流水：`usage_logs.log_type = 5`（枚举见 `db/migrations/0002_usage_logs.sql`）。
/// 上游已应答 ≥400 或传输失败的请求由 pipeline 结算的**零成本观测事件**翻译而来：
/// 不扣费（quota=0）、不进消费聚合（top/trend 只认 2），只留排障痕迹。
pub const LOG_TYPE_ERROR: i16 = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEvent {
    pub log_type: i16,
    pub user_key: Uuid,
    pub username: String,
    pub token_key: Option<Uuid>,
    pub token_name: String,
    pub channel_key: Option<Uuid>,
    pub channel_name: String,
    pub model_name: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub quota: i64,
    pub use_time_ms: i32,
    pub is_stream: bool,
    pub ip: String,
    pub request_id: String,
    pub content: String,
}

impl UsageEvent {
    /// 构造一条消费流水的骨架（`log_type = 2`），量化字段留 0 由调用方填。
    ///
    /// 网关中间件必须走这里而不是手写 `log_type` 字面量：这是 `log_type` 在写侧
    /// 的唯一定义点，与读侧 [`LOG_TYPE_CONSUME`] 同源。
    ///
    /// # 参数
    /// - `user_key`：`auth_users.key`（UUID）
    /// - `username`：冗余用户名，改名不回溯历史日志
    /// - `model_name`：客户端请求的公开模型别名（趋势/排行按它分组，空串会被聚合过滤掉）
    ///
    /// # 示例
    /// ```
    /// let mut e = observe::logs::UsageEvent::consume(uuid::Uuid::nil(), "alice", "gpt-4o");
    /// e.prompt_tokens = 10;
    /// assert_eq!(e.log_type, observe::logs::LOG_TYPE_CONSUME);
    /// ```
    pub fn consume(user_key: Uuid, username: &str, model_name: &str) -> Self {
        Self {
            log_type: LOG_TYPE_CONSUME,
            user_key,
            username: username.into(),
            token_key: None,
            token_name: String::new(),
            channel_key: None,
            channel_name: String::new(),
            model_name: model_name.into(),
            prompt_tokens: 0,
            completion_tokens: 0,
            quota: 0,
            use_time_ms: 0,
            is_stream: false,
            ip: String::new(),
            request_id: String::new(),
            content: String::new(),
        }
    }

    /// 构造一条错误流水的骨架（`log_type = 5`）。与 [`Self::consume`] 同理由：
    /// log_type 的唯一定义点，禁止调用方手写字面量。
    /// 错误摘要放 `content`（列注释：扩展信息 JSON/文本）。
    pub fn error(user_key: Uuid, username: &str, model_name: &str) -> Self {
        Self {
            log_type: LOG_TYPE_ERROR,
            ..Self::consume(user_key, username, model_name)
        }
    }
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogView {
    pub id: i64,
    pub log_type: i16,
    pub user_key: Uuid,
    pub username: String,
    pub token_name: String,
    pub channel_name: String,
    pub model_name: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub quota: i64,
    pub use_time_ms: i32,
    pub is_stream: bool,
    pub ip: String,
    pub request_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageStat {
    pub quota: i64,
    pub requests: i64,
    /// 近 60s 请求数
    pub rpm: i64,
    /// 近 60s token 数
    pub tpm: i64,
}

pub struct LogService {
    pool: PgPool,
}

const COLS: &str = "id, log_type, user_key, username, token_name, channel_name, model_name, \
     prompt_tokens, completion_tokens, quota, use_time_ms, is_stream, ip, request_id, \
     created_at";

impl LogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 网关写入一条消费记录。
    pub async fn record(&self, e: &UsageEvent) -> Result<i64, AuthError> {
        let id: (i64,) = sqlx::query_as(
            r#"INSERT INTO usage_logs
               (log_type, user_key, username, token_key, token_name, channel_key,
                channel_name, model_name, prompt_tokens, completion_tokens, quota,
                use_time_ms, is_stream, ip, request_id, content)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
               RETURNING id"#,
        )
        .bind(e.log_type)
        .bind(e.user_key)
        .bind(&e.username)
        .bind(e.token_key)
        .bind(&e.token_name)
        .bind(e.channel_key)
        .bind(&e.channel_name)
        .bind(&e.model_name)
        .bind(e.prompt_tokens)
        .bind(e.completion_tokens)
        .bind(e.quota)
        .bind(e.use_time_ms)
        .bind(e.is_stream)
        .bind(&e.ip)
        .bind(&e.request_id)
        .bind(&e.content)
        .fetch_one(&self.pool)
        .await?;
        Ok(id.0)
    }

    #[allow(clippy::too_many_arguments)]
    async fn query(
        &self,
        user_key: Option<Uuid>,
        log_type: Option<i16>,
        username: Option<&str>,
        token_name: Option<&str>,
        model_name: Option<&str>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<LogView>, i64), AuthError> {
        let size = size.clamp(1, 100);
        let offset = (page.max(1) - 1) * size;

        // 固定形状 SQL + NULL 传参 — 条件集固定, 不做动态拼接。
        let rows: Vec<LogView> = sqlx::query_as(&format!(
            r#"SELECT {COLS} FROM usage_logs
               WHERE ($1::uuid IS NULL OR user_key = $1)
                 AND ($2::smallint IS NULL OR log_type = $2)
                 AND ($3::text IS NULL OR username = $3)
                 AND ($4::text IS NULL OR token_name = $4)
                 AND ($5::text IS NULL OR model_name = $5)
                 AND ($6::timestamptz IS NULL OR created_at >= $6)
                 AND ($7::timestamptz IS NULL OR created_at < $7)
               ORDER BY id DESC
               LIMIT $8 OFFSET $9"#
        ))
        .bind(user_key)
        .bind(log_type)
        .bind(username.filter(|s| !s.trim().is_empty()))
        .bind(token_name.filter(|s| !s.trim().is_empty()))
        .bind(model_name.filter(|s| !s.trim().is_empty()))
        .bind(start)
        .bind(end)
        .bind(size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT count(*) FROM usage_logs
               WHERE ($1::uuid IS NULL OR user_key = $1)
                 AND ($2::smallint IS NULL OR log_type = $2)
                 AND ($3::text IS NULL OR username = $3)
                 AND ($4::text IS NULL OR token_name = $4)
                 AND ($5::text IS NULL OR model_name = $5)
                 AND ($6::timestamptz IS NULL OR created_at >= $6)
                 AND ($7::timestamptz IS NULL OR created_at < $7)"#,
        )
        .bind(user_key)
        .bind(log_type)
        .bind(username.filter(|s| !s.trim().is_empty()))
        .bind(token_name.filter(|s| !s.trim().is_empty()))
        .bind(model_name.filter(|s| !s.trim().is_empty()))
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok((rows, total))
    }

    /// admin 全量查询。
    #[allow(clippy::too_many_arguments)]
    pub async fn list_logs(
        &self,
        log_type: Option<i16>,
        username: Option<&str>,
        token_name: Option<&str>,
        model_name: Option<&str>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<LogView>, i64), AuthError> {
        self.query(
            None, log_type, username, token_name, model_name, start, end, page, size,
        )
        .await
    }

    /// 用户自查。
    #[allow(clippy::too_many_arguments)]
    pub async fn list_self_logs(
        &self,
        user_key: Uuid,
        log_type: Option<i16>,
        token_name: Option<&str>,
        model_name: Option<&str>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<LogView>, i64), AuthError> {
        self.query(
            Some(user_key),
            log_type,
            None,
            token_name,
            model_name,
            start,
            end,
            page,
            size,
        )
        .await
    }

    async fn stat_inner(&self, user_key: Option<Uuid>) -> Result<UsageStat, AuthError> {
        let stat = sqlx::query_as::<_, (i64, i64)>(
            r#"SELECT COALESCE(sum(quota),0)::bigint, count(*) FROM usage_logs
               WHERE ($1::uuid IS NULL OR user_key = $1)
                 AND created_at >= date_trunc('day', now())"#,
        )
        .bind(user_key)
        .fetch_one(&self.pool)
        .await?;
        let (rpm, tpm): (i64, i64) = sqlx::query_as(
            r#"SELECT count(*), COALESCE(sum(prompt_tokens + completion_tokens),0)::bigint FROM usage_logs
               WHERE ($1::uuid IS NULL OR user_key = $1)
                 AND created_at >= now() - interval '60 seconds'"#,
        )
        .bind(user_key)
        .fetch_one(&self.pool)
        .await?;
        Ok(UsageStat {
            quota: stat.0,
            requests: stat.1,
            rpm,
            tpm,
        })
    }

    pub async fn stat(&self) -> Result<UsageStat, AuthError> {
        self.stat_inner(None).await
    }

    pub async fn self_stat(&self, user_key: Uuid) -> Result<UsageStat, AuthError> {
        self.stat_inner(Some(user_key)).await
    }

    /// dashboard 汇总 — 一次查全。
    ///
    /// `quotaRemaining` 口径：`SUM(quota) FROM auth_users WHERE status = 1`
    /// （仅启用用户）。理由：登录侧可用判据即 `status == 1`（auth service
    /// 只放行启用用户），禁用/封禁用户的残留余额不可再消费，计入会高估
    /// 平台可消耗余量；与 `channelsEnabled` 只数 `status = 1` 渠道同精神。
    /// 响应尾部的 `asOf` 来自 [`Freshness`]（原则 7：不伪造实时）——单机
    /// 平表无分区语义，`partial` 恒 false。
    pub async fn dashboard(&self) -> Result<serde_json::Value, AuthError> {
        let (users, tokens, channels, channels_enabled, groups): (i64, i64, i64, i64, i64) =
            sqlx::query_as(
                r#"SELECT
                   (SELECT count(*) FROM auth_users),
                   (SELECT count(*) FROM api_tokens),
                   (SELECT count(*) FROM api_channels),
                   (SELECT count(*) FROM api_channels WHERE status = 1),
                   (SELECT count(*) FROM api_groups)"#,
            )
            .fetch_one(&self.pool)
            .await?;
        let stat = self.stat().await?;
        // 剩余可用额度：只汇总启用用户（口径见方法 doc）。
        let (quota_remaining,): (i64,) = sqlx::query_as(
            r#"SELECT COALESCE(sum(quota), 0)::bigint FROM auth_users WHERE status = 1"#,
        )
        .fetch_one(&self.pool)
        .await?;
        let fresh = Freshness {
            as_of: Utc::now(),
            partial: false, // 单机平表：要么全量要么查不到，无分区语义
        };
        Ok(serde_json::json!({
            "users": users,
            "tokens": tokens,
            "channels": channels,
            "channelsEnabled": channels_enabled,
            "groups": groups,
            "quotaToday": stat.quota,
            "requestsToday": stat.requests,
            "rpm": stat.rpm,
            "tpm": stat.tpm,
            "quotaRemaining": quota_remaining,
            // chrono 序列化为 RFC3339/ISO8601 UTC 字符串
            "asOf": fresh.as_of,
        }))
    }

    /// 消耗排行聚合（总览 Top10）：按用户或模型 GROUP BY 汇总 tokens/quota/调用数。
    /// `by` = "user" → 按 username 分组；"model" → 按 model_name 分组。
    /// `group_col` 只来自白名单枚举,不拼接外部输入。
    ///
    /// 每行附 [`UsageTopRow::previous_tokens`]：同口径下**上一等长窗口**
    /// `[start-(end-start), start)`（`end` 缺省按 now 计）的同实体 tokens。
    /// `start` 未提供（当前窗口无下界，不可定时）或窗口长度非正时，
    /// 全部行 `previous_tokens = 0`；上窗无该实体亦为 0。
    pub async fn top_usage(
        &self,
        by: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<UsageTopRow>, AuthError> {
        let group_col = match by {
            "model" => "model_name",
            _ => "username",
        };
        let limit = limit.clamp(1, 50);
        // log_type 过滤取常量而非字面量：与写侧 UsageEvent::consume 同源，
        // 任一侧漂移都会让真实消费从榜单里静默消失。
        let consume = LOG_TYPE_CONSUME;
        let sql = format!(
            r#"SELECT {group_col} AS name,
                      sum(prompt_tokens + completion_tokens)::bigint AS tokens,
                      sum(quota)::bigint AS quota,
                      count(*)::bigint AS calls
               FROM usage_logs
               WHERE log_type = {consume}
                 AND ({group_col} <> '')
                 AND ($1::timestamptz IS NULL OR created_at >= $1)
                 AND ($2::timestamptz IS NULL OR created_at < $2)
               GROUP BY {group_col}
               ORDER BY tokens DESC
               LIMIT $3"#
        );
        let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(&sql)
            .bind(start)
            .bind(end)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        // 上一等长窗口聚合：复用同一白名单选列与 log_type 常量拼装（防注入
        // 约束同上）。只在 start 提供且窗口长度为正时才查；上一窗不需要
        // LIMIT——只为当前窗出现的实体查值，多取无害。
        let mut previous: HashMap<String, i64> = HashMap::new();
        if let Some(s) = start {
            let end_eff = end.unwrap_or_else(Utc::now);
            let window = end_eff - s;
            if window > chrono::Duration::zero() {
                let prev_sql = format!(
                    r#"SELECT {group_col} AS name,
                              sum(prompt_tokens + completion_tokens)::bigint AS tokens
                       FROM usage_logs
                       WHERE log_type = {consume}
                         AND ({group_col} <> '')
                         AND created_at >= $1
                         AND created_at < $2
                       GROUP BY {group_col}"#
                );
                let prev_rows: Vec<(String, i64)> = sqlx::query_as(&prev_sql)
                    .bind(s - window)
                    .bind(s)
                    .fetch_all(&self.pool)
                    .await?;
                previous = prev_rows.into_iter().collect();
            }
        }

        Ok(rows
            .into_iter()
            .map(|(name, tokens, quota, calls)| UsageTopRow {
                previous_tokens: previous.get(&name).copied().unwrap_or(0),
                name,
                tokens,
                quota,
                calls,
            })
            .collect())
    }

    /// 错误流水聚合（排障榜）：`log_type = 5` 在 `now() - hours` 起的窗口内
    /// 按 model_name 分组，返回 {模型, 错误次数, 最后一次出现时刻}。
    ///
    /// - `hours` clamp 1..=168（最多回看一周），`limit` clamp 1..=50；
    /// - 排序：count 降序，同数次按模型名升序（确定性输出，便于测试与前端断言）；
    /// - 空模型名剔除（错误可能未关联模型，单独成组无排障价值）；
    /// - log_type 取常量而非字面量：与写侧 [`UsageEvent::error`] 同源。
    pub async fn error_stats(
        &self,
        hours: i64,
        limit: i64,
    ) -> Result<Vec<UsageErrorRow>, AuthError> {
        let hours = hours.clamp(1, 168);
        let limit = limit.clamp(1, 50);
        let err_type = LOG_TYPE_ERROR;
        let sql = format!(
            r#"SELECT model_name,
                      count(*)::bigint AS count,
                      max(created_at) AS last_seen
               FROM usage_logs
               WHERE log_type = {err_type}
                 AND model_name <> ''
                 AND created_at >= now() - make_interval(hours => $1::int)
               GROUP BY model_name
               ORDER BY count DESC, model_name ASC
               LIMIT $2"#
        );
        let rows: Vec<(String, i64, DateTime<Utc>)> = sqlx::query_as(&sql)
            .bind(hours as i32)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(model_name, count, last_seen_at)| UsageErrorRow {
                model_name,
                count,
                last_seen_at,
            })
            .collect())
    }

    /// 用量趋势聚合：把窗口内消费按时间桶 × 模型 GROUP BY（date_trunc）。
    /// `granularity` = hour | day | month（枚举内联,无注入面）。
    pub async fn trend(
        &self,
        granularity: &str,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Result<Vec<UsageTrendRow>, AuthError> {
        let unit = match granularity {
            "day" => "day",
            "month" => "month",
            _ => "hour",
        };
        // 同 top_usage：log_type 取常量，避免读写两侧各写一份字面量。
        let consume = LOG_TYPE_CONSUME;
        let sql = format!(
            r#"SELECT date_trunc('{unit}', created_at)::timestamptz AS bucket,
                      model_name,
                      sum(prompt_tokens + completion_tokens)::bigint AS tokens,
                      sum(quota)::bigint AS quota,
                      count(*)::bigint AS calls
               FROM usage_logs
               WHERE log_type = {consume}
                 AND model_name <> ''
                 AND ($1::timestamptz IS NULL OR created_at >= $1)
                 AND ($2::timestamptz IS NULL OR created_at < $2)
               GROUP BY bucket, model_name
               ORDER BY bucket"#
        );
        let rows: Vec<(DateTime<Utc>, String, i64, i64, i64)> = sqlx::query_as(&sql)
            .bind(start)
            .bind(end)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(bucket, model_name, tokens, quota, calls)| UsageTrendRow {
                bucket,
                model_name,
                tokens,
                quota,
                calls,
            })
            .collect())
    }
}

/// Top 榜单行（按用户或模型聚合）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTopRow {
    pub name: String,
    pub tokens: i64,
    pub quota: i64,
    pub calls: i64,
    /// 上一等长窗口的同实体 tokens 合计（环比参考）：
    /// 同 `by`/`start`/`limit` 口径下 `[start-(end-start), start)` 窗口
    /// （`end` 缺省按 now 计）。上窗无该实体、或请求未提供 `start`
    /// （窗口不可定时，无上一窗可言）时为 0。
    pub previous_tokens: i64,
}

/// 错误流水聚合行（`log_type = 5`，按模型分组）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageErrorRow {
    /// 模型名（空模型名不参与聚合）。
    pub model_name: String,
    /// 窗口内该模型的错误次数。
    pub count: i64,
    /// 窗口内最后一次错误时刻 (RFC3339)。
    pub last_seen_at: DateTime<Utc>,
}

/// 趋势行（时间桶 × 模型）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTrendRow {
    /// 桶起始时间 (RFC3339)。
    pub bucket: DateTime<Utc>,
    pub model_name: String,
    pub tokens: i64,
    pub quota: i64,
    pub calls: i64,
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct LogAppState {
    pub svc: std::sync::Arc<LogService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: LogAppState) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/log", get(list))
        .route("/api/log/stat", get(stat))
        .route("/api/log/top", get(top))
        .route("/api/log/errors", get(errors))
        .route("/api/log/trend", get(trend))
        .route("/api/log/self", get(list_self))
        .route("/api/log/self/stat", get(self_stat))
        .route("/api/dashboard", get(dashboard))
        .with_state(state)
}

fn err_json(e: AuthError) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    (
        e.status(),
        axum::Json(serde_json::json!({ "code": e.code(), "message": e.to_string() })),
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogQuery {
    log_type: Option<i16>,
    username: Option<String>,
    token_name: Option<String>,
    model_name: Option<String>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    page: Option<i64>,
    size: Option<i64>,
}

async fn list(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<LogQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    match state
        .svc
        .list_logs(
            q.log_type,
            q.username.as_deref(),
            q.token_name.as_deref(),
            q.model_name.as_deref(),
            q.start,
            q.end,
            q.page.unwrap_or(1),
            q.size.unwrap_or(20),
        )
        .await
    {
        Ok((items, total)) => Ok(axum::Json(
            serde_json::json!({"items": items, "total": total}),
        )),
        Err(e) => Err(err_json(e)),
    }
}

async fn stat(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    match state.svc.stat().await {
        Ok(s) => Ok(axum::Json(serde_json::json!(s))),
        Err(e) => Err(err_json(e)),
    }
}

async fn list_self(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<LogQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&user.key)
        .map_err(|_| AuthError::InvalidToken)
        .map_err(err_json)?;
    match state
        .svc
        .list_self_logs(
            key,
            q.log_type,
            q.token_name.as_deref(),
            q.model_name.as_deref(),
            q.start,
            q.end,
            q.page.unwrap_or(1),
            q.size.unwrap_or(20),
        )
        .await
    {
        Ok((items, total)) => Ok(axum::Json(
            serde_json::json!({"items": items, "total": total}),
        )),
        Err(e) => Err(err_json(e)),
    }
}

async fn self_stat(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&user.key)
        .map_err(|_| AuthError::InvalidToken)
        .map_err(err_json)?;
    match state.svc.self_stat(key).await {
        Ok(s) => Ok(axum::Json(serde_json::json!(s))),
        Err(e) => Err(err_json(e)),
    }
}

async fn dashboard(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    match state.svc.dashboard().await {
        Ok(d) => Ok(axum::Json(d)),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TopQuery {
    /// "user" | "model"，默认 user。
    by: Option<String>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: Option<i64>,
}

/// GET /api/log/top?by=user|model&start=&end=&limit= — 消耗 Top 榜（总览 Top10 数据源）。
async fn top(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<TopQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    match state
        .svc
        .top_usage(
            q.by.as_deref().unwrap_or("user"),
            q.start,
            q.end,
            q.limit.unwrap_or(10),
        )
        .await
    {
        Ok(items) => Ok(axum::Json(serde_json::json!({ "items": items }))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrendQuery {
    /// "hour" | "day" | "month"，默认 hour。
    granularity: Option<String>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
}

/// GET /api/log/trend?granularity=hour|day|month&start=&end= — 用量趋势桶。
async fn trend(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<TrendQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    match state
        .svc
        .trend(q.granularity.as_deref().unwrap_or("hour"), q.start, q.end)
        .await
    {
        Ok(items) => Ok(axum::Json(serde_json::json!({ "items": items }))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorsQuery {
    /// 回看窗口小时数，默认 24（服务层 clamp 1..=168）。
    hours: Option<i64>,
    /// 返回条数，默认 10（服务层 clamp 1..=50）。
    limit: Option<i64>,
}

/// GET /api/log/errors?hours=24&limit=10 — 错误流水按模型聚合（排障榜）。
/// 鉴权与兄弟端点一致（admin role 阈值，同 router 层）。
async fn errors(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ErrorsQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    let fresh = Freshness {
        as_of: Utc::now(),
        partial: false, // 单机平表：无分区语义
    };
    match state
        .svc
        .error_stats(q.hours.unwrap_or(24), q.limit.unwrap_or(10))
        .await
    {
        Ok(items) => Ok(axum::Json(
            serde_json::json!({ "items": items, "asOf": fresh.as_of }),
        )),
        Err(e) => Err(err_json(e)),
    }
}
