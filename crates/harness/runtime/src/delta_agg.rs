//! Stream aggregation for text, reasoning, and OpenAI-style tool-call fragments.

use std::collections::BTreeMap;

use serde_json::Value;
use thiserror::Error;

use harness_tools::{ToolId, ToolInvocation};

use crate::provider::{ProviderDelta, ToolCallFragment};

/// Accumulates streamed model output until a turn finishes.
#[derive(Debug, Default)]
pub struct DeltaAggregator {
    text: String,
    reasoning: String,
    calls: BTreeMap<usize, PartialToolCall>,
}

#[derive(Debug, Default)]
struct PartialToolCall {
    call_id: Option<String>,
    name: Option<String>,
    arguments: String,
}

/// Errors while converting streamed tool-call fragments into invocations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AggregateError {
    #[error("tool call at index {index} is missing an id")]
    MissingCallId { index: usize },
    #[error("tool call `{call_id}` is missing a function name")]
    MissingName { call_id: String },
    #[error("unknown tool alias `{alias}`")]
    UnknownAlias { alias: String },
    #[error("tool call `{call_id}` has malformed arguments: {message}")]
    MalformedArguments { call_id: String, message: String },
}

/// Maps a streamed function name to a Ferrite [`ToolId`].
pub trait ToolAliasResolver {
    fn resolve(&self, alias: &str) -> Option<ToolId>;
}

impl<F> ToolAliasResolver for F
where
    F: Fn(&str) -> Option<ToolId>,
{
    fn resolve(&self, alias: &str) -> Option<ToolId> {
        self(alias)
    }
}

impl DeltaAggregator {
    pub fn apply(&mut self, delta: &ProviderDelta) {
        if let Some(text) = &delta.text {
            self.text.push_str(text);
        }
        if let Some(reasoning) = &delta.reasoning {
            self.reasoning.push_str(reasoning);
        }
        if let Some(fragment) = &delta.tool_call {
            self.apply_fragment(fragment);
        }
    }

    fn apply_fragment(&mut self, fragment: &ToolCallFragment) {
        let slot = self.calls.entry(fragment.index).or_default();
        if let Some(call_id) = &fragment.call_id
            && slot.call_id.is_none()
        {
            slot.call_id = Some(call_id.clone());
        }
        if let Some(name) = &fragment.name
            && slot.name.is_none()
        {
            slot.name = Some(name.clone());
        }
        if let Some(arguments) = &fragment.arguments {
            slot.arguments.push_str(arguments);
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn reasoning(&self) -> Option<&str> {
        if self.reasoning.is_empty() {
            None
        } else {
            Some(self.reasoning.as_str())
        }
    }

    pub fn finish(
        self,
        resolver: &impl ToolAliasResolver,
    ) -> Result<TurnAggregate, AggregateError> {
        let mut tool_calls = Vec::with_capacity(self.calls.len());
        for (index, partial) in self.calls {
            let call_id = partial
                .call_id
                .ok_or(AggregateError::MissingCallId { index })?;
            let alias = partial.name.ok_or_else(|| AggregateError::MissingName {
                call_id: call_id.clone(),
            })?;
            let tool_id = resolver
                .resolve(&alias)
                .ok_or(AggregateError::UnknownAlias { alias })?;
            let arguments = parse_arguments(&call_id, &partial.arguments)?;
            tool_calls.push(ToolInvocation {
                call_id,
                tool_id,
                arguments,
                provider_metadata: Value::Null,
            });
        }

        Ok(TurnAggregate {
            text: self.text,
            reasoning: if self.reasoning.is_empty() {
                None
            } else {
                Some(self.reasoning)
            },
            tool_calls,
        })
    }
}

fn parse_arguments(call_id: &str, raw: &str) -> Result<Value, AggregateError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str(trimmed).map_err(|error| AggregateError::MalformedArguments {
        call_id: call_id.to_string(),
        message: error.to_string(),
    })
}

/// Completed turn payload after aggregation.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnAggregate {
    pub text: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolInvocation>,
}
