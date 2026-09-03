//! 存储端口 — 每域一个窄 trait。
//!
//! 设计 (调查结论): new-api 的 store_* 按域拆文件 (store_channel/store_user/
//! store_log...), 我们按同样粒度拆 trait 而不是一个大 DomainStore — 域演进互不牵连。
//! 方法同时负责: 写 record + 追加 mutation (同一事务/写批, 铁律 2)。

use contract::mutations::MutationId;
use contract::records::{
    ChannelRecord, GroupRecord, RouteUnitRecord, TokenRecord, UsageEventRecord, UserRecord,
};

use crate::error::StoreError;

pub trait ChannelStore: Send + Sync {
    fn put_channel(&self, record: &ChannelRecord) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
    fn get_channel(&self, key: &str) -> impl Future<Output = Result<ChannelRecord, StoreError>> + Send;
    fn delete_channel(&self, key: &str) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
}

pub trait GroupStore: Send + Sync {
    fn put_group(&self, record: &GroupRecord) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
    fn get_group(&self, id: &str) -> impl Future<Output = Result<GroupRecord, StoreError>> + Send;
    fn delete_group(&self, id: &str) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
}

pub trait RouteUnitStore: Send + Sync {
    fn put_route_unit(&self, record: &RouteUnitRecord) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
    fn get_route_unit(&self, key: &str) -> impl Future<Output = Result<RouteUnitRecord, StoreError>> + Send;
    fn delete_route_unit(&self, key: &str) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
}

pub trait UserStore: Send + Sync {
    fn put_user(&self, record: &UserRecord) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
    fn get_user(&self, key: &str) -> impl Future<Output = Result<UserRecord, StoreError>> + Send;
    fn delete_user(&self, key: &str) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
}

pub trait TokenStore: Send + Sync {
    fn put_token(&self, record: &TokenRecord) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
    fn get_token(&self, key: &str) -> impl Future<Output = Result<TokenRecord, StoreError>> + Send;
    fn delete_token(&self, key: &str) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
}

/// 用量事件: append-only, 无 update/delete。
/// center 实现按 MutationId 幂等去重; edge 实现写本地 WAL。
pub trait UsageStore: Send + Sync {
    fn append_usage(&self, event: &UsageEventRecord) -> impl Future<Output = Result<MutationId, StoreError>> + Send;
    /// edge 侧: 取出未 ACK 的事件批 (cursor 推进由 sync 层管理)。
    /// TODO(#411): batch 参数 — 上限条数 + 最老时间戳, 二者任一满足即返回。
    fn pending_usage(&self, limit: usize) -> impl Future<Output = Result<Vec<contract::mutations::Mutation>, StoreError>> + Send;
}
