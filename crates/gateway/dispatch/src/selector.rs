//! 打分与挑选 — priority 分层 + weight 加权随机 + EWMA 微调。
//!
//! 算法 (对齐 new-api select_route_unit.go + wildtoken matcher.go):
//! 1. 过滤: status=启用 且 health::is_selectable 且不在 exclude 集;
//! 2. 分层: 取最高 priority 层 (one-api 的 MAX(priority) 语义);
//! 3. 加权随机: 层内按 weight 抽取, effective_weight = weight × slow_start。
//!
//! affinity (new-api track_affinity.go): 同会话固定候选 — V2,
//! 需要 session_hash 提取规则定型后做。

use crate::candidate::Candidate;
use crate::health::HealthTable;
use contract::records::RouteUnitRecord;

/// 选择器 trait — 输入候选 + 健康表, 输出唯一选择。
pub trait Selector: Send + Sync {
    /// 从候选集中选出一个。全部被门控剔除 → None (上层报 NoCandidate)。
    fn pick(
        &self,
        units: &[RouteUnitRecord],
        health: &dyn HealthTable,
        exclude: &[String],
        now_ms: u64,
    ) -> Option<RouteUnitRecord>;
}

/// 解析候选 → 可转发目标 (查 catalog 快照补全 secret/base_url)。
/// TODO(#306): 快照上建 channel_key → (secret 解密, base_url) 索引。
pub fn resolve_candidate(_unit: &RouteUnitRecord) -> Candidate {
    todo!("TODO(#306): 从 catalog 快照解析凭据与 base_url")
}
