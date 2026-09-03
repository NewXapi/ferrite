//! 打分与挑选 — priority 分层 + 层内 weight 加权随机 + 健康微调。
//!
//! 算法 (对齐 wildtoken `matcher.go::selectWeightedByPriority` +
//! new-api `route_unit_selector.go::scoreCandidates`):
//! 1. 过滤: status=启用 且 health::is_selectable 且不在 exclude 集;
//! 2. 分层: priority DESC, 取第一个存在正权重候选的层 (wildtoken fallthrough:
//!    顶层全冷却 → 落到下一层, 而不是整体失败);
//! 3. 加权随机: 层内按 effective = (weight+1) × slow_start × latency_quality
//!    抽取 (new-api routingBaseWeight 的 +1 保证 weight=0 也以最低份额参与,
//!    且权重单调: 配置越大份额越大)。
//!
//! affinity (new-api track_affinity.go): 同会话固定候选 — V2,
//! 需要 session_hash 提取规则定型后做。

use crate::candidate::STATUS_ENABLED;
use crate::health::{self, HealthTable};
use contract::records::RouteUnitRecord;
use rand::Rng;

/// 选择器 trait — 输入候选 + 健康表, 输出唯一选择。
pub trait Selector: Send + Sync {
    /// 从候选集中选出一个。全部被门控剔除 → None (上层报 NoCandidate)。
    ///
    /// 返回借用而非所有权 (ocr #8): 候选在快照中已由 Arc 持有,
    /// 热路径每请求零克隆, 只有上层最终 resolve_candidate 时才产出一个 owned。
    fn pick<'a>(
        &self,
        units: &[&'a RouteUnitRecord],
        health: &dyn HealthTable,
        exclude: &[String],
        now_ms: u64,
    ) -> Option<&'a RouteUnitRecord>;
}

/// 分层加权选择器 — dispatch 的默认实现。
pub struct WeightedSelector;

impl Selector for WeightedSelector {
    fn pick<'a>(
        &self,
        units: &[&'a RouteUnitRecord],
        health: &dyn HealthTable,
        exclude: &[String],
        now_ms: u64,
    ) -> Option<&'a RouteUnitRecord> {
        let eligible: Vec<&RouteUnitRecord> = units
            .iter()
            .copied()
            .filter(|u| u.status == STATUS_ENABLED)
            .filter(|u| !exclude.iter().any(|k| k == &u.meta.key))
            .filter(|u| health.is_selectable(&u.meta.key, now_ms))
            .collect();
        if eligible.is_empty() {
            return None;
        }

        // 按 priority 分层, 高优先层先试。
        let mut by_priority: Vec<(i32, Vec<&RouteUnitRecord>)> = Vec::new();
        for u in &eligible {
            match by_priority.iter_mut().find(|(p, _)| *p == u.priority) {
                Some((_, tier)) => tier.push(u),
                None => by_priority.push((u.priority, vec![u])),
            }
        }
        by_priority.sort_by(|a, b| b.0.cmp(&a.0));

        for (_, tier) in by_priority {
            let weighted: Vec<(&RouteUnitRecord, f64)> = tier
                .iter()
                .copied()
                .map(|u| {
                    let w = health::routing_weight(u.weight, &health.get(&u.meta.key), now_ms);
                    (u, w)
                })
                .filter(|(_, w)| *w > 0.0)
                .collect();
            if weighted.is_empty() {
                continue; // 这一层全被健康门控 → 落下一层
            }
            return Some(pick_weighted(&weighted));
        }
        None
    }
}

/// 累积加权随机 (new-api selectByWeight / wildtoken weightedIndex 的同一算法)。
fn pick_weighted<'a>(weighted: &[(&'a RouteUnitRecord, f64)]) -> &'a RouteUnitRecord {
    let total: f64 = weighted.iter().map(|(_, w)| w).sum();
    let mut target = rand::thread_rng().gen_range(0.0..total);
    for (u, w) in weighted {
        target -= w;
        if target < 0.0 {
            return u;
        }
    }
    // 浮点累计误差兜底 (new-api 的 last-candidate safety net)。
    weighted.last().unwrap().0
}
