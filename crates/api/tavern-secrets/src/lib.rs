//! tavern-secrets — 用户自己的上游密钥
//!
//! 对标 SillyTavern `src/endpoints/secrets.js`，落 `<user>/secrets.json`。
//!
//! 明文只在进程内流动：对外 API 只回「是否已配置」。

pub mod http;
pub use http::{SecretsState, router};

use std::collections::BTreeMap;
use std::path::Path;

use tavern_storage::{self as storage, StorageError};

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

type Map = BTreeMap<String, String>;

fn read_map(path: &Path) -> Result<Map, SecretError> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(serde_json::from_str(&raw).unwrap_or_default()),
        Err(_) => Ok(Map::new()),
    }
}

/// 读明文。**只给后端内部用**，不要直接塞进 HTTP 响应。
pub fn read(path: &Path, key: &str) -> Result<Option<String>, SecretError> {
    Ok(read_map(path)?.get(key).cloned())
}

pub fn write(path: &Path, key: &str, value: &str) -> Result<(), SecretError> {
    let mut map = read_map(path)?;
    map.insert(key.to_string(), value.to_string());
    persist(path, &map)
}

pub fn remove(path: &Path, key: &str) -> Result<(), SecretError> {
    let mut map = read_map(path)?;
    map.remove(key);
    persist(path, &map)
}

/// 对外状态：只暴露键名和是否已配置，不回显明文。
pub fn state(path: &Path) -> Result<BTreeMap<String, bool>, SecretError> {
    Ok(read_map(path)?
        .into_iter()
        .map(|(k, v)| (k, !v.is_empty()))
        .collect())
}

fn persist(path: &Path, map: &Map) -> Result<(), SecretError> {
    storage::write_atomic(path, serde_json::to_vec(map)?.as_slice())?;
    Ok(())
}
