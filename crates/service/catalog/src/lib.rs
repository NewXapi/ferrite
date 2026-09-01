//! # catalog — 配置权威 (center 控制面)
//!
//! console 的所有管理写操作都汇聚到这里。职责:
//!
//! 1. **业务校验**: 渠道连通性字段、route-unit 引用完整性 (channel 存在、
//!    model 别名不冲突) 等 — store 层只管持久化, 不管业务规则;
//! 2. **逻辑版本推进**: 每次成功写操作 → 该记录 logical_version +1, 域 revision +1;
//! 3. **凭据加密**: ChannelKey::secret 在入库前加密 (envelope encryption),
//!    快照下发的是密文, edge 持有解密 key (TODO(#420): KMS 方案 — 静态 key 先行)。
//!
//! 与 new-api 的差异: 不允许 edge 直写任何配置; console 是唯一写入口。

use contract::records::{ChannelRecord, GroupRecord, RouteUnitRecord, TokenRecord};
use store::StoreError;

/// 配置服务 trait — console handler 的直接依赖。
/// 方法粒度 = 一次用户操作 (不是一次 CRUD), 内部可组合多个 store 调用。
pub trait Catalog: Send + Sync {
    /// 创建/更新渠道。校验: base_url 合法、keys 非空、index 无重复。
    /// TODO(#422): 创建后是否立即探活 (ping 上游)? new-api 有"测试"按钮 — 交互期行为待定。
    fn upsert_channel(
        &self,
        ch: &ChannelRecord,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// 删除渠道 → 级联: 其下所有 route-unit 同步失效 (生成 Delete mutation)。
    fn delete_channel(&self, key: &str)
    -> impl Future<Output = Result<(), StoreError>> + Send;

    /// 增加 route-unit。校验: 引用的 channel_key 存在、key_index 越界检查。
    fn upsert_route_unit(
        &self,
        ru: &RouteUnitRecord,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// 分组 CRUD (allowed_models 变更影响 admission 的模型白名单)。
    fn upsert_group(&self, g: &GroupRecord)
    -> impl Future<Output = Result<(), StoreError>> + Send;

    /// token 生命周期: 创建 (响应含一次性明文) / 吊销。
    fn create_token(&self, t: &TokenRecord)
    -> impl Future<Output = Result<String, StoreError>> + Send;

    fn revoke_token(&self, key: &str)
    -> impl Future<Output = Result<(), StoreError>> + Send;
}

// TODO(#423): 用户余额调整 (admin 面板) — 需要审计事件 (操作者/原因),
// 复用 Usage 域还是新增审计域? 等 admin 面板接真后端时定。
