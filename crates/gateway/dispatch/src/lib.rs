pub mod candidate;
pub mod failure_scope;
pub mod health;
pub mod ratelimit;
pub mod retry;
pub mod retry_policy;
pub mod selector;
pub mod stage;

pub use candidate::{Candidate, STATUS_ENABLED, resolve_candidate};
pub use failure_scope::{FailureScope, classify_channel_scope};
pub use health::{FailureClass, HealthState, HealthTable, MemoryHealthTable};
pub use ratelimit::{RateLimitSpec, SlidingWindow};
pub use retry::{Attempt, AttemptOutcome, Failover, RetryLoop, RetryPolicy, run_retry_loop};
pub use retry_policy::ModelRetryPolicies;
pub use selector::{Selector, WeightedSelector};
pub use stage::DispatchStage;

use arc_swap::ArcSwap;
use contract::records::{ChannelRecord, RouteUnitRecord};
use rand::SeedableRng;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub units: Vec<RouteUnitRecord>,
    pub channels: HashMap<String, ChannelRecord>,
}

pub trait Dispatch: Send + Sync {
    fn select(
        &self,
        group: &str,
        public_model: &str,
        exclude: &[String],
    ) -> Result<Candidate, DispatchError>;
    fn report(&self, unit_key: &str, outcome: Result<u16, FailureClass>);

    /// 查渠道展示名（快照 channels map，key = 渠道 UUID）。
    ///
    /// DispatchStage 选中路由后用它给 ctx 补渠道归因名；快照未就绪或渠道
    /// 已被移除时返回 None（调用方降级为只带 key）。
    /// 默认 None：`Candidate`/`SelectedRoute` 不携带渠道名（见 ctx.rs），
    /// 名字只能由持快照的实现者回查；非 `Dispatcher` 实现者不承担该职责。
    fn channel_name(&self, channel_key: &str) -> Option<String> {
        let _ = channel_key;
        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("no candidate for {group}/{model}")]
    NoCandidate { group: String, model: String },
    #[error("all candidates rate-limited for {group}/{model}")]
    RateLimited { group: String, model: String },
    #[error("catalog snapshot not ready")]
    SnapshotNotReady,
    #[error("retries exhausted for {group}/{model}")]
    RetriesExhausted { group: String, model: String },
}

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

/// 兜底候选：组内**没有**该模型的路由单元时，回落到 settings 标了
/// `fallback: true` 的启用渠道（api-hub `[aliases] default` 的 PG 版）。
///
/// 模型名原样透传（不 rename）——兜底渠道的语义就是「什么都接下来自己分发」。
/// key_index 固定 0：多 key 渠道兜底走哪把 key 由运维在 settings 里显式控制
/// 超出本次范围（TODO(#158)）。未标 fallback 的渠道不参与，保持旧行为。
fn fallback_units(snap: &Snapshot, group: &str, public_model: &str) -> Vec<RouteUnitRecord> {
    snap.channels
        .values()
        .filter(|c| {
            c.status == STATUS_ENABLED
                && !c.keys.is_empty()
                && c.groups.iter().any(|g| g == group)
                && c.settings.get("fallback").and_then(|v| v.as_bool()) == Some(true)
        })
        .map(|c| RouteUnitRecord {
            meta: contract::records::SyncMeta {
                key: format!("{}#fallback", c.meta.key),
                schema_version: c.meta.schema_version,
                logical_version: c.meta.logical_version,
                origin: c.meta.origin.clone(),
                updated_at: c.meta.updated_at,
            },
            group: group.to_string(),
            public_model: public_model.to_string(),
            channel_key: c.meta.key.clone(),
            key_index: 0,
            upstream_model: public_model.to_string(),
            priority: 0,
            weight: 1,
            status: STATUS_ENABLED,
        })
        .collect()
}

pub struct Dispatcher {
    snapshot: ArcSwap<Option<Arc<Snapshot>>>,
    health: Arc<MemoryHealthTable>,
    selector: WeightedSelector,
    now_ms: Box<dyn Fn() -> u64 + Send + Sync>,
    limits: ArcSwap<HashMap<String, RateLimitSpec>>,
    rl: Arc<SlidingWindow>,
}

impl Dispatcher {
    pub fn new(snapshot: Option<Arc<Snapshot>>, health: Arc<MemoryHealthTable>) -> Self {
        Self::with_limits(
            snapshot,
            health,
            Arc::new(HashMap::new()),
            Arc::new(SlidingWindow::new()),
        )
    }

    pub fn with_limits(
        snapshot: Option<Arc<Snapshot>>,
        health: Arc<MemoryHealthTable>,
        limits: Arc<HashMap<String, RateLimitSpec>>,
        rl: Arc<SlidingWindow>,
    ) -> Self {
        Self::with_limits_and_clock(snapshot, health, limits, rl, || {
            chrono::Utc::now().timestamp_millis().max(0) as u64
        })
    }

    pub fn with_limits_and_clock(
        snapshot: Option<Arc<Snapshot>>,
        health: Arc<MemoryHealthTable>,
        limits: Arc<HashMap<String, RateLimitSpec>>,
        rl: Arc<SlidingWindow>,
        now_ms: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            snapshot: ArcSwap::new(Arc::new(snapshot)),
            health,
            selector: WeightedSelector,
            now_ms: Box::new(now_ms),
            limits: ArcSwap::new(limits),
            rl,
        }
    }

    pub fn set_snapshot(&self, snapshot: Arc<Snapshot>) {
        self.snapshot.store(Arc::new(Some(snapshot)));
    }

    /// 取当前渠道快照的只读口 — 返回共享的 `Arc` 句柄（零拷贝，非克隆数据）。
    ///
    /// 语义：boot 前未装载过任何快照（[`DispatchError::SnapshotNotReady`] 场景）
    /// 时为 `None`；装载后即使被 reload 覆盖，旧句柄仍保持有效（ArcSwap 原子替换）。
    /// 面向 admin 查询面（如 `/api/gateway/health` 的渠道归因 join），
    /// 不用于热路径选择（热路径走 [`Dispatch::select`]）。
    pub fn snapshot(&self) -> Option<Arc<Snapshot>> {
        Arc::clone(&self.snapshot.load_full()).as_ref().clone()
    }

    pub fn set_limits(&self, limits: HashMap<String, RateLimitSpec>) {
        self.limits.store(Arc::new(limits));
    }
}

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
        let loaded = self.snapshot.load();
        let snap = match loaded.as_ref() {
            Some(s) => s,
            None => return Err(DispatchError::SnapshotNotReady),
        };
        let cands = candidates_from_snapshot(&snap.units, group, public_model);
        // 无该模型的路由单元 → 兜底渠道（settings.fallback，见 fallback_units）。
        // 有单元但全被禁用不兜底：那是运维主动下线，静默改道会掩盖配置错误。
        let fallback_owned: Vec<RouteUnitRecord>;
        let cands: Vec<&RouteUnitRecord> = if cands.is_empty() {
            fallback_owned = fallback_units(snap, group, public_model);
            if fallback_owned.is_empty() {
                return Err(no_candidate(group, public_model));
            }
            fallback_owned.iter().collect()
        } else {
            cands
        };
        let cands: Vec<&RouteUnitRecord> = cands
            .into_iter()
            .filter(|u| {
                snap.channels
                    .get(&u.channel_key)
                    .is_some_and(|c| c.status == STATUS_ENABLED)
            })
            .collect();
        let limits = self.limits.load();
        let mut rng = rand::rngs::StdRng::from_entropy();
        let unit = if limits.is_empty() {
            self.selector
                .pick(&cands, &*self.health, exclude, (self.now_ms)(), &mut rng)
                .ok_or_else(|| no_candidate(group, public_model))?
        } else {
            let now = (self.now_ms)();
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
                let admitted = match limits.get(&picked.meta.key) {
                    Some(spec) => self.rl.admits(&picked.meta.key, Some(spec), now),
                    None => true,
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
            .ok_or(DispatchError::NoCandidate {
                group: group.to_string(),
                model: public_model.to_string(),
            })?;
        resolve_candidate(unit, channel).ok_or(DispatchError::NoCandidate {
            group: group.to_string(),
            model: public_model.to_string(),
        })
    }

    fn report(&self, unit_key: &str, outcome: Result<u16, FailureClass>) {
        self.health.record(unit_key, outcome);
    }

    fn channel_name(&self, channel_key: &str) -> Option<String> {
        self.snapshot
            .load()
            .as_ref()
            .as_ref()
            .and_then(|s| s.channels.get(channel_key))
            .map(|c| c.name.clone())
    }
}
