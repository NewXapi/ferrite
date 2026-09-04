//! `quota` —— gate 3：账户余额 vs 预估成本（pre-consume 语义）

use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;

use super::chain::{Gate, GateCtx};
use super::error::Rejection;
use super::model::parse_body_model_into;
use super::snapshot::PriceRow;
use super::snapshot::{PricingSnapshot, QuotaSnapshot};

pub struct QuotaGate {
    quotas: Arc<ArcSwap<QuotaSnapshot>>,
    pricing: Arc<ArcSwap<PricingSnapshot>>,
}

impl QuotaGate {
    pub fn new(
        quotas: Arc<ArcSwap<QuotaSnapshot>>,
        pricing: Arc<ArcSwap<PricingSnapshot>>,
    ) -> Self {
        Self { quotas, pricing }
    }
}

#[async_trait]
impl Gate for QuotaGate {
    fn name(&self) -> &'static str {
        "quota"
    }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        // 0. 解析请求体（如果 ModelGate 还没跑过）
        if ctx.requested_model.is_none() {
            let body = ctx.request_meta.body.clone();
            parse_body_model_into(&body, ctx)?;
        }

        let token = ctx.token.as_ref().ok_or(Rejection::AuthSkipped)?;
        let model = ctx
            .requested_model
            .as_deref()
            .ok_or(Rejection::ModelNotSpecified)?;

        // 1. 查余额（按 token_id；key 来自 TokenInfo.id）
        let remaining = self.quotas.load().remaining(&token.id.to_string());
        if remaining <= 0 {
            return Err(Rejection::InsufficientQuota { remaining, cost: 0 });
        }

        // 2. 估成本
        let max_tokens = ctx.requested_max_tokens.unwrap_or(4096);
        let group = ctx.group.as_deref().unwrap_or("default");
        let cost = self
            .pricing
            .load()
            .lookup(model, group)
            .map(|p| estimate_cost(&p, max_tokens))
            .unwrap_or(0);

        // 3. 余额 < 成本 → 拒绝
        if remaining < cost {
            return Err(Rejection::InsufficientQuota { remaining, cost });
        }

        ctx.estimated_cost = Some(cost);
        Ok(())
    }
}

/// 预估成本（按输出 token 上限）。输入与 cache 命中 token 暂不计，避免重复计算。
///
/// ponytail: 只估 output_per_m × max_tokens；input/cache 真实消耗由 metering 层
/// 完成后写回余额。
pub fn estimate_cost(price: &PriceRow, max_tokens: u32) -> i64 {
    ((price.output_per_m * max_tokens as f64) / 1_000_000.0) as i64 * 1_000_000
}
