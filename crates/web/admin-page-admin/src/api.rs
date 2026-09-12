//! Admin page API adapter.
//!
//! Provides typed REST API calls using `client::ApiClient` and `contract::api` DTOs
//! for tokens, channels, and groups.

use client::{ApiClient, ApiResult};
use contract::api::admin::{ChannelDto, ChannelUpsertRequest, GroupDto, GroupUpsertRequest};
use contract::api::token::{
    CreateTokenRequest, CreateTokenResult, TokenDto, TokenList, UpdateTokenRequest,
};

/// 后端列表端点 (`GET /api/group` / `GET /api/channel`) 的 data 统一包装
/// `{"items":[...]}`（渠道另有 `total`，本 crate 页面暂不展示总数，忽略）。
///
/// `ApiClient::get::<T>` 只剥最外层 `Envelope{success,message,data}`
/// （admin-catalog 这三个端点目前直接回裸 map），拿到 map 后再按裸
/// `Vec<Dto>` 解必炸 `invalid type: map, expected a sequence`——历史事故，
/// `tests/list_envelope.rs` 把这一现场钉死为反向断言。列表 helper 一律先经
/// 本类型剥壳，再返回裸 `Vec<Dto>` 给调用点。
///
/// 对外仅暴露给集成测试钉 wire 契约（同 crate 惯例见 state.rs hydrate 的
/// 局部 `Items<T>`、admin-page-overview api.rs）。
#[doc(hidden)]
#[derive(Debug, Default, serde::Deserialize)]
pub struct Items<T> {
    #[serde(default)]
    pub items: Vec<T>,
}

// ---------------------------------------------------------------------------
// Tokens (API Keys)
// ---------------------------------------------------------------------------

/// 真实调用: GET /api/token (列表，admin 模式下包含全局令牌)。
/// 后端 data 为 `{items:[...]}`（契约 [`TokenList`]），剥壳后返回裸 Vec。
pub async fn list_tokens_api(client: &ApiClient) -> ApiResult<Vec<TokenDto>> {
    let r: TokenList = client.get("/api/token").await?;
    Ok(r.items)
}

/// 真实调用: POST /api/token (创建) — 响应为 `{plaintext, token}`
pub async fn create_token_api(
    client: &ApiClient,
    req: &CreateTokenRequest,
) -> ApiResult<CreateTokenResult> {
    client.post("/api/token", req).await
}

/// 真实调用: PUT /api/token/{key} (编辑)
pub async fn update_token_api(
    client: &ApiClient,
    key: &str,
    req: &UpdateTokenRequest,
) -> ApiResult<TokenDto> {
    client.put(&format!("/api/token/{key}"), req).await
}

/// 真实调用: DELETE /api/token/{key} (删除)
pub async fn delete_token_api(client: &ApiClient, key: &str) -> ApiResult<serde_json::Value> {
    client.delete(&format!("/api/token/{key}")).await
}

// ---------------------------------------------------------------------------
// Channels
// ---------------------------------------------------------------------------

/// 真实调用: GET /api/channel (列表，密钥已掩码)。
/// 后端 data 为 `{items:[...],total:n}`，剥壳后返回裸 Vec（total 忽略）。
pub async fn list_channels_api(client: &ApiClient) -> ApiResult<Vec<ChannelDto>> {
    let r: Items<ChannelDto> = client.get("/api/channel").await?;
    Ok(r.items)
}

/// 真实调用: GET /api/channel/{key} (单查，包含完整 keys)
pub async fn get_channel_api(client: &ApiClient, key: &str) -> ApiResult<ChannelDto> {
    client.get(&format!("/api/channel/{key}")).await
}

/// 真实调用: POST /api/channel (创建)
pub async fn create_channel_api(
    client: &ApiClient,
    req: &ChannelUpsertRequest,
) -> ApiResult<ChannelDto> {
    client.post("/api/channel", req).await
}

/// 真实调用: PUT /api/channel/{key} (更新)
pub async fn update_channel_api(
    client: &ApiClient,
    key: &str,
    req: &ChannelUpsertRequest,
) -> ApiResult<ChannelDto> {
    client.put(&format!("/api/channel/{key}"), req).await
}

/// 真实调用: POST /api/channel/{key}/status (启停切换)
pub async fn set_channel_status_api(
    client: &ApiClient,
    key: &str,
    status: i16,
) -> ApiResult<serde_json::Value> {
    client
        .post(
            &format!("/api/channel/{key}/status"),
            &serde_json::json!({ "status": status }),
        )
        .await
}

/// 真实调用: DELETE /api/channel/{key} (删除)
pub async fn delete_channel_api(client: &ApiClient, key: &str) -> ApiResult<serde_json::Value> {
    client.delete(&format!("/api/channel/{key}")).await
}

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

/// 真实调用: GET /api/group (分组列表)。
/// 后端 data 为 `{items:[...]}`，剥壳后返回裸 Vec。
pub async fn list_groups_api(client: &ApiClient) -> ApiResult<Vec<GroupDto>> {
    let r: Items<GroupDto> = client.get("/api/group").await?;
    Ok(r.items)
}

/// 真实调用: POST /api/group (创建)
pub async fn create_group_api(client: &ApiClient, req: &GroupUpsertRequest) -> ApiResult<GroupDto> {
    client.post("/api/group", req).await
}

/// 真实调用: PUT /api/group/{key} (更新)
pub async fn update_group_api(
    client: &ApiClient,
    key: &str,
    req: &GroupUpsertRequest,
) -> ApiResult<GroupDto> {
    client.put(&format!("/api/group/{key}"), req).await
}

/// 真实调用: DELETE /api/group/{key} (删除)
pub async fn delete_group_api(client: &ApiClient, key: &str) -> ApiResult<serde_json::Value> {
    client.delete(&format!("/api/group/{key}")).await
}

// ---------- 兑换码 (billing_redemptions) ----------

/// 兑换码视图 — 对齐 admin-billing RedemptionView (camelCase)。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionView {
    pub key: String,
    pub code_preview: String,
    pub quota: i64,
    pub status: i16,
    pub redeemed_by: Option<String>,
    pub redeemed_at: Option<String>,
    pub created_at: String,
}

/// 后端列表端点统一包装 `{"items":[...]}`。
#[derive(Debug, Default, serde::Deserialize)]
struct RedemptionItems {
    #[serde(default)]
    items: Vec<RedemptionView>,
    #[serde(default)]
    total: u64,
}

/// 真实调用: GET /api/redemption?status=&page=&size=
pub async fn list_redemptions_api(
    client: &ApiClient,
    status: Option<i16>,
    page: Option<u32>,
    size: Option<u32>,
) -> ApiResult<(Vec<RedemptionView>, u64)> {
    let mut query = Vec::new();
    if let Some(s) = status {
        query.push(format!("status={s}"));
    }
    if let Some(p) = page {
        query.push(format!("page={p}"));
    }
    if let Some(sz) = size {
        query.push(format!("size={sz}"));
    }
    let path = if query.is_empty() {
        "/api/redemption".to_string()
    } else {
        format!("/api/redemption?{}", query.join("&"))
    };
    let r: RedemptionItems = client.get(&path).await?;
    Ok((r.items, r.total))
}

/// 真实调用: POST /api/redemption {quota, count} — 明文码只返回一次。
pub async fn generate_redemptions_api(
    client: &ApiClient,
    quota: i64,
    count: u32,
) -> ApiResult<Vec<String>> {
    #[derive(Default, serde::Deserialize)]
    struct CodesResp {
        #[serde(default)]
        codes: Vec<String>,
    }
    let r: CodesResp = client
        .post(
            "/api/redemption",
            &serde_json::json!({ "quota": quota, "count": count }),
        )
        .await?;
    Ok(r.codes)
}

/// 真实调用: DELETE /api/redemption/{key} — 后端语义为停用 (status→2)。
pub async fn disable_redemption_api(client: &ApiClient, key: &str) -> ApiResult<()> {
    client
        .delete::<serde_json::Value>(&format!("/api/redemption/{key}"))
        .await?;
    Ok(())
}
