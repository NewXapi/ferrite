//! Root single-agent model → tool → model loop.

use chrono::Utc;
use serde_json::json;
use thiserror::Error;

use harness_core::{
    AgentChatRef, AgentRun, AgentRunEventLevel, AgentRunPresentation, AgentRunStatus,
};
use harness_prompt::{AgentModelContentPart, AgentModelMessage, AgentModelRequest, AgentModelRole};
use harness_tools::{AgentModelTool, InvocationToolSnapshot, ToolChoice, ToolId, ToolTurnContract};

use crate::cancel::CancellationToken;
use crate::delta_agg::ToolAliasResolver;
use crate::event_sink::{EventFactory, EventSink};
use crate::persistence::{PersistenceError, RunPersistence};
use crate::provider::{ChatProvider, ProviderError, ProviderFinishReason, empty_request};
use crate::tool_exec::{ToolExecError, ToolExecutor};
use crate::turn::{TurnDriver, TurnError};

/// Request to start a root agent run.
#[derive(Debug, Clone)]
pub struct AgentRunRequest {
    pub run_id: String,
    pub workspace_id: String,
    pub stable_chat_id: String,
    pub chat_ref: AgentChatRef,
    pub profile_id: Option<String>,
    pub model: String,
    pub prompt: AgentModelRequest,
    pub snapshot: InvocationToolSnapshot,
    pub max_rounds: usize,
}

/// Injected loop dependencies.
pub struct AgentRunDeps<'a, P> {
    pub provider: &'a P,
    pub executor: &'a mut ToolExecutor,
    pub persistence: &'a RunPersistence,
    pub cancel: CancellationToken,
}

/// Loop errors.
#[derive(Debug, Error)]
pub enum LoopError {
    #[error(transparent)]
    Turn(#[from] TurnError),
    #[error(transparent)]
    Tool(#[from] ToolExecError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}

struct BindingResolver<'a> {
    snapshot: &'a InvocationToolSnapshot,
}

impl ToolAliasResolver for BindingResolver<'_> {
    fn resolve(&self, alias: &str) -> Option<ToolId> {
        self.snapshot
            .bindings()
            .iter()
            .find(|binding| binding.model_alias() == alias)
            .map(|binding| binding.tool_id().clone())
    }
}

/// Drive one root agent run to a terminal status.
pub async fn run_agent_run<P: ChatProvider>(
    request: AgentRunRequest,
    deps: AgentRunDeps<'_, P>,
    sink: &mut impl EventSink,
) -> Result<AgentRun, LoopError> {
    let now = Utc::now();
    let mut run = AgentRun {
        id: request.run_id.clone(),
        workspace_id: request.workspace_id,
        stable_chat_id: request.stable_chat_id,
        chat_ref: request.chat_ref,
        generation_type: "chat".to_string(),
        profile_id: request.profile_id,
        skill_scope_refs: Default::default(),
        persist_base_state_id: None,
        input_message_count: Some(request.prompt.messages.len()),
        presentation: AgentRunPresentation::Foreground,
        status: AgentRunStatus::Created,
        created_at: now,
        updated_at: now,
    };
    let mut events = EventFactory::new(&run.id);
    persist_status(
        &mut run,
        AgentRunStatus::Created,
        deps.persistence,
        &mut events,
        sink,
        "run.started",
        AgentRunEventLevel::Info,
        json!({}),
    )
    .await?;

    persist_status(
        &mut run,
        AgentRunStatus::InitializingWorkspace,
        deps.persistence,
        &mut events,
        sink,
        "workspace.initialized",
        AgentRunEventLevel::Info,
        json!({}),
    )
    .await?;

    persist_status(
        &mut run,
        AgentRunStatus::AssemblingContext,
        deps.persistence,
        &mut events,
        sink,
        "context.assembled",
        AgentRunEventLevel::Info,
        json!({ "messages": request.prompt.messages.len() }),
    )
    .await?;

    let mut messages = request.prompt.messages.clone();
    if let Some(system) = &request.prompt.system {
        messages.insert(
            0,
            AgentModelMessage::text(AgentModelRole::System, system.clone()),
        );
    }
    let tools: Vec<AgentModelTool> = request
        .snapshot
        .bindings()
        .iter()
        .map(|binding| AgentModelTool {
            tool_id: binding.tool_id().clone(),
            model_alias: binding.model_alias().to_string(),
            description: binding.descriptor().description.clone(),
            input_schema: binding.descriptor().input_schema.clone(),
        })
        .collect();
    let turn_contract =
        ToolTurnContract::all(&request.snapshot, ToolChoice::Auto).map_err(|error| {
            LoopError::Turn(TurnError::Provider(ProviderError::Failed(
                error.to_string(),
            )))
        })?;

    deps.executor.begin_run();
    let resolver = BindingResolver {
        snapshot: &request.snapshot,
    };
    let driver = TurnDriver::new(deps.provider);
    let mut saw_tool_error = false;
    let max_rounds = request.max_rounds.max(1);

    for round in 1..=max_rounds {
        if deps.cancel.is_cancelled() {
            persist_status(
                &mut run,
                AgentRunStatus::Cancelling,
                deps.persistence,
                &mut events,
                sink,
                "generation.cancelled",
                AgentRunEventLevel::Warn,
                json!({ "reason": deps.cancel.reason() }),
            )
            .await?;
            persist_status(
                &mut run,
                AgentRunStatus::Cancelled,
                deps.persistence,
                &mut events,
                sink,
                "run.completed",
                AgentRunEventLevel::Warn,
                json!({ "status": "cancelled" }),
            )
            .await?;
            return Ok(run);
        }

        persist_status(
            &mut run,
            AgentRunStatus::CallingModel,
            deps.persistence,
            &mut events,
            sink,
            "model.round",
            AgentRunEventLevel::Info,
            json!({ "round": round }),
        )
        .await?;

        let mut provider_request = empty_request(&request.model, messages.clone());
        provider_request.tools = tools.clone();
        provider_request.tool_choice = Some(ToolChoice::Auto);

        let outcome = match driver
            .run(
                provider_request,
                deps.cancel.clone(),
                &resolver,
                &mut events,
                sink,
            )
            .await
        {
            Ok(outcome) => outcome,
            Err(TurnError::Provider(ProviderError::Cancelled)) => {
                persist_status(
                    &mut run,
                    AgentRunStatus::Cancelled,
                    deps.persistence,
                    &mut events,
                    sink,
                    "run.completed",
                    AgentRunEventLevel::Warn,
                    json!({ "status": "cancelled" }),
                )
                .await?;
                return Ok(run);
            }
            Err(error) => {
                persist_status(
                    &mut run,
                    AgentRunStatus::Failed,
                    deps.persistence,
                    &mut events,
                    sink,
                    "run.failed",
                    AgentRunEventLevel::Error,
                    json!({ "error": error.to_string() }),
                )
                .await?;
                return Err(error.into());
            }
        };

        deps.persistence
            .write_model_response(
                &run.id,
                round as u64,
                &json!({
                    "text": outcome.text,
                    "reasoning": outcome.reasoning,
                    "toolCalls": outcome.tool_calls,
                }),
            )
            .await?;
        let model_event = events.next(
            "model.delta",
            AgentRunEventLevel::Debug,
            json!({ "text": outcome.text, "reasoning": outcome.reasoning }),
        );
        deps.persistence.append_event(&run.id, &model_event).await?;
        sink.emit(model_event);

        if outcome.finish_reason == ProviderFinishReason::Error {
            persist_status(
                &mut run,
                AgentRunStatus::Failed,
                deps.persistence,
                &mut events,
                sink,
                "run.failed",
                AgentRunEventLevel::Error,
                json!({ "error": "provider reported error finish reason" }),
            )
            .await?;
            return Err(LoopError::Turn(TurnError::Provider(ProviderError::Failed(
                "provider reported error finish reason".to_string(),
            ))));
        }

        if outcome.finish_reason == ProviderFinishReason::Cancelled || deps.cancel.is_cancelled() {
            persist_status(
                &mut run,
                AgentRunStatus::Cancelled,
                deps.persistence,
                &mut events,
                sink,
                "run.completed",
                AgentRunEventLevel::Warn,
                json!({ "status": "cancelled" }),
            )
            .await?;
            return Ok(run);
        }

        if outcome.tool_calls.is_empty() {
            persist_status(
                &mut run,
                AgentRunStatus::Finishing,
                deps.persistence,
                &mut events,
                sink,
                "run.finishing",
                AgentRunEventLevel::Info,
                json!({}),
            )
            .await?;
            let status = if saw_tool_error {
                AgentRunStatus::PartialSuccess
            } else {
                AgentRunStatus::Completed
            };
            persist_status(
                &mut run,
                status,
                deps.persistence,
                &mut events,
                sink,
                "run.completed",
                AgentRunEventLevel::Info,
                json!({ "status": format!("{status:?}").to_ascii_lowercase() }),
            )
            .await?;
            deps.persistence
                .write_checkpoint(&run.id, round as u64, &run)
                .await?;
            return Ok(run);
        }

        persist_status(
            &mut run,
            AgentRunStatus::DispatchingTool,
            deps.persistence,
            &mut events,
            sink,
            "tool.round",
            AgentRunEventLevel::Info,
            json!({ "count": outcome.tool_calls.len() }),
        )
        .await?;

        let mut assistant_parts = Vec::new();
        if !outcome.text.is_empty() {
            assistant_parts.push(AgentModelContentPart::Text {
                text: outcome.text.clone(),
            });
        }
        for call in &outcome.tool_calls {
            let alias = request
                .snapshot
                .binding(&call.tool_id)
                .map(|binding| binding.model_alias().to_string())
                .unwrap_or_else(|| call.tool_id.to_string());
            assistant_parts.push(AgentModelContentPart::ToolCall {
                call_id: call.call_id.clone(),
                tool_id: call.tool_id.to_string(),
                model_alias: alias,
                arguments: call.arguments.clone(),
            });
        }
        messages.push(AgentModelMessage {
            role: AgentModelRole::Assistant,
            parts: assistant_parts,
            name: None,
        });

        for call in outcome.tool_calls {
            sink.emit(events.next(
                "tool.started",
                AgentRunEventLevel::Info,
                json!({ "callId": call.call_id, "toolId": call.tool_id.to_string() }),
            ));
            deps.persistence
                .write_tool_args(&run.id, &call.call_id, &call.arguments)
                .await?;
            let result = match deps
                .executor
                .execute(&request.snapshot, &turn_contract, call)
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    persist_status(
                        &mut run,
                        AgentRunStatus::Failed,
                        deps.persistence,
                        &mut events,
                        sink,
                        "run.failed",
                        AgentRunEventLevel::Error,
                        json!({ "error": error.to_string() }),
                    )
                    .await?;
                    return Err(error.into());
                }
            };
            if result.is_error {
                saw_tool_error = true;
            }
            sink.emit(events.next(
                "tool.completed",
                if result.is_error {
                    AgentRunEventLevel::Warn
                } else {
                    AgentRunEventLevel::Info
                },
                json!({
                    "callId": result.call_id,
                    "isError": result.is_error,
                }),
            ));
            deps.persistence.write_tool_result(&run.id, &result).await?;
            messages.push(AgentModelMessage {
                role: AgentModelRole::Tool,
                parts: vec![AgentModelContentPart::ToolResult {
                    call_id: result.call_id,
                    tool_id: result.tool_id.to_string(),
                    content: result.content,
                    is_error: result.is_error,
                }],
                name: None,
            });
        }
    }

    persist_status(
        &mut run,
        AgentRunStatus::Failed,
        deps.persistence,
        &mut events,
        sink,
        "run.failed",
        AgentRunEventLevel::Error,
        json!({ "error": "max rounds exceeded" }),
    )
    .await?;
    Ok(run)
}

async fn persist_status(
    run: &mut AgentRun,
    status: AgentRunStatus,
    persistence: &RunPersistence,
    events: &mut EventFactory,
    sink: &mut impl EventSink,
    event_type: &str,
    level: AgentRunEventLevel,
    payload: serde_json::Value,
) -> Result<(), PersistenceError> {
    run.status = status;
    run.updated_at = Utc::now();
    persistence.write_run(run).await?;
    let event = events.next(event_type, level, payload);
    persistence.append_event(&run.id, &event).await?;
    sink.emit(event);
    Ok(())
}
