//! # dispatch — 调度状态机 (热路径第 2 步)
//!
//! 从 catalog 快照中选出本次请求的执行目标。四阶段:
//!
//! ```text
//! ① 候选过滤 (candidate)   group + public_model 匹配的 RouteUnit 全集
//! ② 健康门控 (health)      剔除: 熔断冷却中 / status != 启用; 慢启动折扣权重
//! ③ 限流门控 (ratelimit)   按 unit.meta.key 的滑动窗口剔除超额单元
//! ④ 权重打分 (selector)    priority 分层 → 层内按 (weight+1)×slow_start×latency 加权随机
//! ⑤ 失败回退 (retry)       可重试失败 → 排除已试候选 → 回到 ②
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
//!   `internal/proxy/health.go` (整数健康分 + 定时渐进恢复),
//!   `internal/ratelimit/{limiter.go,parser.go}` (按 key 滑动窗口)
//! - sub2api `backend/internal/handler/failover_loop.go` (重试状态机)
//!
//! 模块地图:
//! | 模块 | 职责 |
//! |------|------|
//! | [`candidate`] | 候选过滤与凭据解析 (选择产物 = 可转发的完整目标) |
//! | [`health`]    | 本地健康表: EWMA/熔断/冷却/慢启动 |
//! | [`ratelimit`] | 滑动窗口限流: 按 unit.key 保护上游频率 |
//! | [`selector`]  | 分层打分与加权挑选算法 |
//! | [`retry`]     | failover 排除集 + 尝试预算 |
//!
//! 组装入口: [`Dispatcher`] 把快照 + 健康表 + 限流 + 选择器串成 [`Dispatch`] 实现,
//! apps/gateway 直接持有。

pub mod candidate;
pub mod health;
pub mod ratelimit;
pub mod retry;
pub mod selector;

pub use candidate::{Candidate, STATUS_ENABLED, resolve_candidate};
pub use health::{FailureClass, HealthState, HealthTable, MemoryHealthTable};
pub use ratelimit::{RateLimitSpec, SlidingWindow};
pub use retry::{Attempt, AttemptOutcome, Failover, RetryLoop, RetryPolicy};
pub use selector::{Selector, WeightedSelector};

use contract::records::{ChannelRecord, RouteUnitRecord};
use rand::SeedableRng;
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
    /// 全候选因限流被剔除 — 上层映射 429; 与 NoCandidate=503 区分,
    /// 让客户端区分"暂时无可用单元"与"上游频率被触顶"。
    #[error("all candidates rate-limited for {group}/{model}")]
    RateLimited { group: String, model: String },
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
pub fn candidates_from_snapshot<'a>(
    units: &'a [RouteUnitRecord],
    group: &str,
    model: &str,
) -> Vec<&'a RouteUnitRecord> {
    units
        .iter()
        .filter(|u| u.status == STATUS_ENABLED && u.group == group && u.public_model == model)
        .collect()
}

/// dispatch 默认组装 — 快照 + 健康表 + 限流 + 分层选择器。
///
/// `SnapshotNotReady`: 启动期 `snapshot` 为 None 时 select 直接 503,
/// 与"安全配置 fail-closed"原则一致; sync 完成首次拉取后调 `set_snapshot`
/// 整体替换 (catalog 变更经过 sync 重新构建, 与 new-api channelSyncLock 语义一致)。
///
/// `limits`: 可选按 unit.meta.key 的限流规格。空 → 限流门控完全跳过,
/// 行为与历史实现一致 (forward 兼容既有调用方)。
pub struct Dispatcher {
    snapshot: Option<Arc<Snapshot>>,
    health: Arc<MemoryHealthTable>,
    selector: WeightedSelector,
    now_ms: Box<dyn Fn() -> u64 + Send + Sync>,
    /// unit.meta.key → 限流规格。未配置 = 不限流。
    limits: HashMap<String, RateLimitSpec>,
    /// 滑动窗口状态机; 即便 limits 为空也要有实例 (set_limits 入口留好)。
    rl: Arc<SlidingWindow>,
}

impl Dispatcher {
    pub fn new(snapshot: Option<Arc<Snapshot>>, health: Arc<MemoryHealthTable>) -> Self {
        Self::with_limits(
            snapshot,
            health,
            HashMap::new(),
            Arc::new(SlidingWindow::new()),
        )
    }

    /// 时钟注入 (测试确定性)。
    pub fn with_clock(
        snapshot: Option<Arc<Snapshot>>,
        health: Arc<MemoryHealthTable>,
        now_ms: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self::with_limits_and_clock(
            snapshot,
            health,
            HashMap::new(),
            Arc::new(SlidingWindow::new()),
            now_ms,
        )
    }

    /// 注入限流规格表 (按 unit.meta.key) 与共享窗口状态机。
    pub fn with_limits(
        snapshot: Option<Arc<Snapshot>>,
        health: Arc<MemoryHealthTable>,
        limits: HashMap<String, RateLimitSpec>,
        rl: Arc<SlidingWindow>,
    ) -> Self {
        Self::with_limits_and_clock(snapshot, health, limits, rl, || {
            chrono::Utc::now().timestamp_millis().max(0) as u64
        })
    }

    /// 限流 + 时钟一并注入 (测试全确定性)。
    pub fn with_limits_and_clock(
        snapshot: Option<Arc<Snapshot>>,
        health: Arc<MemoryHealthTable>,
        limits: HashMap<String, RateLimitSpec>,
        rl: Arc<SlidingWindow>,
        now_ms: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            snapshot,
            health,
            selector: WeightedSelector,
            now_ms: Box::new(now_ms),
            limits,
            rl,
        }
    }

    /// 整体替换调度快照 (catalog sync 完成后调用)。
    pub fn set_snapshot(&mut self, snapshot: Arc<Snapshot>) {
        self.snapshot = Some(snapshot);
    }

    /// 整体替换限流规格表 (限流配置变更 / 启动期一次性载入)。
    pub fn set_limits(&mut self, limits: HashMap<String, RateLimitSpec>) {
        self.limits = limits;
    }
}

/// 构造 NoCandidate (ocr #9: select 内四处重复, 提出来)。
fn no_candidate(group: &str, model: &str) -> DispatchError {
    DispatchError::NoCandidate {
        group: group.to_string(),
        model: model.to_string(),
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
            return Err(no_candidate(group, public_model));
        }
        // 渠道门控 (new-api filterCandidatesByChannelStatusAndKey 等价):
        // 快照里 channel 缺失或 status != 启用 → 单元不可调度。
        // 放宽判定在候选阶段而非 pick 后, 避免选中一个必败的单元浪费尝试。
        let cands: Vec<&RouteUnitRecord> = cands
            .into_iter()
            .filter(|u| {
                snap.channels
                    .get(&u.channel_key)
                    .is_some_and(|c| c.status == STATUS_ENABLED)
            })
            .collect();
        if cands.is_empty() {
            return Err(no_candidate(group, public_model));
        }
        // 限流门控 — pick 一个 → 检查它 → 超限则排除重选 (wildtoken
        // runProxyAttempts 语义: 只对"被选中"的渠道记账, 绝不消耗未选中
        // 候选的配额)。全部被拒 → RateLimited (429); 与 NoCandidate (503) 区分。
        // limits 空时直接走 pick, 与历史行为一致 (零开销)。
        let mut rng = rand::rngs::StdRng::from_entropy();
        let unit = if self.limits.is_empty() {
            self.selector
                .pick(&cands, &*self.health, exclude, (self.now_ms)(), &mut rng)
                .ok_or_else(|| no_candidate(group, public_model))?
        } else {
            let now = (self.now_ms)();
            // 局部排除集 = 调用方已试 + 本次限流拒绝的 key。每轮要么放行,
            // 要么把被拒 key 加入排除集 — 排除集单调增长, pick 必在某轮
            // 因候选耗尽返回 None, 循环终止。
            let mut refused: Vec<String> = Vec::with_capacity(cands.len() + exclude.len());
            refused.extend_from_slice(exclude);
            let mut refused_by_limit = false;
            loop {
                let picked =
                    match self
                        .selector
                        .pick(&cands, &*self.health, &refused, now, &mut rng)
                    {
                        Some(p) => p,
                        None => {
                            // 候选耗尽: 若本轮限流拒过至少一个 → 全被限流 (429);
                            // 否则是纯粹的不可调度 (503)。
                            return Err(if refused_by_limit {
                                DispatchError::RateLimited {
                                    group: group.to_string(),
                                    model: public_model.to_string(),
                                }
                            } else {
                                no_candidate(group, public_model)
                            });
                        }
                    };
                let admitted = match self.limits.get(&picked.meta.key) {
                    Some(spec) => self.rl.admits(&picked.meta.key, Some(spec), now),
                    None => true, // 未配置限流的单元恒放行
                };
                if admitted {
                    break picked;
                }
                refused_by_limit = true;
                refused.push(picked.meta.key.clone());
            }
        };
        let channel = snap
            .channels
            .get(&unit.channel_key)
            .ok_or_else(|| no_candidate(group, public_model))?;
        resolve_candidate(unit, channel).ok_or_else(|| no_candidate(group, public_model))
    }

    fn report(&self, unit_key: &str, outcome: Result<u16, FailureClass>, latency_ms: u32) {
        self.health.record(unit_key, outcome, latency_ms);
    }
}
