//! # dispatch — 调度状态机 (热路径第 2 步)
//!
//! 从 catalog 快照中选出本次请求的执行目标。四阶段:
//!
//! ```text
//! ① 候选过滤 (candidate)   group + public_model 匹配的 RouteUnit 全集
//! ② 健康门控 (health)      剔除: 熔断冷却中 / 并发已满 / status != 启用
//! ③ 权重打分 (selector)    priority 分层 → 层内按 weight 加权随机 (EWMA 微调)
//! ④ 失败回退 (retry)       可重试失败 → 排除已试候选 → 回到 ②
//! ```
//!
//! 健康数据全部是**本节点内存观测** (health::HealthTable), 不跨节点同步 —
//! 设计文档原则 10: 每 Pod 独立熔断, center 只做趋势汇总。
//!
//! 模块地图:
//! | 模块 | 职责 |
//! |------|------|
//! | [`candidate`] | 选择产物 (可转发的完整目标) |
//! | [`health`]    | 本地健康表: EWMA/熔断/冷却 |
//! | [`selector`]  | 打分与挑选算法 |
//! | [`retry`]     | failover 重试循环编排 |

pub mod candidate;
pub mod health;
pub mod retry;
pub mod selector;

pub use candidate::Candidate;
pub use health::FailureClass;
pub use selector::Selector;

use contract::records::RouteUnitRecord;

/// 调度器 trait — apps/gateway 用 catalog 快照 + health 表实现。
pub trait Dispatch: Send + Sync {
    /// 为一次请求选出候选。
    ///
    /// `exclude`: 已失败的候选 key 集合 (failover 回退时排除)。
    fn select(
        &self,
        group: &str,
        public_model: &str,
        exclude: &[String],
    ) -> Result<Candidate, DispatchError>;

    /// 转发结束后回报结果, 驱动健康状态演化 (EWMA / 熔断开关)。
    fn report(&self, unit_key: &str, outcome: Result<u16, FailureClass>, latency_ms: u32);
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// 该分组下没有此模型的路由 (或全部被健康门控剔除)。
    #[error("no candidate for {group}/{model}")]
    NoCandidate { group: String, model: String },
    /// catalog 快照尚未就绪 (启动期 sync 未完成首次拉取)。
    /// 决策: fail-closed (503), 与"安全配置 fail-closed"原则一致。
    #[error("catalog snapshot not ready")]
    SnapshotNotReady,
}

/// dispatch 输入侧: 从快照取出 (group, model) 的候选单元集。
/// TODO(#306): 候选提取实现 — 在 RouteUnit 快照上按 (group, model) 建索引。
pub fn candidates_from_snapshot(
    _units: &[RouteUnitRecord],
    _group: &str,
    _model: &str,
) -> Vec<RouteUnitRecord> {
    Vec::new()
}
