//! 网关渠道健康 — 本地 wire DTO 与客户端封装。
//!
//! 对应后端 `crates/api/admin-observe/src/gateway_health.rs` 的
//! `GET /api/gateway/health` (admin 角色白名单,响应无外层 Envelope
//! 包装——端点直接返回 `GatewayHealthView` 裸 JSON):
//!
//! ```json
//! { "items": [ { "unitKey", "channelKey", "channelName", "publicModel",
//!   "state": "cooling"|"slow_start"|"ok", "lastCoolingOutcome",
//!   "remainingCooldownMs", "slowStartFactor" } ] }
//! ```
//!
//! 语义:items 只含有记录的渠道 (健康表从未 record 过 / 已自然恢复的
//! 渠道不出现,正常态是空数组),因此 **空 items 不是错误**。
//!
//! DTO 按任务约定定义在本 crate (web 域私有),不进 `crates/contract`,
//! 避免跨域共享点变更。

use crate::{ApiClient, ApiResult};

/// 渠道健康三态 — 与后端 `HealthStateView` 同源
/// (`#[serde(rename_all = "snake_case")]`:cooling / slow_start / ok)。
///
/// 前端轮询判定用它:存在 cooling / slow_start 条目才继续轮询。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthItemState {
    /// 冷却窗口内。
    Cooling,
    /// 冷却刚结束、ramp 渐进期。
    SlowStart,
    /// 当前健康 (含历史 404/401 等 Neutral 记录但当前健康的渠道)。
    /// `#[serde(other)]`:未知 state 串 (后端加枚举值) 降级为健康态,
    /// 前端不 panic、不误报异常。
    #[serde(other)]
    Ok,
}

impl Default for HealthItemState {
    /// `data: null` 的信封兜底走 `Default` (未知 state 串走 `#[serde(other)]`
    /// 落到 `Ok`),两条降级路径都不误报异常态。
    fn default() -> Self {
        Self::Ok
    }
}

impl HealthItemState {
    /// 是否处于需要继续轮询的异常态 (cooling / slow_start)。
    pub fn is_abnormal(self) -> bool {
        !matches!(self, Self::Ok)
    }

    /// 中文标签 (面板徽标文案)。
    pub fn label(self) -> &'static str {
        match self {
            Self::Cooling => "冷却中",
            Self::SlowStart => "慢启动",
            Self::Ok => "正常",
        }
    }
}

/// 单条渠道健康视图 — 对齐后端 `HealthView` (camelCase)。
///
/// `channel_key` / `channel_name` / `public_model` 为 `None` 时:
/// unit 的 channel_key 已不在当前渠道目录 (被删 / 目录未同步),
/// 前端渲染占位而非造数。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHealthItem {
    /// 候选 key (`{channel_key}/{key_index}:{public_model}`),始终存在。
    pub unit_key: String,
    /// 渠道 UUID;join 失败为 None。
    pub channel_key: Option<String>,
    /// 渠道展示名;join 失败为 None,前端回退显示 channelKey 前 8 位。
    pub channel_name: Option<String>,
    /// 公开模型别名;join 失败为 None。
    pub public_model: Option<String>,
    /// 三态 (cooling / slow_start / ok)。
    pub state: HealthItemState,
    /// 最近一次触发冷却的 outcome (success/fatal/throttled/neutral);从未冷却 = None。
    pub last_cooling_outcome: Option<String>,
    /// 剩余冷却毫秒;未冷却 = 0。
    pub remaining_cooldown_ms: u64,
    /// 当前慢启动因子 (0.0..=1.0)。
    pub slow_start_factor: f64,
}

/// 响应体 — 对齐后端 `GatewayHealthView`。
///
/// `items` 为空 = 所有渠道健康 (正常态),面板显示虚线占位卡。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHealthView {
    /// 有记录的渠道健康行 (只含有记录,正常态为空数组)。
    pub items: Vec<GatewayHealthItem>,
}

/// 拉取网关渠道健康快照:GET `/api/gateway/health` (admin 白名单)。
///
/// 复用 [`ApiClient::get`] 的信封解码:该端点返回裸
/// `{"items":[...]}` (无 `success` 外层),命中裸 JSON 分支直接解
/// `GatewayHealthView`。401 走标准刷新重试路径。
pub async fn fetch_gateway_health(client: &ApiClient) -> ApiResult<GatewayHealthView> {
    client.get("/api/gateway/health").await
}

// ---------- billing (#179/#187): 钱包 / 拉人统计 / 兑换码 / 充值开单 ----------

/// 钱包内单币种余额行 — 对齐后端 `contract::api::billing::UserBalanceDto` (camelCase)。
///
/// `amount` 为该币种单位的原始数量 (非内部单位)。结构级 `default`:字段缺省
/// (含信封 `data: null` 走 Default 的路径) 落空值,前端渲染 0 余额而非解码炸。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserBalanceDto {
    /// 货币 code (`currency_defs.code` 外键, 如 "FREE")。
    pub currency_code: String,
    /// 余额 (该币种单位的原始数量)。
    pub amount: i64,
}

/// 用户钱包视图 — 对齐后端 `contract::api::billing::WalletView` (camelCase)。
///
/// `available_i64` 为折算后的可用内部单位 (各币种 amount × internal_rate 向下
/// 取整之和),即计费配额第二层的取数口径。`balances` 为空 = 账号尚未 seed 的
/// 正常空态,面板渲染虚线占位。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WalletView {
    /// 登录用户 UUID。
    pub user_key: String,
    /// 各币种余额行。
    pub balances: Vec<UserBalanceDto>,
    /// 折算可用内部单位。
    pub available_i64: i64,
}

/// `GET /api/user/wallet` 的原始响应体 — 后端返回裸 `{"wallet": {...}}`
/// (无 `success` 信封),走 [`crate::ApiClient`] 的裸 JSON 解码分支。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WalletResponse {
    /// 钱包视图。
    pub wallet: WalletView,
}

/// 拉人统计视图 — 对齐后端 `admin-billing/affiliate.rs::AffiliateOverview` (camelCase)。
///
/// 后端真实统计落地于 PR #188 (0009 affiliate_links 表);本分支基线上
/// `invite_count` / `total_reward` 是占位 0,合并 #188 后即为真实值。
/// 前端接的是真端点,0 即后端返回值,前端不造数。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AffiliateOverviewView {
    /// 登录用户 UUID。
    pub user_key: String,
    /// 邀请注册人数。
    pub invite_count: i64,
    /// 累计拉人奖励 (内部单位)。
    pub total_reward: i64,
}

/// `GET /api/affiliate/overview` 的原始响应体 — 裸 `{"overview": {...}}`。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AffiliateOverviewResponse {
    /// 统计视图。
    pub overview: AffiliateOverviewView,
}

/// 兑换码核销请求体 — 对齐后端 redeem.rs 本地 `TopupRequest { key }`
/// (无 serde rename,wire 上有且只有 `key` 一个字段)。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RedeemRequest {
    /// 兑换码明文。
    pub key: String,
}

/// 充值开单请求体 — 对齐后端 `contract::api::billing::TopUpRequest`
/// (camelCase: `userKey` / `currency` / `amount`)。
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenTopupRequest {
    /// 当前登录用户 UUID (后端校验只允许对本人开单)。
    pub user_key: String,
    /// 充值币种 code。
    pub currency: String,
    /// 充值金额 (该币种单位, >0)。
    pub amount: i64,
}

/// 充值开单成功响应 — 裸 `{"order_id": "..."}`。
///
/// 注意 key 逐字是 snake_case `order_id` (后端 `json!` 字面量,非 camelCase)。
/// 缺省时 `order_id = None`,调用方降级为通用文案,不假造单号。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct TopupOrder {
    /// pending 订单号 (UUID 字符串)。
    pub order_id: Option<String>,
}

/// 拉取当前用户钱包:`GET /api/user/wallet` (self),解包 `{"wallet": ...}`。
pub async fn fetch_wallet(client: &ApiClient) -> ApiResult<WalletView> {
    let resp: WalletResponse = client.get("/api/user/wallet").await?;
    Ok(resp.wallet)
}

/// 拉取拉人统计:`GET /api/affiliate/overview` (self),解包 `{"overview": ...}`。
pub async fn fetch_affiliate_overview(client: &ApiClient) -> ApiResult<AffiliateOverviewView> {
    let resp: AffiliateOverviewResponse = client.get("/api/affiliate/overview").await?;
    Ok(resp.overview)
}

/// 兑换码充值:`POST /api/user/topup` `{ key }`。
///
/// 成功响应为裸 `{"quota": <i64 内部单位>, "success": true}` (无 message 字段,
/// 信封解码必失败 → 走裸 JSON 分支);`quota` 提取与降级语义见调用方
/// `admin-page-account::api::topup_credited_quota`。
pub async fn redeem_code(client: &ApiClient, req: &RedeemRequest) -> ApiResult<serde_json::Value> {
    client.post("/api/user/topup", req).await
}

/// 充值开单:`POST /api/user/topup/order` — 只建 pending 订单 (支付 provider
/// 为占位,无支付页),入账需 admin 手工 settle (`POST /api/user/topup/{key}/settle`)。
///
/// 路径契约:兑换码核销与充值开单原本都注册在 `POST /api/user/topup` 且 axum
/// merge 同 path 同 method 直接 panic;Main 定稿 hotfix 后开单挪到 `/order`
/// 子路径 (2026-09-14 契约),后端合入前该端点 404,面板按错误态诚实展示。
pub async fn open_topup(client: &ApiClient, req: &OpenTopupRequest) -> ApiResult<TopupOrder> {
    client.post("/api/user/topup/orders", req).await
}
