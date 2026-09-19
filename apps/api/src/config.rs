//! 系统配置 (TOML) + PG 连接池

use std::path::Path;
use thiserror::Error;

// epay 商户配置只在 billing feature 下存在（个人形态不解析支付配置）。
#[cfg(feature = "billing")]
use billing::topup_epay::EpayMerchant;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub listen: String,
    pub database_url: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// 支付渠道配置（`[payment.epay]` 等）。整段省略 = 只用 manual 渠道，
    /// 开单指定 epay 会被服务层拒绝（payment provider not configured）。
    #[serde(default)]
    pub payment: PaymentConfig,
}

/// 支付配置（`[payment]` 段）：父段预留，将来 stripe 挂 `[payment.stripe]`。
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PaymentConfig {
    /// 易支付商户；省略 = 不注册 epay 渠道。
    /// 个人形态（无 billing feature）该字段不存在：配置反序列化与 epay 渠道
    /// 注册都属计费域；admin_router 的参数是 Option，None 是它支持的缺省。
    #[cfg(feature = "billing")]
    #[serde(default)]
    pub epay: Option<EpayMerchant>,
}

fn default_log_level() -> String {
    "info".into()
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    ReadFile(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
}

pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    let cfg: Config = toml::from_str(&content)?;
    Ok(cfg)
}

pub type PgPool = sqlx::PgPool;

pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(8)
        .connect(database_url)
        .await
}
