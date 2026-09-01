//! 路由记录 — 分组与调度单元。
//!
//! route_units 是 one-api `abilities` 表 (group, model, channel, enabled) 的演进:
//! 采纳"索引随渠道变更重建"语义; 额外引入 key_index 与 upstream_model 显式化。

use super::SyncMeta;
use serde::{Deserialize, Serialize};

/// 用户分组: 费率倍率 + 可见模型的载体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupRecord {
    pub meta: SyncMeta,
    pub id: String,
    pub display_name: String,
    /// 该组用户计费时的全局倍率 (对齐 new-api group ratio)。
    pub rate_multiplier: f64,
    /// 该组允许访问的公开模型别名白名单; 空 = 全部可见。
    pub allowed_models: Vec<String>,
}

/// 路由调度单元 — dispatch 状态机的最小选择对象。
///
/// 解耦了 new-api "channel 大表混装多模型多 key" 的问题:
/// (渠道, key, 上游模型) 三元组显式化, 公开别名到上游真名的映射在这里。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteUnitRecord {
    pub meta: SyncMeta,
    pub group: String,
    /// 客户端请求的模型名 (公开别名), 如 "gpt-4o"。
    pub public_model: String,
    pub channel_key: String, // 指向 ChannelRecord::meta.key
    /// ChannelRecord::keys 的下标。
    pub key_index: u32,
    /// 实际发往上游的模型名, 如 "gpt-4o-2024-08-06"。
    pub upstream_model: String,
    /// 数字越大越优先; dispatch 先按 priority 分层, 层内按 weight 加权。
    pub priority: i32,
    pub weight: u32,
    pub status: u8,
}
