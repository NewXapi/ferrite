//! `snapshot` —— 数据源：identity / quota / ip-policy 持有
//!
//! 所有 gate 的只读快照都来自这里，由 `service::sync` 推送更新。

use arc_swap::ArcSwap;
use std::sync::Arc;

/// Token 快照（来自 `service::sync`）
pub struct TokenSnapshot {
    /// sha256(raw_key) → TokenRecord
    by_hash: dashmap::DashMap<[u8; 32], TokenRecord>,
}

impl Default for TokenSnapshot {
    fn default() -> Self {
        Self { by_hash: dashmap::DashMap::new() }
    }
}

impl TokenSnapshot {
    pub fn lookup(&self, hash: &[u8; 32]) -> Option<TokenRecord> {
        self.by_hash.get(hash).map(|r| r.clone())
    }
}

#[derive(Debug, Clone)]
pub struct TokenRecord {
    pub id: i64,
    pub user_id: i64,
    pub group: Option<String>,
    pub enabled: bool,
    pub expires_at: Option<i64>,
    pub allowed_models: Option<Vec<String>>,
    pub auth_version: u64,
}

/// User 快照
pub struct UserSnapshot {
    by_id: dashmap::DashMap<i64, UserRecord>,
}

impl Default for UserSnapshot {
    fn default() -> Self {
        Self { by_id: dashmap::DashMap::new() }
    }
}

impl UserSnapshot {
    pub fn lookup(&self, user_id: i64) -> Option<UserRecord> {
        self.by_id.get(&user_id).map(|r| r.clone())
    }
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: i64,
    pub enabled: bool,
    pub group: String,
    pub auth_version: u64,
}
