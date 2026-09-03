//! `quota` —— gate 3：账户余额 vs 预估成本（pre-consume 语义）

use async_trait::async_trait;
use std::sync::Arc;
use arc_swap::ArcSwap;
use gateway_pipeline::{PricingSnapshot, QuotaSnapshot};
use super::chain::{Gate, GateCtx};
use super::error::Rejection;

pub struct QuotaGate {
    quotas: Arc<ArcSwap<QuotaSnapshot>>,
    pricing: Arc<ArcSwap<PricingSnapshot>>,
}

impl QuotaGate {
    pub fn new(quotas: Arc<ArcSwap<QuotaSnapshot>>, pricing: Arc<ArcSwap<PricingSnapshot>>) -> Self {
        Self { quotas, pricing }
    }
}

#[async_trait]
impl Gate for QuotaGate {
    fn name(&self) -> &'static str { "quota" }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        let user = ctx.user.as_ref().ok_or(Rejection::AuthSkipped)?;
        let model = ctx.requested_model.as_deref()
            .ok_or(Rejection::ModelNotSpecified)?;

        // 1. 查余额
        let remaining = self.quotas.load().remaining(user.id);
        if remaining <= 0 {
            return Err(Rejection::InsufficientQuota { remaining, cost: 0 });
        }

        // 2. 估成本
        let max_tokens = ctx.requested_max_tokens.unwrap_or(4096);
        let cost = self.pricing.load()
            .lookup(model, ctx.group.as_deref().unwrap_or("default"))
            .map(|p| estimate_cost(p, max_tokens))
            .unwrap_or(0);

        // 3. 余额 < 成本 → 拒绝
        if remaining < cost {
            return Err(Rejection::InsufficientQuota { remaining, cost });
        }

        ctx.estimated_cost = Some(cost);
        Ok(())
    }
}

fn estimate_cost(price: gateway_pipeline::PriceRow, max_tokens: u32) -> i64 {
    // cost = output_per_m * max_tokens / 1_000_000 * 1_000_000 (单位对齐)
    ((price.output_per_m * max_tokens as f64) / 1_000_000.0) as i64 * 1_000_000
}
