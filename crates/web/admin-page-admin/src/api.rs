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

/// 后端列表端点统一包装 `{"items":[...]}`（渠道另带 `total`，未声明即忽略）。
///
/// 历史：admin-catalog 的 list handler 经 `ok_json` 返回**裸** map（无外层
/// Envelope），`ApiClient` 剥壳后按裸 `Vec<Dto>` 解码会撞
/// `decode error: invalid type: map, expected a sequence`——分组页曾因此
/// 永远走 error 分支（#182 修），渠道端点同型（本结构 + `list_channels_api`
/// 剥壳修，wire 契约见 `tests/list_envelope.rs`）。
#[derive(Debug, Default, serde::Deserialize)]
pub struct Items<T> {
    #[serde(default)]
    pub items: Vec<T>,
}

/// 真实调用: GET /api/channel (列表，密钥已掩码；响应为 `{"items":[..],"total":n}`)
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

/// 渠道编辑（PUT）的最小 diff 请求体 —— 对齐后端私有结构 `UpdateChannelRequest`
/// （channels.rs，字段全 `Option`，缺省 = 不改动）。
///
/// 设计依据（后端 `ChannelService::update` 实读）：
/// - 多数列走 `COALESCE($n, col)`：字段缺席 = 保持现值。弹窗不管理的
///   `models`/`priority`/`weight`/`status` 一律不发——历史上用裸
///   [`ChannelUpsertRequest`] 全量 PUT 时这些列恒带 `[]`/`0`/`null`，
///   保存一次就把它们静默清零。
/// - `keys`：缺席 = 后端保持现有密钥（svc.update 的 `None` 分支）。仅在用户
///   重新输入明文时携带；恒发 `[]` 会被后端 validate 以
///   "at least one key required" 拒绝 → 编辑保存必然 400（历史事故，
///   `tests/channel_update_body.rs` 钉死）。掩码值绝不回传。
/// - `test_model`：SQL 直绑、**无** COALESCE，缺席即把列清成 NULL——所以
///   必须恒带现值（None 序列化为显式 `null`，与「现值本就是 NULL」等价）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateChannelBody {
    /// 渠道名称（弹窗编辑项，恒发）。
    pub name: String,
    /// 渠道类型（弹窗 select 恒有值，恒发）。
    pub channel_type: String,
    /// API 基址（弹窗编辑项，恒发）。
    pub base_url: String,
    /// 绑定分组（弹窗编辑项，恒发）。
    pub groups: Vec<String>,
    /// 备注（弹窗编辑项，恒发；空串 = 显式清空备注）。
    pub remark: String,
    /// 测速模型——恒发（该列无 COALESCE，缺席即清 NULL）。
    pub test_model: Option<String>,
    /// 明文密钥列表——仅用户重输时携带；None = 字段缺席 = 后端保持现有密钥。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<String>>,
    /// 调度模型列表——仅「拉取模型」面板被使用过后携带（touched 语义）；
    /// None = 字段缺席 = 后端 COALESCE 保持现有 models。元素为模型 id 字符串，
    /// 写入会整体替换该列（保留既有条目的责任在调用方：打开面板时已把现值
    /// 预填进候选池）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub models: Option<serde_json::Value>,
}

/// 真实调用: PUT /api/channel/{key} (更新) — 最小 diff 体，语义见
/// [`UpdateChannelBody`]。
pub async fn update_channel_api(
    client: &ApiClient,
    key: &str,
    body: &UpdateChannelBody,
) -> ApiResult<ChannelDto> {
    client.put(&format!("/api/channel/{key}"), body).await
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

// 后端列表端点统一包装 `{"items":[...]}`(分组端点裸对象则直接 decode)。
#[derive(Debug, Default, serde::Deserialize)]
struct GroupItems {
    #[serde(default)]
    items: Vec<GroupDto>,
}

/// 真实调用: GET /api/group (分组列表,响应为 `{"items":[...]}`)
pub async fn list_groups_api(client: &ApiClient) -> ApiResult<Vec<GroupDto>> {
    let r: GroupItems = client.get("/api/group").await?;
    Ok(r.items)
}

// ---------------------------------------------------------------------------
// Models (别名/模型目录)
// ---------------------------------------------------------------------------

/// 模型目录 DTO — 对齐 admin-catalog /api/models 的 `ModelView` 投影。
/// `name` 是别名/对外模型名,网络拓扑的中间层(Mapping)即用它;
/// 价格/倍率字段该端点暂未提供,拓扑层一律取 0/1.0。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    pub name: String,
}

/// 后端列表端点统一包装 `{"items":[...]}`。
#[derive(Debug, Default, serde::Deserialize)]
struct ModelItems {
    #[serde(default)]
    items: Vec<ModelView>,
}

/// 真实调用: GET /api/models?size=100 (模型/别名列表,按 name 升序)
pub async fn list_models_api(client: &ApiClient) -> ApiResult<Vec<ModelView>> {
    let r: ModelItems = client.get("/api/models?size=100").await?;
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

/// 后端列表端点统一包装 `{"items":[...]}`（别名侧；与 ModelItems 区分元素类型）。
#[derive(Debug, Default, serde::Deserialize)]
struct ModelAliasItems {
    #[serde(default)]
    items: Vec<ModelAliasView>,
}

/// 真实调用: GET /api/models?size=100 (模型别名列表;后端 size clamp 1..=100)。
///
/// 响应 items 为 `ModelView`,此处只映射 key+name,其余字段由 serde 忽略。
/// 错误情况:401(未登录)、网络失败、JSON 不含 items → `ApiError`。
pub async fn list_model_aliases_api(client: &ApiClient) -> ApiResult<Vec<ModelAliasView>> {
    let r: ModelAliasItems = client.get("/api/models?size=100").await?;
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

// ---------------------------------------------------------------------------
// 系统选项 (admin-ops options)
// ---------------------------------------------------------------------------

/// 单条运行时选项,对齐后端 `OptionsService::list` 返回的 OptionView。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionView {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: String,
}

/// 后端列表端点统一包装 `{"items":[...]}`。
#[derive(Debug, Default, serde::Deserialize)]
struct OptionItems {
    #[serde(default)]
    items: Vec<OptionView>,
}

/// 真实调用: GET /api/option (全部选项,库值回退注册表默认值)
pub async fn list_options_api(client: &ApiClient) -> ApiResult<Vec<OptionView>> {
    let r: OptionItems = client.get("/api/option").await?;
    Ok(r.items)
}

/// 真实调用: PUT /api/option {key, value} — 写入(未知 key 或非法值域被拒)。
/// 返回更新后的值(后端 echo)。
pub async fn update_option_api(
    client: &ApiClient,
    key: &str,
    value: &serde_json::Value,
) -> ApiResult<serde_json::Value> {
    #[derive(Default, serde::Deserialize)]
    struct UpdateResp {
        #[serde(default)]
        value: serde_json::Value,
    }
    let req = serde_json::json!({ "key": key, "value": value });
    let r: UpdateResp = client.put("/api/option", &req).await?;
    Ok(r.value)
}
