//! 账户页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 两类入口并存:
//! - `fetch_*`: mock 直连 (同步返回 `mock` crate 静态数据), 覆盖尚无后端端点的
//!   面板 (钱包 / 邀请分成), 接上后端时只改本文件;
//! - `*_api`: 真实后端调用 (async, 走 `client::ApiClient`), 已覆盖密钥 / 用量 /
//!   用户信息 / 会话 / 设置 / 兑换码充值。

pub use mock::account::{ApiKey, Invitee, Profile, Recharge, RewardStat, UsageLog, Wallet};

// ---- 密钥·资料面板 ----

/// 统计卡:(值, 标签)
pub fn fetch_key_stats() -> &'static [(&'static str, &'static str)] {
    mock::account::KEY_STATS
}

pub fn fetch_profile() -> &'static Profile {
    &mock::account::PROFILE
}

pub fn fetch_keys() -> &'static [ApiKey] {
    mock::account::KEYS
}

// ---- 用量日志面板 ----

/// 统计卡:(值, 标签)
pub fn fetch_usage_stats() -> &'static [(&'static str, &'static str)] {
    mock::account::USAGE_STATS
}

/// 筛选可选模型,首项 "全部" 表示不过滤。
pub fn fetch_log_models() -> &'static [&'static str] {
    mock::account::LOG_MODELS
}

pub fn fetch_logs() -> &'static [UsageLog] {
    mock::account::LOGS
}

// ---- 邀请奖励面板 ----

pub fn fetch_wallet() -> &'static Wallet {
    &mock::account::WALLET
}

pub fn fetch_recharges() -> &'static [Recharge] {
    mock::account::RECHARGES
}

pub fn fetch_reward_stats() -> &'static [RewardStat] {
    mock::account::REWARD_STATS
}

pub fn fetch_invitees() -> &'static [Invitee] {
    mock::account::INVITEES
}

pub fn fetch_invite_link() -> &'static str {
    mock::account::INVITE_LINK
}

use client::{ApiClient, ApiResult};
use contract::api::token::{
    CreateTokenRequest, CreateTokenResult, TokenDto, TokenList, UpdateTokenRequest,
};
use contract::api::usage::{UsageLogPage, UsageStatDto};
use contract::api::user::{SessionDto, UpdateSelfRequest, UserDto, UserTopupRequest};

/// 真实调用: GET /api/token (owner 模式, 后端按 token 属主过滤, 无 query 参数)。
/// 列表为 `{items}` 信封, 拆包后返回 `Vec<TokenDto>`。
pub async fn list_tokens_api(client: &ApiClient) -> ApiResult<Vec<TokenDto>> {
    let page = client.get::<TokenList>("/api/token").await?;
    Ok(page.items)
}

/// 真实调用: POST /api/token — 响应含一次性明文 key (只在创建时返回一次)。
pub async fn create_token_api(
    client: &ApiClient,
    req: &CreateTokenRequest,
) -> ApiResult<CreateTokenResult> {
    client.post("/api/token", req).await
}

/// 真实调用: PUT /api/token/{key} — 缺省字段不发, 后端按「缺省=不改」处理。
pub async fn update_token_api(
    client: &ApiClient,
    key: &str,
    req: &UpdateTokenRequest,
) -> ApiResult<TokenDto> {
    client.put(&format!("/api/token/{key}"), req).await
}

/// 真实调用: DELETE /api/token/{key}
pub async fn delete_token_api(client: &ApiClient, key: &str) -> ApiResult<serde_json::Value> {
    client.delete(&format!("/api/token/{key}")).await
}

/// 真实调用: GET /api/log/self — 用户自查用量日志。
/// 参数与后端 `LogQuery` 对齐 (camelCase): modelName 过滤、start/end 为 RFC3339
/// (start 闭 / end 开)、page 1-based、size 后端 clamp 1..=100 (缺省 20)。
pub async fn list_self_logs_api(
    client: &ApiClient,
    model: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    page: Option<i64>,
    size: Option<i64>,
) -> ApiResult<UsageLogPage> {
    let mut query = Vec::new();
    if let Some(m) = model
        && !m.is_empty()
    {
        query.push(format!("modelName={m}"));
    }
    if let Some(st) = start
        && !st.is_empty()
    {
        query.push(format!("start={st}"));
    }
    if let Some(en) = end
        && !en.is_empty()
    {
        query.push(format!("end={en}"));
    }
    if let Some(p) = page {
        query.push(format!("page={p}"));
    }
    if let Some(sz) = size {
        query.push(format!("size={sz}"));
    }
    let path = if query.is_empty() {
        "/api/log/self".to_string()
    } else {
        format!("/api/log/self?{}", query.join("&"))
    };
    client.get(&path).await
}

/// 真实调用: GET /api/log/self/stat
pub async fn get_self_stat_api(client: &ApiClient) -> ApiResult<UsageStatDto> {
    client.get("/api/log/self/stat").await
}

// ---- 用户信息 (self / sessions / settings) ----

/// 真实调用: GET /api/user/self → 当前登录用户的真实资料 (UserView wire)。
pub async fn get_self_api(client: &ApiClient) -> ApiResult<UserDto> {
    client.get("/api/user/self").await
}

/// 真实调用: PUT /api/user/self — 改显示名 / 改密码 (改密须原密码+新密码成对)。
pub async fn update_self_api(client: &ApiClient, req: &UpdateSelfRequest) -> ApiResult<UserDto> {
    client.put("/api/user/self", req).await
}

/// 真实调用: GET /api/user/self/sessions → 当前用户全部存活会话。
pub async fn list_sessions_api(client: &ApiClient) -> ApiResult<Vec<SessionDto>> {
    client.get("/api/user/self/sessions").await
}

/// 真实调用: DELETE /api/user/self/sessions/{sid} — 吊销指定会话。
pub async fn revoke_session_api(client: &ApiClient, sid: &str) -> ApiResult<serde_json::Value> {
    client
        .delete(&format!("/api/user/self/sessions/{sid}"))
        .await
}

/// 真实调用: POST /api/user/self/sessions/revoke-others — 吊销其它设备会话。
pub async fn revoke_others_sessions_api(client: &ApiClient) -> ApiResult<serde_json::Value> {
    client
        .post(
            "/api/user/self/sessions/revoke-others",
            &serde_json::json!({}),
        )
        .await
}

/// 真实调用: GET /api/user/self/setting → 用户设置 (自由 JSONB 对象)。
pub async fn get_settings_api(client: &ApiClient) -> ApiResult<serde_json::Value> {
    client.get("/api/user/self/setting").await
}

/// 真实调用: PUT /api/user/self/setting — 合并保存用户设置。
pub async fn update_settings_api(
    client: &ApiClient,
    settings: &serde_json::Value,
) -> ApiResult<serde_json::Value> {
    client.put("/api/user/self/setting", settings).await
}

// ---- 兑换码充值 (rewards 面板) ----

/// 真实调用: POST /api/user/topup — 兑换码充值 (CAS 核销, 事务内入账用户 quota)。
///
/// 请求体 [`UserTopupRequest`] 与后端本地 `TopupRequest { key }` 逐字对齐;
/// 成功响应为裸 JSON `{"quota": <入账额度, 内部单位>, "success": true}`,
/// 用 [`topup_credited_quota`] 提取入账值展示。
pub async fn topup_api(client: &ApiClient, req: &UserTopupRequest) -> ApiResult<serde_json::Value> {
    client.post("/api/user/topup", req).await
}

/// 从 POST /api/user/topup 的成功响应提取入账额度。
///
/// 后端成功返回 `{"quota": <i64 内部单位>, "success": true}`; 字段缺失、
/// 非整数 (含浮点) 或为 null 时返回 `None`, 调用方降级为通用成功文案,
/// 不假造入账数值。
pub fn topup_credited_quota(resp: &serde_json::Value) -> Option<i64> {
    resp.get("quota").and_then(|q| q.as_i64())
}
