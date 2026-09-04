//! Admin page API adapter.
//!
//! Provides typed REST API calls using `client::ApiClient` and `contract::api` DTOs
//! for tokens, channels, and groups.

use client::{ApiClient, ApiResult};
use contract::api::admin::{ChannelDto, ChannelUpsertRequest, GroupDto, GroupUpsertRequest};
use contract::api::token::{CreateTokenRequest, TokenDto, UpdateTokenRequest};

// ---------------------------------------------------------------------------
// Tokens (API Keys)
// ---------------------------------------------------------------------------

/// 真实调用: GET /api/token (列表，admin 模式下包含全局令牌)
pub async fn list_tokens_api(client: &ApiClient) -> ApiResult<Vec<TokenDto>> {
    client.get("/api/token").await
}

/// 真实调用: POST /api/token (创建)
pub async fn create_token_api(client: &ApiClient, req: &CreateTokenRequest) -> ApiResult<TokenDto> {
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

/// 真实调用: GET /api/channel (列表，密钥已掩码)
pub async fn list_channels_api(client: &ApiClient) -> ApiResult<Vec<ChannelDto>> {
    client.get("/api/channel").await
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

/// 真实调用: GET /api/group (分组列表)
pub async fn list_groups_api(client: &ApiClient) -> ApiResult<Vec<GroupDto>> {
    client.get("/api/group").await
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
