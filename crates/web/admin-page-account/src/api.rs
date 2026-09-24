//! 账户页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 全部入口为 `*_api`: async 真实后端调用 (走 `client::ApiClient`), 覆盖
//! 密钥 / 用量 / 用户信息 / 会话 / 设置 / 钱包 / 拉人统计 / 充值开单。

// ---- 邀请奖励面板 ----

/// 真实调用: GET /api/user/topup/orders — 当前用户充值订单列表,
/// 裸 `{"items":[...]}` 信封,空数组 = 无订单的正常空态。
pub async fn fetch_recharges_api(client: &ApiClient) -> ApiResult<Vec<TopupOrderView>> {
    client::fetch_topup_orders(client).await
}

/// 真实调用: GET /api/affiliate/invitees — 当前用户被邀人列表,
/// 裸 `{"items":[...]}` 信封,空数组 = 无人受邀的正常空态。
pub async fn fetch_invitees_api(client: &ApiClient) -> ApiResult<Vec<InviteeView>> {
    client::fetch_invitees(client).await
}

use client::{ApiClient, ApiResult};
use contract::api::token::{
    CreateTokenRequest, CreateTokenResult, TokenDto, TokenList, UpdateTokenRequest,
};
use contract::api::usage::{UsageLogPage, UsageStatDto};
use contract::api::user::{SessionDto, UpdateSelfRequest, UserDto};

/// 真实调用: GET /api/token (owner 模式, 后端按 token 属主过滤, 无 query 参数)。
/// 列表为 `{items}` 信封, 拆包后返回 `Vec<TokenDto>`。
pub async fn list_tokens_api(client: &ApiClient) -> ApiResult<Vec<TokenDto>> {
    let page = client.get::<TokenList>("/api/token").await?;
    Ok(page.items)
}

/// 真实调用: GET /api/token/auto-groups — 可自动分配的启用分组名
/// (`{items: [name]}` 信封, 后端 admin-catalog tokens auto_groups)。
/// 密钥卡「分组」下拉切换菜单的数据源; 非数组/缺失按空列表处理。
pub async fn list_auto_groups_api(client: &ApiClient) -> ApiResult<Vec<String>> {
    let resp: serde_json::Value = client.get("/api/token/auto-groups").await?;
    Ok(resp
        .get("items")
        .and_then(|items| serde_json::from_value::<Vec<String>>(items.clone()).ok())
        .unwrap_or_default())
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

pub use client::{
    AffiliateOverviewView, InviteeView, OpenTopupRequest, RedeemRequest, TopupOrderView,
    WalletView, fetch_affiliate_overview as fetch_affiliate_overview_api,
    fetch_wallet as fetch_wallet_api, open_topup as open_topup_api, redeem_code as redeem_code_api,
};

/// 从 POST /api/user/topup 的成功响应提取入账额度。
///
/// 后端成功返回 `{"quota": <i64 内部单位>, "success": true}`; 字段缺失、
/// 非整数 (含浮点) 或为 null 时返回 `None`, 调用方降级为通用成功文案,
/// 不假造入账数值。
pub fn topup_credited_quota(resp: &serde_json::Value) -> Option<i64> {
    resp.get("quota").and_then(|q| q.as_i64())
}

/// 拼装邀请链接: `{origin}/register?invite={code}`。
///
/// `code` 优先 aff_code 短码 (短、不可枚举; 后端 `resolve_invite_code` 查
/// `auth_users.aff_code` 解析), 未生成/空串回落 `user_key` (UUID, 旧链接
/// 兼容——后端解析的 UUID 分支天然兜底)。参数名沿用 `invite`: 落地页
/// admin-page-auth 只读 `invite`, 换名要二改落地页 (todo 项 2 明确「保留
/// invite 参数名」可选)。纯字符串拼装, 无副作用, 便于 tests/ 直接断言。
pub fn invite_link(origin: &str, user_key: &str, aff_code: Option<&str>) -> String {
    let code = aff_code.filter(|c| !c.is_empty()).unwrap_or(user_key);
    format!("{origin}/register?invite={code}")
}
