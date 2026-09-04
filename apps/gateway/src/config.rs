//! 网关配置 — 从 config/config.toml 加载。

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct GatewayConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
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
