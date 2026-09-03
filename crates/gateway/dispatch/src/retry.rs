//! failover 重试循环 — 一次请求生命内的状态机编排。
//!
//! 参考: one-api controller/relay.go (重试忽略首优先级),
//! new-api transport/handler/handle_relay.go (重试前 reset body / 退款 / 健康回报)。
//!
//! 循环:
//! ```text
//! attempt = 0
//! loop:
//!   candidate = dispatch.select(group, model, exclude=tried)
//!   forward(candidate)                    # metering: 预扣在第一次尝试前完成
//!   on success: dispatch.report(ok); metering.settle(); return
//!   on FailureClass::Fatal: report; metering.settle(as_error); return 4xx
//!   on FailureClass::Retryable: report; tried += candidate; 
//!        if attempts >= MAX: return 502; continue
//! ```
//!
//! 关键不变量:
//! - 预扣只发生一次 (不随重试叠加), 结算一次;
//! - 重试前必须能**重放请求体** (forward 的 replayable body 保证);
//! - 每次尝试的健康回报不可省 (跨候选统计)。

use crate::candidate::Candidate;
use crate::health::FailureClass;

/// 单次尝试结果 — forward → retry 的回报。
pub enum AttemptOutcome {
    /// 上游 2xx/4xx-with-usage (billing 正常)。
    Done { status: u16 },
    /// 可重试失败。
    Retryable(FailureClass),
    /// 不可重试失败。
    Fatal(FailureClass),
}

/// 重试策略参数 (TODO(#311) 配置化)。
pub struct RetryPolicy {
    /// 含首次在内最多尝试次数 (new-api retry 次数语义)。
    pub max_attempts: u32,
    /// 重试是否忽略首优先级层 (one-api ignoreFirstPriority 语义)。
    pub ignore_first_priority_on_retry: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 3, ignore_first_priority_on_retry: true }
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
