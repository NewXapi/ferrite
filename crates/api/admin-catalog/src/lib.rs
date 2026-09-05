//! # catalog — 配置权威 (center 控制面)
//!
//! console 的所有管理写操作都汇聚到这里。store 只管持久化, 本 crate 管业务规则。
//!
//! ## 模块地图
//!
//! | 模块 | 职责 |
//! |------|------|
//! | [`channels`]   | 渠道 CRUD + 校验 + 探活触发 |
//! | [`routes`]     | 路由单元 CRUD + 引用完整性 |
//! | [`groups`]     | 分组 CRUD + 白名单 |
//! | [`tokens`]     | 令牌生命周期 (创建含一次性明文/吊销) |
//! | [`users`]      | 用户管理 + 余额调整 (审计) |
//!
//! ## 权威规则
//!
//! 1. 每次成功写 → logical_version+1, 域 revision+1, outbox 行 (store 层保证);
//! 2. edge 禁止直写配置 — console 是唯一写入口;
//! 3. ChannelKey::secret 入库前 envelope encryption, 快照下发密文。

pub mod channels;
pub mod groups;
pub mod routes;
pub mod tokens;
pub mod users;
pub mod models;

use store::StoreError;

/// 配置服务 trait — console handler 的直接依赖。
/// 方法粒度 = 一次用户操作, 内部可组合多个 store 调用。
pub trait Catalog: Send + Sync {
    /// 创建/更新渠道。校验: base_url 合法、keys 非空、index 无重复。
    /// TODO(#422): 创建后是否立即探活? 探活走 ops::jobs (channel_probe)。
    fn upsert_channel(
        &self,
        ch: &contract::records::ChannelRecord,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// 删除渠道 → 级联 route_units 失效 (store FK CASCADE + Delete mutations)。
    fn delete_channel(&self, key: &str) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn upsert_route_unit(
        &self,
        ru: &contract::records::RouteUnitRecord,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn upsert_group(
        &self,
        g: &contract::records::GroupRecord,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// token 创建: 生成明文 (sk- + 32B random), 存哈希, 返回明文一次。
    fn create_token(
        &self,
        t: &contract::records::TokenRecord,
    ) -> impl Future<Output = Result<String, StoreError>> + Send;

    fn revoke_token(&self, key: &str) -> impl Future<Output = Result<(), StoreError>> + Send;
}
