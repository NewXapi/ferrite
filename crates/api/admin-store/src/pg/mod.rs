//! PostgresStore (center) — SQLx 实现。
//!
//! 关键语义 (对齐 traits 铁律):
//! 1. put_* = 单事务内 UPSERT record + INSERT revision_outbox + watermark++;
//! 2. append_usage = INSERT ... ON CONFLICT (mutation_id) DO NOTHING (幂等);
//! 3. get_* = 单行 SELECT, codec: record ↔ PG 行的 serde 映射在本模块。
//!
//! TODO(#410): SQLx 依赖引入 (workspace 已有) + 连接池装配;
//! 每个 trait 一个 impl 块, 查询用 sqlx::query! 宏 (编译期校验)。

use crate::error::StoreError;
use crate::traits::*;

/// center 存储 — 持有 `sqlx::PgPool`。
/// TODO(#410): pub struct PostgresStore { pool: sqlx::PgPool }
pub struct PostgresStore;

impl ChannelStore for PostgresStore {
    async fn put_channel(&self, _record: &contract::records::ChannelRecord) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410): UPSERT channels + outbox (单事务)")
    }
    async fn get_channel(&self, _key: &str) -> Result<contract::records::ChannelRecord, StoreError> {
        todo!("TODO(#410): SELECT * FROM channels WHERE key=$1 → decode")
    }
    async fn delete_channel(&self, _key: &str) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410): DELETE + outbox (级联 route_units 由 FK 处理)")
    }
}

impl GroupStore for PostgresStore {
    async fn put_group(&self, _record: &contract::records::GroupRecord) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410)")
    }
    async fn get_group(&self, _id: &str) -> Result<contract::records::GroupRecord, StoreError> {
        todo!("TODO(#410)")
    }
    async fn delete_group(&self, _id: &str) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410)")
    }
}

impl RouteUnitStore for PostgresStore {
    async fn put_route_unit(&self, _record: &contract::records::RouteUnitRecord) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410)")
    }
    async fn get_route_unit(&self, _key: &str) -> Result<contract::records::RouteUnitRecord, StoreError> {
        todo!("TODO(#410)")
    }
    async fn delete_route_unit(&self, _key: &str) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410)")
    }
}

impl UserStore for PostgresStore {
    async fn put_user(&self, _record: &contract::records::UserRecord) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410)")
    }
    async fn get_user(&self, _key: &str) -> Result<contract::records::UserRecord, StoreError> {
        todo!("TODO(#410)")
    }
    async fn delete_user(&self, _key: &str) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410)")
    }
}

impl TokenStore for PostgresStore {
    async fn put_token(&self, _record: &contract::records::TokenRecord) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410)")
    }
    async fn get_token(&self, _key: &str) -> Result<contract::records::TokenRecord, StoreError> {
        todo!("TODO(#410): 热路径不经此 — edge 用快照; 本方法服务 console 自读")
    }
    async fn delete_token(&self, _key: &str) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410)")
    }
}

impl UsageStore for PostgresStore {
    async fn append_usage(&self, _event: &contract::records::UsageEventRecord) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#410): INSERT ON CONFLICT DO NOTHING → MutationId")
    }
    async fn pending_usage(&self, _limit: usize) -> Result<Vec<contract::mutations::Mutation>, StoreError> {
        todo!("TODO(#411): center 侧恒空 — pending 只在 edge WAL 有意义")
    }
}
