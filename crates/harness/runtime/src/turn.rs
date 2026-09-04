//! Single-turn streaming driver.

use futures_util::StreamExt;
use serde_json::json;
use thiserror::Error;

use harness_core::AgentRunEventLevel;
use harness_tools::ToolInvocation;

use crate::cancel::{CancelReason, CancellationToken};
use crate::delta_agg::{AggregateError, DeltaAggregator, ToolAliasResolver};
use crate::event_sink::{EventFactory, EventSink};
use crate::provider::{
    ChatProvider, ProviderError, ProviderFinishReason, ProviderRequest, ProviderUsage,
};

/// Outcome of one model turn.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnOutcome {
    pub text: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolInvocation>,
    pub usage: Option<ProviderUsage>,
    pub finish_reason: ProviderFinishReason,
}

/// Turn driver errors.
#[derive(Debug, Error)]
pub enum TurnError {
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Aggregate(#[from] AggregateError),
}

/// Drives one streaming model turn.
pub struct TurnDriver<'a, P> {
    provider: &'a P,
}

impl<'a, P: ChatProvider> TurnDriver<'a, P> {
    pub fn new(provider: &'a P) -> Self {
        Self { provider }
    }

    pub async fn run(
        &self,
        request: ProviderRequest,
        cancel: CancellationToken,
        resolver: &impl ToolAliasResolver,
        events: &mut EventFactory,
        sink: &mut impl EventSink,
    ) -> Result<TurnOutcome, TurnError> {
        sink.emit(events.next(
            "model.started",
            AgentRunEventLevel::Info,
            json!({ "model": request.model }),
        ));

        let mut stream = self.provider.stream(request, cancel.clone());
        let mut aggregator = DeltaAggregator::default();
        let mut usage = None;
        let mut finish_reason = None;

        loop {
            let item = tokio::select! {
                _ = cancel.cancelled() => {
                    finish_reason = Some(ProviderFinishReason::Cancelled);
                    break;
                }
                item = stream.next() => item,
            };
            let Some(item) = item else {
                break;
            };
            let delta = item?;
            aggregator.apply(&delta);
            if let Some(text) = &delta.text {
                sink.emit(events.next(
                    "model.delta",
                    AgentRunEventLevel::Debug,
                    json!({ "text": text }),
                ));
            }
            if let Some(reported) = delta.usage {
                usage = Some(reported);
            }
            if let Some(reason) = delta.finish_reason {
                finish_reason = Some(reason);
            }
        }

        if cancel.is_cancelled() {
            sink.emit(events.next(
                "generation.cancelled",
                AgentRunEventLevel::Warn,
                json!({ "reason": cancel.reason() }),
            ));
            if finish_reason.is_none() {
                finish_reason = Some(ProviderFinishReason::Cancelled);
            }
        }

        let aggregate = aggregator.finish(resolver)?;
        Ok(TurnOutcome {
            text: aggregate.text,
            reasoning: aggregate.reasoning,
            tool_calls: aggregate.tool_calls,
            usage,
            finish_reason: finish_reason.unwrap_or(ProviderFinishReason::Stop),
        })
    }
}

/// Maps a user abort into a [`CancelReason`].
pub fn user_cancel(cancel: &CancellationToken) {
    cancel.cancel(CancelReason::UserRequested);
}
