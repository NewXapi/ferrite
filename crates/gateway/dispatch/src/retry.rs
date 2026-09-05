//! failover 重试编排 — 一次请求生命内的状态机数据。
//!
//! 参考: one-api controller/relay.go (重试忽略首优先级),
//! new-api transport/handler/handle_relay.go (重试前 reset body / 退款 / 健康回报),
//! sub2api backend/internal/handler/failover_loop.go (FailoverState:
//! FailedAccountIDs 排除集 + MaxSwitches 预算)。
//!
//! 上层循环形状:
//! ```text
//! failover = Failover::new(policy)
//! while let Some(attempt_no) = failover.next_attempt() {
//!   candidate = dispatch.select(group, model, failover.exclude())
//!   forward(candidate)                    # metering: 预扣在第一次尝试前完成
//!   on success: dispatch.report(ok); metering.settle(); return
//!   on FailureClass::Fatal: report; metering.settle(as_error); return 4xx
//!   on FailureClass::Retryable: report; failover.mark_tried(&candidate); continue
//! }
//! return 502   # 预算耗尽
//! ```
//!
//! 关键不变量:
//! - 预扣只发生一次 (不随重试叠加), 结算一次;
//! - 重试前必须能**重放请求体** (forward 的 replayable body 保证);
//! - 每次尝试的健康回报不可省 (跨候选统计);
//! - 排除集按单元 key (unit.meta.key) 记账, 同 key 不重选 (sub2api FailedAccountIDs)。

use crate::candidate::Candidate;
use crate::health::FailureClass;

/// 单次尝试结果 — forward → retry 的回报。
#[derive(Debug)]
pub enum AttemptOutcome {
    /// 上游 2xx/4xx-with-usage (billing 正常)。
    Done { status: u16 },
    /// 可重试失败。
    Retryable(FailureClass),
    /// 不可重试失败。
    Fatal(FailureClass),
}

/// 重试策略参数 (TODO(#311) 配置化)。
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// 含首次在内最多尝试次数 (new-api retry 次数语义; wildtoken 默认 1)。
    pub max_attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 3 }
    }
}

/// 编排 trait — apps/gateway 的转发 handler 实现 (需要访问 forward+metering)。
pub trait RetryLoop: Send + Sync {
    fn run(
        &self,
        group: &str,
        model: &str,
        policy: &RetryPolicy,
    ) -> impl Future<Output = Result<AttemptOutcome, crate::DispatchError>> + Send;
}

/// 一个尝试的上下文 (retry 传给 forward 的最小信息)。
pub struct Attempt {
    pub candidate: Candidate,
    pub attempt_no: u32,
}

/// 故障转移状态机 — 已试排除集 + 尝试预算 (sub2api FailoverState 的等价物)。
///
/// 本机只保证"不重选已试、不超过预算"; 同渠道退避等时序由上层循环控制
/// (sub2api sameAccountRetryDelay/指数退避)。
#[derive(Debug, Default, Clone)]
pub struct Failover {
    policy: RetryPolicy,
    // ponytail: Vec 而非 HashSet — tried 长度有上界 (max_attempts-1, 默认 2),
    // 线性扫描与哈希在 n<=2 时同量级, Vec 保序且无分配抖动 (ocr #12 驳回)。
    /// 已试候选 key (unit.meta.key), 传给 Dispatch::select 做 exclude。
    tried: Vec<String>,
    /// 已完成尝试计数。
    attempts: u32,
}

impl Failover {
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            policy,
            tried: Vec::new(),
            attempts: 0,
        }
    }

    /// 当前排除集 (外借给 Dispatch::select)。
    pub fn exclude(&self) -> &[String] {
        &self.tried
    }

    /// 推进到下一次尝试; 预算已耗尽 → None (上层返回 502)。
    pub fn next_attempt(&mut self) -> Option<u32> {
        if self.attempts >= self.policy.max_attempts {
            return None;
        }
        self.attempts += 1;
        Some(self.attempts)
    }

    /// 记一笔已试 (失败后调用, 排除集去重)。接受 owned 避免重复分配
    /// (ocr #11: 调用方常已持有 String)。
    pub fn mark_tried(&mut self, key: impl Into<String>) {
        let key = key.into();
        if !self.tried.contains(&key) {
            self.tried.push(key);
        }
    }

    /// 预算是否耗尽。
    pub fn is_exhausted(&self) -> bool {
        self.attempts >= self.policy.max_attempts
    }
}

/// 完整重试循环 — dispatch 拥有的编排逻辑。
///
/// 用户要求: 重试机制放 dispatch, 不散在 forward。本函数把"选候选 →
/// 尝试 → 健康回报 → 排除已试 → 再选"的循环写死在 dispatch::retry,
/// forward 只提供单次 attempt 闭包 (纯 IO, 不含重试判断)。
///
/// 循环形状 (sub2api FailoverState / new-api relay retry 语义):
/// ```text
/// while let Some(attempt_no) = failover.next_attempt() {
///   candidate = select(group, model, failover.exclude())?   // 排除已试
///   outcome   = attempt(candidate).await                     // 一次转发
///   report(candidate, outcome)                               // 健康回报
///   match outcome {
///     Done { .. }      => return Ok           // 成功 / 4xx 已处理
///     Retryable(_)     => mark_tried; continue  // 换候选重试
///     Fatal(_)         => return Ok           // 客户端问题, 不换渠道
///   }
/// }
/// Err(RetriesExhausted)
/// ```
///
/// `RetryPolicy::max_attempts` 预算耗尽 → `DispatchError::RetriesExhausted`
/// (上层映射 502/503, 与 NoCandidate=503 区分)。
pub async fn run_retry_loop<Sel, Attempt, Fut>(
    group: &str,
    model: &str,
    policy: &RetryPolicy,
    mut select: Sel,
    mut attempt: Attempt,
    mut report: impl FnMut(&str, Result<u16, FailureClass>),
) -> Result<AttemptOutcome, crate::DispatchError>
where
    Sel: FnMut(&str, &str, &[String]) -> Result<Candidate, crate::DispatchError>,
    Attempt: FnMut(&Candidate) -> Fut,
    Fut: std::future::Future<Output = AttemptOutcome>,
{
    let mut failover = Failover::new(*policy);
    while failover.next_attempt().is_some() {
        let candidate = select(group, model, failover.exclude())?;
        let outcome = attempt(&candidate).await;
        report(
            &candidate.unit.meta.key,
            match &outcome {
                AttemptOutcome::Done { status } => Ok(*status),
                AttemptOutcome::Retryable(_) | AttemptOutcome::Fatal(_) => Err(match &outcome {
                    AttemptOutcome::Retryable(c) | AttemptOutcome::Fatal(c) => *c,
                    _ => unreachable!(),
                }),
            },
        );
        match outcome {
            AttemptOutcome::Done { .. } | AttemptOutcome::Fatal(_) => return Ok(outcome),
            AttemptOutcome::Retryable(_) => {
                failover.mark_tried(candidate.unit.meta.key.clone());
            }
        }
    }
    Err(crate::DispatchError::RetriesExhausted {
        group: group.to_string(),
        model: model.to_string(),
    })
}

/// 来自配置的重试策略实现。
#[derive(Debug, Clone)]
pub struct ConfigRetryLoop {
    max_attempts: u32,
    retryable_status_codes: Vec<u16>,
}

impl ConfigRetryLoop {
    pub fn new(max_attempts: u32, retryable_status_codes: Vec<u16>) -> Self {
        Self { max_attempts, retryable_status_codes }
    }
    pub fn max_attempts(&self) -> u32 { self.max_attempts }
    pub fn is_retryable(&self, status: u16) -> bool {
        if self.retryable_status_codes.is_empty() {
            matches!(status, 429 | 500..=599)
        } else {
            self.retryable_status_codes.contains(&status)
        }
    }
}
