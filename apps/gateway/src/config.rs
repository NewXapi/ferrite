//! 网关配置 — 从 `config/config.toml` 加载。
//!
//! 配置结构在此，各 crate 的运行期类型（`HealthSetting` / `ConfigPriceTable` /
//! `ConfigRetryLoop`）由 `lib.rs` 的 `build_app` 组装，配置结构不反向依赖它们的语义。

use metering::pricing::ModelPrice;
use serde::Deserialize;
use std::collections::HashMap;

/// 调度健康参数；字段名对齐 `dispatch::health::HealthSetting`。
#[derive(Debug, Deserialize, Clone)]
pub struct DispatchConfig {
    /// 连续失败达此数进入冷却。
    #[serde(default = "default_cooldown_threshold")]
    pub cooldown_threshold: u32,
    /// 冷却基础时长（秒）。
    #[serde(default = "default_cooldown_base_seconds")]
    pub cooldown_base_seconds: u64,
    /// 冷却最大时长（秒）；连续冷却时长递增到此上限。
    #[serde(default = "default_cooldown_max_seconds")]
    pub cooldown_max_seconds: u64,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            cooldown_threshold: default_cooldown_threshold(),
            cooldown_base_seconds: default_cooldown_base_seconds(),
            cooldown_max_seconds: default_cooldown_max_seconds(),
        }
    }
}

fn default_cooldown_threshold() -> u32 {
    5
}
fn default_cooldown_base_seconds() -> u64 {
    10
}
fn default_cooldown_max_seconds() -> u64 {
    60
}

/// 计量配置；`prices` 为空 = 不计费（本地单机默认）。
#[derive(Debug, Deserialize, Clone, Default)]
pub struct MeteringConfig {
    /// model 名 → 价格；键是对外模型名。
    #[serde(default)]
    pub prices: HashMap<String, ModelPrice>,
}

/// 重试策略。
///
/// 只有尝试预算：可重试的状态码由 `forward::egress::classify_status` 判定
/// （429 / 5xx 可重试，其余 4xx 致命），不在配置里开第二个判定口。
#[derive(Debug, Deserialize, Clone)]
pub struct RetryConfig {
    /// 单请求最大尝试次数（含首次）。
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
        }
    }
}

fn default_max_attempts() -> u32 {
    3
}

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
