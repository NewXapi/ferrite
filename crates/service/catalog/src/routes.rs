//! 路由单元管理 — 引用完整性。
//!
//! 这是 one-api abilities 表语义的写侧: route_units 的增删改必须与
//! channels 的生命周期同步 (渠道删除 → 级联失效)。

use store::StoreError;

/// 路由单元校验 (写前):
/// - channel_key 存在且 status=启用;
/// - key_index < channel.keys.len();
/// - (group, public_model) 无更高 priority 冲突提示 (允许, 仅告警)。
/// TODO(#425): 重建索引语义 — 渠道变更后 (group, model) 候选集的重建由
/// dispatch 快照加载时完成, 写侧只保证数据一致。
pub fn validate_route_unit(_ru: &contract::records::RouteUnitRecord) -> Result<(), StoreError> {
    todo!("TODO(#425): 引用完整性校验")
}
