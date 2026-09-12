//! Admin page API adapter.
//!
//! Provides typed REST API calls using `client::ApiClient` and `contract::api` DTOs
//! for tokens, channels, groups, and model aliases.

use client::{ApiClient, ApiResult};
use contract::api::admin::{ChannelDto, ChannelUpsertRequest, GroupDto, GroupUpsertRequest};
use contract::api::billing::AliasUpsertRequest;
use contract::api::token::{CreateTokenRequest, CreateTokenResult, TokenDto, UpdateTokenRequest};

// ---------------------------------------------------------------------------
// Tokens (API Keys)
// ---------------------------------------------------------------------------

/// 真实调用: GET /api/token (列表，admin 模式下包含全局令牌)
pub async fn list_tokens_api(client: &ApiClient) -> ApiResult<Vec<TokenDto>> {
    client.get("/api/token").await
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

// ---------------------------------------------------------------------------
// Model aliases (/api/models — admin-catalog models 域)
// ---------------------------------------------------------------------------

/// models 域列表项视图 — 对齐 admin-catalog `ModelView`(camelCase)。
///
/// 别名页只需要两个字段:`key`(UUID,PUT/DELETE 路径定位符)与
/// `name`(对外别名)。价格/倍率在后端 models 域没有对应列,不在此映射。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAliasView {
    pub key: String,
    pub name: String,
}

/// 后端列表端点统一包装 `{"items":[...]}`。
#[derive(Debug, Default, serde::Deserialize)]
struct ModelItems {
    #[serde(default)]
    items: Vec<ModelAliasView>,
}

/// 真实调用: GET /api/models?size=100 (模型别名列表;后端 size clamp 1..=100)。
///
/// 响应 items 为 `ModelView`,此处只映射 key+name,其余字段由 serde 忽略。
/// 错误情况:401(未登录)、网络失败、JSON 不含 items → `ApiError`。
pub async fn list_model_aliases_api(client: &ApiClient) -> ApiResult<Vec<ModelAliasView>> {
    let r: ModelItems = client.get("/api/models?size=100").await?;
    Ok(r.items)
}

/// 真实调用: PUT /api/models/{key} (更新;`key` 为 ModelView.key 的 UUID)。
///
/// 请求体复用 contract `AliasUpsertRequest`:其中与后端 models 域对应的仅
/// `name`(对外别名);display_name/价格/倍率字段后端无对应列,序列化为 null
/// 后被 `UpdateModelRequest`(全 Option + 未知字段忽略)读成 None,不会写库。
/// 注意:后端更新是「COALESCE 合并 → validate_model 整体校验 merged 值」两步 —
/// name-only 语义仍成立(页面只有 maskedKey,无法也不应回传真实凭据),但若该行
/// 存量 api_key 为空(无效数据,如绕过 create 校验的种子行),合并后校验不过,
/// 任何更新都会 400,须先修复存量数据。响应为 `ModelView`,页面只关心成败,
/// 此处解成原始 Value。
pub async fn update_model_alias_api(
    client: &ApiClient,
    key: &str,
    req: &AliasUpsertRequest,
) -> ApiResult<serde_json::Value> {
    client.put(&format!("/api/models/{key}"), req).await
}

/// 真实调用: DELETE /api/models/{key} (删除;后端返回 `{"success": true}`)。
///
/// 错误情况:key 非 UUID(400)、模型不存在(404)→ `ApiError`。
pub async fn delete_model_alias_api(client: &ApiClient, key: &str) -> ApiResult<serde_json::Value> {
    client.delete(&format!("/api/models/{key}")).await
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
