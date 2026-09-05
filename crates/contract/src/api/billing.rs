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
            created_at: Some(r.meta.updated_at.format("%Y-%m-%d").to_string()), // ponytail: SyncMeta 无 created_at，用 updated_at 代
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
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionUpsertRequest {
    pub name: String,
    pub price: String,
    pub currency: String,
    pub duration_days: u32,
    pub quota: i64,
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