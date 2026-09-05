//! 网关配置 — 从 config/config.toml 加载。

use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DispatchConfig {
    #[serde(default = "default_health_fail_streak_threshold")]
    pub health_fail_streak_threshold: u32,
    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: u64,
}

fn default_health_fail_streak_threshold() -> u32 { 3 }
fn default_cooldown_seconds() -> u64 { 60 }

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MeteringConfig {
    #[serde(default)]
    pub prices: HashMap<String, ModelPrice>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RetryConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_backoff_base_ms")]
    pub backoff_base_ms: u64,
    #[serde(default)]
    pub retryable_status_codes: Vec<u16>,
}

fn default_max_attempts() -> u32 { 3 }
fn default_backoff_base_ms() -> u64 { 100 }

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct ModelPrice {
    /// $/M 输入 tokens
    pub input: f64,
    /// $/M 输出 tokens
    pub output: f64,
    /// $/M 缓存读 tokens
    pub cache: f64,
    /// 分组倍率 (GroupRecord.rate_multiplier)
    #[serde(default = "default_group_multiplier")]
    pub group_multiplier: f64,
}

fn default_group_multiplier() -> f64 { 1.0 }

#[derive(Debug, Deserialize, Clone)]
pub struct GatewayConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub dispatch: DispatchConfig,
    #[serde(default)]
    pub metering: MeteringConfig,
    #[serde(default)]
    pub retry: RetryConfig,
}

fn default_listen() -> String {
    "0.0.0.0:3000".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl GatewayConfig {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}