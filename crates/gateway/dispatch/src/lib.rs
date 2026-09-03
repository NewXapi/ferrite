//! # dispatch — 调度状态机 (热路径第 2 步)
//!
//! 从 catalog 快照中选出本次请求的执行目标。四阶段:
//!
//! ```text
//! ① 候选过滤 (candidate)   group + public_model 匹配的 RouteUnit 全集
//! ② 健康门控 (health)      剔除: 熔断冷却中 / status != 启用; 慢启动折扣权重
//! ③ 权重打分 (selector)    priority 分层 → 层内按 (weight+1)×slow_start×latency 加权随机
//! ④ 失败回退 (retry)       可重试失败 → 排除已试候选 → 回到 ②
//! ```
//!
//! 健康数据全部是**本节点内存观测** (health::MemoryHealthTable), 不跨节点同步 —
//! 设计文档原则 10: 每 Pod 独立熔断, center 只做趋势汇总。
//!
//! 参考实现:
//! - new-api `model/route_unit_selector.go` (候选/过滤/权重打分),
//!   `model/channel_model_health.go` (失败隔离状态机), `pkg/routestats/quality.go`
//!   (EWMA 延迟质量分)
//! - wildtoken `internal/proxy/matcher.go` (priority 分层加权随机),
//!   `internal/proxy/health.go` (整数健康分 + 定时渐进恢复)
//! - sub2api `backend/internal/handler/failover_loop.go` (重试状态机)
//!
//! 限流 (滑动窗口 RPM) 属于 `gateway-gate` 的 gate/ratelimit.rs — 见 gateway
//! README 分工, dispatch 不持有。
//!
//! 模块地图:
//! | 模块 | 职责 |
//! |------|------|
//! | [`candidate`] | 候选过滤与凭据解析 (选择产物 = 可转发的完整目标) |
//! | [`health`]    | 本地健康表: EWMA/熔断/冷却/慢启动 |
//! | [`selector`]  | 分层打分与加权挑选算法 |
//! | [`retry`]     | failover 排除集 + 尝试预算 |
//!
//! 组装入口: [`Dispatcher`] 把快照 + 健康表 + 选择器串成 [`Dispatch`] 实现,
//! apps/gateway 直接持有。

pub mod candidate;
pub mod health;
pub mod retry;
pub mod selector;

pub use candidate::{Candidate, STATUS_ENABLED, resolve_candidate};
pub use health::{FailureClass, HealthState, HealthTable, MemoryHealthTable};
pub use retry::{Attempt, AttemptOutcome, Failover, RetryLoop, RetryPolicy};
pub use selector::{Selector, WeightedSelector};

use contract::records::{ChannelRecord, RouteUnitRecord};
use std::collections::HashMap;
use std::sync::Arc;

/// 调度快照 — host (apps/gateway) 从 catalog sync 构建, 一次性整体替换。
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// 全部启用路由单元; select 时按 (group, model) 过滤。
    pub units: Vec<RouteUnitRecord>,
    /// 渠道索引: ChannelRecord::meta.key → 渠道 (凭据/URL 解析用)。
    pub channels: HashMap<String, ChannelRecord>,
}

/// 调度器 trait — [`Dispatcher`] 提供默认实现。
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
    /// 该分组下没有此模型的路由 (或全部被健康门控剔除 / 凭据缺失)。
    #[error("no candidate for {group}/{model}")]
    NoCandidate { group: String, model: String },
    /// catalog 快照尚未就绪 (启动期 sync 未完成首次拉取)。
    /// 决策: fail-closed (503), 与"安全配置 fail-closed"原则一致。
    #[error("catalog snapshot not ready")]
    SnapshotNotReady,
}

/// 从快照提取 (group, model) 的启用候选 — new-api getCandidatesFromCache 的
/// 等价过滤。
///
/// 别名精确匹配; 不做模糊归一 (new-api FormatMatchingModelName)。
/// ponytail: ferrite 的 RouteUnitRecord 已显式分离 public_model 与
/// upstream_model, 别名是配置数据而非推导结果; 模糊匹配留到确有需求时。
pub fn candidates_from_snapshot(
    units: &[RouteUnitRecord],
    group: &str,
    model: &str,
) -> Vec<RouteUnitRecord> {
    units
        .iter()
        .filter(|u| u.status == STATUS_ENABLED && u.group == group && u.public_model == model)
        .cloned()
        .collect()
}

/// dispatch 默认组装 — 快照 + 健康表 + 分层选择器。
///
/// `SnapshotNotReady`: 启动期 `snapshot` 为 None 时 select 直接 503,
/// 与"安全配置 fail-closed"原则一致; sync 完成首次拉取后调 `set_snapshot`
/// 整体替换 (catalog 变更经过 sync 重新构建, 与 new-api channelSyncLock 语义一致)。
pub struct Dispatcher {
    snapshot: Option<Arc<Snapshot>>,
    health: Arc<MemoryHealthTable>,
    selector: WeightedSelector,
    now_ms: Box<dyn Fn() -> u64 + Send + Sync>,
}

impl Dispatcher {
    pub fn new(snapshot: Option<Arc<Snapshot>>, health: Arc<MemoryHealthTable>) -> Self {
        Self::with_clock(snapshot, health, || {
            chrono::Utc::now().timestamp_millis().max(0) as u64
        })
    }

    /// 时钟注入 (测试确定性)。
    pub fn with_clock(
        snapshot: Option<Arc<Snapshot>>,
        health: Arc<MemoryHealthTable>,
        now_ms: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            snapshot,
            health,
            selector: WeightedSelector,
            now_ms: Box::new(now_ms),
        }
    }

    /// 整体替换调度快照 (catalog sync 完成后调用)。
    pub fn set_snapshot(&mut self, snapshot: Arc<Snapshot>) {
        self.snapshot = Some(snapshot);
    }
}

impl Dispatch for Dispatcher {
    fn select(
        &self,
        group: &str,
        public_model: &str,
        exclude: &[String],
    ) -> Result<Candidate, DispatchError> {
        let snap = self
            .snapshot
            .as_ref()
            .ok_or(DispatchError::SnapshotNotReady)?;
        let cands = candidates_from_snapshot(&snap.units, group, public_model);
        if cands.is_empty() {
            return Err(DispatchError::NoCandidate {
                group: group.to_string(),
                model: public_model.to_string(),
            });
        }
        let unit = self
            .selector
            .pick(&cands, &*self.health, exclude, (self.now_ms)())
            .ok_or(DispatchError::NoCandidate {
                group: group.to_string(),
                model: public_model.to_string(),
            })?;
        let channel = snap
            .channels
            .get(&unit.channel_key)
            .ok_or(DispatchError::NoCandidate {
                group: group.to_string(),
                model: public_model.to_string(),
            })?;
        resolve_candidate(&unit, channel).ok_or(DispatchError::NoCandidate {
            group: group.to_string(),
            model: public_model.to_string(),
        })
    }

    fn report(&self, unit_key: &str, outcome: Result<u16, FailureClass>, latency_ms: u32) {
        self.health.record(unit_key, outcome, latency_ms);
    }
}
