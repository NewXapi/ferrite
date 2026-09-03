//! `ratelimit` —— gate 4：速率限流（per-key / per-channel RPM/TPM 表达式）

use async_trait::async_trait;
use std::sync::Arc;
use gateway_pipeline::{RateLimiter, LimitScope};
use super::chain::{Gate, GateCtx};
use super::error::Rejection;

pub struct RateLimitGate {
    limiter: Arc<RateLimiter>,
}

impl RateLimitGate {
    pub fn new(limiter: Arc<RateLimiter>) -> Self {
        Self { limiter }
    }
}

#[async_trait]
impl Gate for RateLimitGate {
    fn name(&self) -> &'static str { "ratelimit" }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let token = ctx.token.as_ref().ok_or(Rejection::AuthSkipped)?;

        // per-key 限流
        if !self.limiter.try_acquire(LimitScope::PerKey, token.id) {
            return Err(Rejection::RateLimited);
        }
        Ok(())
    }
}
