pub mod candidate;
pub mod health;
pub mod ratelimit;
pub mod retry;
pub mod selector;
pub mod stage;

pub use candidate::{Candidate, STATUS_ENABLED, resolve_candidate};
pub use health::{FailureClass, HealthState, HealthTable, MemoryHealthTable};
pub use ratelimit::{RateLimitSpec, SlidingWindow};
pub use retry::{Attempt, AttemptOutcome, Failover, RetryLoop, RetryPolicy, run_retry_loop};
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
        if cands.is_empty() {
            return Err(no_candidate(group, public_model));
        }
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
}
