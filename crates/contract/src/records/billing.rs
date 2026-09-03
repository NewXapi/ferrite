//! 商业化记录 — 订阅/订单/兑换 (v2 域)。
//!
//! 字段直接对应 07-database-schema.md 域四; 接 console 支付页时启用。
//! 参考: new-api internal/billing (store_subscription/topup_api/store_redemption),
//! sub2api ent/schema/{subscription_plan,user_subscription,payment_order,redeem_code}。

use super::SyncMeta;
use serde::{Deserialize, Serialize};

/// 可购买的订阅产品 (价格/时长/额度/升级组)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionPlanRecord {
    pub meta: SyncMeta,
    pub name: String,
    pub price: String,      // NUMERIC 语义; JSON 传字符串避免浮点误差
    pub currency: String,   // "CNY" | "USD"
    pub duration_days: u32,
    pub quota: i64,
    /// 购买后用户升级到的分组; None = 不变。
    pub upgrade_group: Option<String>,
    pub max_purchases: Option<u32>,
    pub enabled: bool,
}

/// 用户持有的订阅实例 (时间窗 + 消耗上限)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSubscriptionRecord {
    pub meta: SyncMeta,
    pub user_key: String,
    pub plan_key: String,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// 消耗窗口: "daily" | "weekly" | "monthly" | "custom"。
    pub window_kind: String,
    /// 窗口内消耗上限 (0 = 无限)。
    pub window_limit: i64,
    pub status: u8,
}

/// 支付订单 (状态机: pending → paid|failed|refunded, 终态不可变)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentOrderRecord {
    pub meta: SyncMeta,
    pub user_key: String,
    pub provider: String, // "epay" | "stripe" | "creem"
    pub amount: String,
    pub quota: i64,
    /// None = 纯充值单; Some = 订阅购买单。
    pub plan_key: Option<String>,
    pub state: String,
    pub provider_txn_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 兑换码 (单次核销, 行锁/CAS 保证)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemCodeRecord {
    pub meta: SyncMeta,
    /// sha256(明文码); 明文只在批量生成导出时出现。
    pub code_hash: String,
    pub quota: i64,
    pub batch: String,
    pub redeemed_by: Option<String>,
    pub redeemed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}
