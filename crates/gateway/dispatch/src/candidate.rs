//! 候选产物 — 选路结果的完整可执行目标。

use contract::records::RouteUnitRecord;

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
