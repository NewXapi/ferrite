//! 候选产物 — 选路结果的完整可执行目标。

use contract::records::{ChannelRecord, RouteUnitRecord};

/// 路由单元启用状态 (对齐 ChannelRecord::status: 1 启用 / 2 手动禁用 / 3 自动熔断)。
/// dispatch 只选 status == 1 的单元; 自动熔断由 health 内存表本地判定,
/// 与 center 的汇总熔断 (status=3) 互不干扰。
pub const STATUS_ENABLED: u8 = 1;

/// dispatch 的最终产出: 一个可转发的具体目标。
///
/// 解析完成度: forward 拿到它之后**不需要再查任何快照**。
#[derive(Debug, Clone)]
pub struct Candidate {
    /// 命中的路由单元 (原始记录, 供计量/日志引用)。
    pub unit: RouteUnitRecord,
    /// 解析后的上游凭据 (channel.keys[key_index].secret, 已解密)。
    pub secret: String,
    /// 上游 base_url (含路径前缀拼接规则, 见 forward::pipeline)。
    pub base_url: String,
    /// 上游真名 (unit.upstream_model)。
    pub upstream_model: String,
}

/// 路由单元 + 渠道快照 → 完整候选。
///
/// 凭据取自 `channel.keys` 中 `index == unit.key_index` 的那把 key
/// (new-api `GetNextEnabledKeyForIndex` 的语义: key 按下标解析, 不轮询)。
/// key 缺失或越界 → None, 上层按 NoCandidate 处理。
pub fn resolve_candidate(unit: &RouteUnitRecord, channel: &ChannelRecord) -> Option<Candidate> {
    let key = channel.keys.iter().find(|k| k.index == unit.key_index)?;
    Some(Candidate {
        unit: unit.clone(),
        secret: key.secret.clone(),
        base_url: channel.base_url.clone(),
        upstream_model: unit.upstream_model.clone(),
    })
}
