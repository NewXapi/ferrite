//! 系统配置 (TOML) + PG 连接池

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub listen: String,
    pub database_url: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
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
