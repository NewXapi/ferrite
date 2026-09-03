//! identity 快照持有 — admission 的唯一数据源。
//!
//! 数据流: service/sync 拉取 identity 域增量 → apps/gateway 组装 →
//! `ArcSwap<TokenSnapshot>::store` 原子替换。admission 所有读操作都是
//! 快照的 wait-free 读。
//!
//! 参考: new-api one-api model/cache.go 的思路 (整表重建), 但用不可变
//! Arc 快照替代 RWLock 嵌套 map (wildtoken 已验证的模式)。

use std::collections::HashMap;

use contract::records::{TokenRecord, UserRecord};

/// identity 快照 — 一次构建, 多次原子读。
///
/// key_hash → token 是热路径; 其余是低频查询 (console 自读不走这里)。
#[derive(Debug, Clone, Default)]
pub struct TokenSnapshot {
    /// SHA-256(明文key) → token 记录。热路径唯一索引。
    pub by_key_hash: HashMap<String, TokenRecord>,
    /// user_key → user 记录。
    pub users: HashMap<String, UserRecord>,
    /// 快照构建时的 identity 域 revision (对账/诊断用)。
    pub revision: u64,
}

impl TokenSnapshot {
    /// 热路径: 按 key 哈希查找令牌。
    pub fn find_token(&self, key_hash: &str) -> Option<&TokenRecord> {
        self.by_key_hash.get(key_hash)
    }

    /// 关联用户查找 (令牌校验通过后)。
    pub fn find_user(&self, user_key: &str) -> Option<&UserRecord> {
        self.users.get(user_key)
    }
}

/// 快照容器 trait — apps/gateway 用 arc_swap::ArcSwap 实现。
///
/// 抽成 trait 的原因: admission 单测需要可替换的假快照源。
/// TODO(#301): 替换时保留旧 Arc 存活 (在途请求安全) — ArcSwap 天然保证。
pub trait SnapshotStore: Send + Sync {
    fn load(&self) -> std::sync::Arc<TokenSnapshot>;
    fn store(&self, snapshot: std::sync::Arc<TokenSnapshot>);
}
