//! # store — 存储端口 (capability CRUD)
//!
//! ```text
//! capability CRUD API
//!   → Store port (本模块)
//!       ├── PostgresStore (SQLx, center)     ← migrations/ DDL 在此 crate 内
//!       └── EmbeddedStore (Fjall, edge)      ← key encoding 在此 crate 内
//! ```
//!
//! 铁律 (02-b 路线图):
//! 1. handler / 业务代码**永不**出现 backend 分支 — 同一 trait 两个实现;
//! 2. 写 record 与写 mutation journal 必须在**同一事务/写批**内原子完成 (D2);
//! 3. 重复 append 同一 MutationId 必须幂等去重。

use contract::mutations::{Mutation, MutationId};
use contract::records::{
    ChannelRecord, GroupRecord, RouteUnitRecord, TokenRecord, UsageEventRecord, UserRecord,
};

/// 存储错误的统一形状 — 不暴露底层 SQL/KV 细节。
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("record not found: {0}")]
    NotFound(String),
    /// 违反唯一约束 / 域不匹配 / 校验失败。
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("backend failure: {0}")]
    Backend(String),
}

/// 每个同步域一个窄 trait, 而不是一个巨型 trait — 按域演进互不牵连。
/// 每个方法同时负责: 写 record + 追加 mutation (原子)。
///
/// TODO(#410): 列表/分页方法签名 — 后续按 console 的查询需求加 (filter/page),
/// 现在只有单点 CRUD, 避免提前发明查询 DSL。
pub trait ChannelStore: Send + Sync {
    fn put_channel(&self, record: &ChannelRecord) -> Result<MutationId, StoreError>;
    fn get_channel(&self, key: &str) -> Result<ChannelRecord, StoreError>;
    fn delete_channel(&self, key: &str) -> Result<MutationId, StoreError>;
}

pub trait GroupStore: Send + Sync {
    fn put_group(&self, record: &GroupRecord) -> Result<MutationId, StoreError>;
    fn get_group(&self, id: &str) -> Result<GroupRecord, StoreError>;
    fn delete_group(&self, id: &str) -> Result<MutationId, StoreError>;
}

pub trait RouteUnitStore: Send + Sync {
    fn put_route_unit(&self, record: &RouteUnitRecord) -> Result<MutationId, StoreError>;
    fn get_route_unit(&self, key: &str) -> Result<RouteUnitRecord, StoreError>;
    fn delete_route_unit(&self, key: &str) -> Result<MutationId, StoreError>;
}

pub trait UserStore: Send + Sync {
    fn put_user(&self, record: &UserRecord) -> Result<MutationId, StoreError>;
    fn get_user(&self, key: &str) -> Result<UserRecord, StoreError>;
    fn delete_user(&self, key: &str) -> Result<MutationId, StoreError>;
}

pub trait TokenStore: Send + Sync {
    fn put_token(&self, record: &TokenRecord) -> Result<MutationId, StoreError>;
    fn get_token(&self, key: &str) -> Result<TokenRecord, StoreError>;
    fn delete_token(&self, key: &str) -> Result<MutationId, StoreError>;
}

/// 用量事件: append-only, 无 update/delete。
/// center 实现按 MutationId 幂等去重; edge 实现写本地 WAL。
pub trait UsageStore: Send + Sync {
    fn append_usage(&self, event: &UsageEventRecord) -> Result<MutationId, StoreError>;
    /// edge 侧: 取出未 ACK 的事件批 (cursor 推进由 sync 层管理)。
    /// TODO(#411): batch 参数 — 上限条数 + 最老时间戳, 二者任一满足即返回。
    fn pending_usage(&self, limit: usize) -> Result<Vec<Mutation>, StoreError>;
}
