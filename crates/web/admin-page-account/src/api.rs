//! 账户页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 现在是 mock 直连(同步返回 `mock` crate 的静态数据);接上真实后端时
//! 只改本文件 —— 换成 `crates/shared/client` 的请求,必要时把签名改成 async,
//! 三个面板(keys / usage_logs / rewards)本身无需改动。

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
use contract::api::token::{CreateTokenRequest, TokenDto, UpdateTokenRequest};
use contract::api::user::{SessionDto, UpdateSelfRequest, UserDto};
use contract::api::usage::{UsageLogPage, UsageStatDto};

/// 真实调用: GET /api/token
pub async fn list_tokens_api(client: &ApiClient) -> ApiResult<Vec<TokenDto>> {
    client.get("/api/token").await
}

/// 真实调用: POST /api/token
pub async fn create_token_api(client: &ApiClient, req: &CreateTokenRequest) -> ApiResult<TokenDto> {
    client.post("/api/token", req).await
}

/// 真实调用: PUT /api/token/{key}
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

/// 真实调用: GET /api/log/self?start=&end=&model=&p=&page_size=
pub async fn list_self_logs_api(
    client: &ApiClient,
    model: Option<&str>,
    page: Option<u32>,
    page_size: Option<u32>,
) -> ApiResult<UsageLogPage> {
    let mut query = Vec::new();
    if let Some(m) = model
        && !m.is_empty()
    {
        query.push(format!("model={m}"));
    }
    if let Some(p) = page {
        query.push(format!("p={p}"));
    }
    if let Some(ps) = page_size {
        query.push(format!("page_size={ps}"));
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
pub async fn update_self_api(
    client: &ApiClient,
    req: &UpdateSelfRequest,
) -> ApiResult<UserDto> {
    client.put("/api/user/self", req).await
}

/// 真实调用: GET /api/user/self/sessions → 当前用户全部存活会话。
pub async fn list_sessions_api(client: &ApiClient) -> ApiResult<Vec<SessionDto>> {
    client.get("/api/user/self/sessions").await
}

/// 真实调用: DELETE /api/user/self/sessions/{sid} — 吊销指定会话。
pub async fn revoke_session_api(client: &ApiClient, sid: &str) -> ApiResult<serde_json::Value> {
    client.delete(&format!("/api/user/self/sessions/{sid}")).await
}

/// 真实调用: POST /api/user/self/sessions/revoke-others — 吊销其它设备会话。
pub async fn revoke_others_sessions_api(
    client: &ApiClient,
) -> ApiResult<serde_json::Value> {
    client.post("/api/user/self/sessions/revoke-others", &serde_json::json!({})).await
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
