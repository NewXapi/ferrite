//! Billing endpoints DTO — 订阅 / 兑换 / 模型别名 相关的 console API。
//!
//! 参考: new-api /api/alias | /api/subscription | /api/redemption
//! + contract::records::billing 对应记录定义。

use crate::records::billing::{RedeemCodeRecord, SubscriptionPlanRecord};
use serde::{Deserialize, Serialize};

/// 别名 DTO — 对标 admin-api /api/alias 响应
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasDto {
    pub key: String,
    pub name: String,
    pub display_name: Option<String>,
    pub input_per_1k: Option<f64>,
    pub output_per_1k: Option<f64>,
    pub multiplier: Option<f64>,
    pub status: u8,
    pub created_at: Option<String>,
}

/// 订阅计划 DTO — 对标 admin-api /api/subscription 响应
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDto {
    pub id: Option<u32>,
    pub name: String,
    pub description: Option<String>,
    pub price: Option<f64>,
    pub quota: Option<f64>,
    pub currency_price: Option<f64>,
    pub payment_method: Option<String>,
    pub group: Option<String>,
    pub downgrade_group: Option<String>,
    pub period_val: Option<u32>,
    pub period_unit: Option<String>,
    pub reset_cycle: Option<String>,
    pub priority: Option<u32>,
    pub enabled: Option<bool>,
    pub allow_redeem: Option<bool>,
    pub allow_wallet: Option<bool>,
    pub max_per_user: Option<u32>,
    pub sort_order: Option<u32>,
    pub stripe_price_id: Option<String>,
    pub creem_product_id: Option<String>,
    pub waffo_product_id: Option<String>,
}

/// 兑换码 DTO — 对标 admin-api /api/redemption 响应
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionDto {
    pub name: String,
    pub key: String,
    pub quota: Option<f64>,
    pub status: Option<i16>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
}

impl From<&SubscriptionPlanRecord> for SubscriptionDto {
    fn from(r: &SubscriptionPlanRecord) -> Self {
        Self {
            id: None,
            name: r.name.clone(),
            description: None,
            price: r.price.parse().ok(),
            quota: Some(r.quota as f64),
            currency_price: None,
            payment_method: None,
            group: r.upgrade_group.clone(),
            downgrade_group: None,
            period_val: Some(r.duration_days),
            period_unit: Some("days".into()),
            reset_cycle: None,
            priority: None,
            enabled: Some(r.enabled),
            allow_redeem: None,
            allow_wallet: None,
            max_per_user: r.max_purchases,
            sort_order: None,
            stripe_price_id: None,
            creem_product_id: None,
            waffo_product_id: None,
        }
    }
}

impl From<&RedeemCodeRecord> for RedemptionDto {
    fn from(r: &RedeemCodeRecord) -> Self {
        Self {
            name: r.batch.clone(),
            key: r.code_hash.clone(), // 这里是 hash，前端显示可能需要另处理
            quota: Some(r.quota as f64),
            status: Some(r.redeemed_by.is_some() as i16),
            created_at: Some(r.meta.updated_at.format("%Y-%m-%d").to_string()), // TODO(#211): SyncMeta 缺 created_at，暂用 updated_at 代
            expires_at: r.expires_at.map(|dt| dt.format("%Y-%m-%d").to_string()),
        }
    }
}

/// 创建/更新模型别名请求 — 对标 admin-api /api/alias。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasUpsertRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub input_per_1k: Option<f64>,
    pub output_per_1k: Option<f64>,
    pub multiplier: Option<f64>,
    pub status: Option<u8>,
}

/// 创建/更新订阅产品请求 — 对标 admin-api /api/subscription。
///
/// quota 用 f64 与 SubscriptionDto/RedemptionDto 展示层口径一致；
/// 记录层 SubscriptionPlanRecord.quota 用 i64 内部单位，边界处换算。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionUpsertRequest {
    pub name: String,
    /// NUMERIC 语义：JSON 传字符串避免浮点误差（同 SubscriptionPlanRecord.price）。
    pub price: String,
    pub currency: String,
    pub duration_days: u32,
    pub quota: f64,
    pub upgrade_group: Option<String>,
    pub max_purchases: Option<u32>,
    pub enabled: Option<bool>,
}

/// 创建/更新兑换码请求 — 对标 admin-api /api/redemption。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionUpsertRequest {
    pub name: String,
    pub quota: f64,
    pub count: u32,
    pub expires_at: Option<String>,
}

/// 货币 DTO — 对标 admin-api /api/currency 响应
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyView {
    pub code: String,
    pub name: String,
    pub internal_rate: f64,
    pub enabled: bool,
    pub remark: String,
    /// 展示符号（¥ / $ / P）；fiat 必填，points 可空。
    #[serde(default)]
    pub symbol: String,
    /// `points` = 可扣费余额货币；`fiat` = 仅计价展示（不进余额）。
    #[serde(default)]
    pub kind: String,
    /// 展示小数位：法币 2、点数 0。
    #[serde(default)]
    pub precision: i16,
}

/// 用户余额项 DTO
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserBalanceDto {
    pub currency_code: String,
    pub amount: i64,
    /// 该货币展示符号（来自 currency_defs.symbol；旧后端无此字段时为空串）。
    #[serde(default)]
    pub symbol: String,
}

/// 用户钱包 DTO — 综合展示
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletView {
    pub user_key: String,
    pub balances: Vec<UserBalanceDto>,
    pub available_i64: i64,
}

/// 充值请求 — 对标 admin-api /api/user/topup（开单 `POST /api/user/topup/orders`）。
///
/// `provider` 金额口径按渠道不同（落库前由 `TopupService::open_topup` 折算）：
/// - manual（缺省）：`amount` = 直接入账的点数（`currency` 单位）；
/// - epay：`amount` = 要付的人民币**元**，`currency` = 入账的目标点数货币，
///   点数 = `convert(amount, "CNY", currency)`——fiat 不进余额，订单行存点数。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopUpRequest {
    pub user_key: String,
    pub currency: String,
    pub amount: i64,
    /// 支付渠道：缺省/空串 = "manual"（旧调用零变化）；真渠道如 "epay"。
    #[serde(default = "default_topup_provider")]
    pub provider: String,
}

/// `TopUpRequest.provider` 的反序列化缺省值（manual = 无真支付的默认渠道）。
fn default_topup_provider() -> String {
    "manual".into()
}

/// 奖励请求 — 对标 admin-api /api/affiliate/reward
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardRequest {
    pub kind: String,
    pub user_key: String,
    pub amount: i64,
}

/// 充值订单 DTO — `GET /api/user/topup/orders` 列表项（前端奖励面板「充值记录」）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopupOrderView {
    /// 订单 key（UUID 字符串，`billing_topups.key`）。
    pub key: String,
    /// 充值货币 code（`currency_defs.code`，如 "FREE"）。
    pub currency: String,
    /// 充值金额（该货币单位，非内部单位）。
    pub amount: i64,
    /// 订单状态：pending | settling | paid | failed | refunded。
    pub state: String,
    /// 支付渠道（"" = manual/未接真支付，如 "epay" | "stripe"）。
    pub provider: String,
    /// 创建时间（RFC3339/ISO8601 字符串，`billing_topups.created_at`）。
    pub created_at: String,
}

/// 被邀人 DTO — `GET /api/affiliate/invitees` 列表项（前端奖励面板「被邀人列表」）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteeView {
    /// 被邀人 key（UUID 字符串，`auth_users.key`）。
    pub user_key: String,
    /// 展示名：`display_name` 为空时回落 `username`。
    pub name: String,
    /// 邀请归属建立时间（RFC3339/ISO8601 字符串，`affiliate_links.created_at`）。
    pub joined_at: String,
    /// 该被邀人为邀请人带来的累计奖励额（FREE 内部单位，无奖励 = 0）。
    pub reward: i64,
}
