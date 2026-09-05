//! tavern-settings — 用户设置读存
//!
//! 对标 SillyTavern `/api/settings/get|save`，落 `<user>/settings.json`。

pub mod http;
pub use http::{SettingsState, router};

use std::path::Path;

use tavern_storage::{self as storage, StorageError};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// 读设置。文件不存在返回空对象。
///
/// 用 `serde_json::Value` 而不是强类型结构：前端加新字段时后端不该丢掉它。
pub fn load(path: &Path) -> Result<serde_json::Value, SettingsError> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(serde_json::from_str(&raw)?),
        Err(_) => Ok(serde_json::json!({})),
    }
}

/// 整份覆盖写。
pub fn save(path: &Path, value: &serde_json::Value) -> Result<(), SettingsError> {
    storage::write_atomic(path, serde_json::to_vec_pretty(value)?.as_slice())?;
    Ok(())
}
