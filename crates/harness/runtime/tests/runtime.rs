//! Runtime integration tests.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::stream;
use serde_json::json;

use harness_core::GenerationType;
use harness_core::{AgentChatRef, AgentModelRetryPolicy, AgentRunStatus};
use harness_prompt::{AgentModelMessage, AgentModelRole};
use harness_runtime::{
    AgentRunDeps, AgentRunRequest, CancelReason, CancellationToken, ChatProvider, DeltaAggregator,
    EventFactory, EventSink, MpscEventSink, PersistenceError, ProviderDelta, ProviderFinishReason,
    ProviderRequest, ProviderStream, RunPersistence, ToolCallFragment, ToolExecutor, ToolHandler,
    VecEventSink, load_resumable_run, run_agent_run,
};
use harness_tools::{
    InvocationToolSnapshot, ToolBinding, ToolChoice, ToolDescriptor, ToolId, ToolSnapshotId,
};
struct ScriptedProvider {
    rounds: Mutex<Vec<Vec<Result<ProviderDelta, harness_runtime::ProviderError>>>>,
    seen: Mutex<Vec<ProviderRequest>>,
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

impl ScriptedProvider {
    fn new(rounds: Vec<Vec<Result<ProviderDelta, harness_runtime::ProviderError>>>) -> Self {
        Self {
            rounds: Mutex::new(rounds),
            seen: Mutex::new(Vec::new()),
            calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Provider invocations so far; exhausted-script tests assert this instead of
    /// relying on an implicit empty stream.
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// 收到的 provider 请求副本，供生成类型 gating 断言使用。
    fn seen(&self) -> Vec<ProviderRequest> {
        self.seen.lock().expect("seen mutex").clone()
    }
}

impl ChatProvider for ScriptedProvider {
    fn stream(&self, request: ProviderRequest, _cancel: CancellationToken) -> ProviderStream {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().expect("seen mutex").push(request);
        let mut rounds = self.rounds.lock().expect("mock provider mutex");
        // Exhausted script yields an empty stream instead of panicking, so tests
        // that abort mid-retry do not depend on an exact round count.
        let deltas = if rounds.is_empty() {
            Vec::new()
        } else {
            rounds.remove(0)
        };
        drop(rounds);
        Box::pin(stream::iter(deltas))
    }
}

fn descriptor(id: ToolId) -> ToolDescriptor {
    ToolDescriptor {
        id,
        title: None,
        description: None,
        input_schema: json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        }),
        output_schema: None,
        annotations: json!({}),
    }
}

fn snapshot() -> InvocationToolSnapshot {
    let id = ToolId::builtin("read_file").expect("tool id");
    InvocationToolSnapshot::try_new(
        ToolSnapshotId::parse("inv_root").expect("snapshot"),
        vec![ToolBinding::new(descriptor(id), "read_file", Some(4)).unwrap()],
        8,
    )
    .expect("snapshot")
}

fn request(run_id: &str, persistence_ok: bool) -> AgentRunRequest {
    let _ = persistence_ok;
    AgentRunRequest {
        run_id: run_id.to_string(),
        workspace_id: "ws".into(),
        stable_chat_id: "chat".into(),
        chat_ref: AgentChatRef::Character {
            character_id: "alice".into(),
            file_name: "alice".into(),
        },
        profile_id: None,
        generation_type: GenerationType::Chat,
        model: "test-model".into(),
        prompt: harness_prompt::AgentModelRequest {
            system: Some("sys".into()),
            messages: vec![AgentModelMessage::text(AgentModelRole::User, "hi")],
            tools: Vec::new(),
            metadata: None,
        },
        snapshot: snapshot(),
        max_rounds: 4,
        retry: AgentModelRetryPolicy {
            max_retries: 0,
            interval_ms: 0,
        },
    }
}

fn text_delta(text: &str, finish: Option<ProviderFinishReason>) -> ProviderDelta {
    ProviderDelta {
        text: Some(text.into()),
        finish_reason: finish,
        ..ProviderDelta::default()
    }
}

fn tool_delta(call_id: &str, name: &str, arguments: &str) -> Vec<ProviderDelta> {
    vec![
        ProviderDelta {
            tool_call: Some(ToolCallFragment {
                index: 0,
                call_id: Some(call_id.into()),
                name: Some(name.into()),
                arguments: Some(arguments.into()),
            }),
            ..ProviderDelta::default()
        },
        ProviderDelta {
            finish_reason: Some(ProviderFinishReason::ToolCalls),
            ..ProviderDelta::default()
        },
    ]
}

fn ok_handler() -> ToolHandler {
    Arc::new(|invocation| {
        Box::pin(async move {
            harness_tools::AgentToolResult {
                call_id: invocation.call_id,
                tool_id: invocation.tool_id,
                content: "file contents".into(),
                structured: json!({}),
                is_error: false,
                error_code: None,
                resource_refs: Vec::new(),
            }
        })
    })
}

fn err_handler() -> ToolHandler {
    Arc::new(|invocation| {
        Box::pin(async move {
            harness_tools::AgentToolResult {
                call_id: invocation.call_id,
                tool_id: invocation.tool_id,
                content: "boom".into(),
                structured: json!({}),
                is_error: true,
                error_code: Some("failed".into()),
                resource_refs: Vec::new(),
            }
        })
    })
}

fn tmp_root(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ferrite-runtime-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[tokio::test]
async fn text_only_run_completes_and_persists() {
    let root = tmp_root("text");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Ok(text_delta(
        "hello",
        Some(ProviderFinishReason::Stop),
    ))]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let run = run_agent_run(
        request("run_text", true),
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");

    assert_eq!(run.status, AgentRunStatus::Completed);
    let loaded = persistence.load_run("run_text").await.expect("load run");
    assert_eq!(loaded.status, AgentRunStatus::Completed);
    let events = persistence.load_events("run_text").await.expect("events");
    assert!(events.iter().any(|event| event.event_type == "model.delta"));
    assert!(
        events
            .iter()
            .any(|event| event.event_type == "run.completed")
    );
}

#[tokio::test]
async fn two_turn_tool_loop_injects_result() {
    let root = tmp_root("tools");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![
        tool_delta("c1", "read_file", "{\"path\":\"/tmp/x\"}")
            .into_iter()
            .map(Ok)
            .collect(),
        vec![Ok(text_delta("done", Some(ProviderFinishReason::Stop)))],
    ]);
    let mut executor = ToolExecutor::new();
    executor.register(ToolId::builtin("read_file").unwrap(), ok_handler());
    let mut sink = VecEventSink::default();
    let run = run_agent_run(
        request("run_tools", true),
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");
    assert_eq!(run.status, AgentRunStatus::Completed);
    let result = tokio::fs::read_to_string(
        persistence
            .run_dir("run_tools")
            .unwrap()
            .join("tool-results/c1.json"),
    )
    .await
    .expect("result file");
    assert!(result.contains("file contents"));
}

#[tokio::test]
async fn handler_failure_is_partial_success() {
    let root = tmp_root("fail");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![
        tool_delta("c1", "read_file", "{\"path\":\"/tmp/x\"}")
            .into_iter()
            .map(Ok)
            .collect(),
        vec![Ok(text_delta(
            "recovered",
            Some(ProviderFinishReason::Stop),
        ))],
    ]);
    let mut executor = ToolExecutor::new();
    executor.register(ToolId::builtin("read_file").unwrap(), err_handler());
    let mut sink = VecEventSink::default();
    let run = run_agent_run(
        request("run_fail", true),
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");
    assert_eq!(run.status, AgentRunStatus::PartialSuccess);
}

#[tokio::test]
async fn cancellation_stops_stream() {
    let root = tmp_root("cancel");
    let persistence = RunPersistence::new(&root);
    let cancel = CancellationToken::new();
    cancel.cancel(CancelReason::UserRequested);
    let provider = ScriptedProvider::new(vec![vec![Ok(text_delta(
        "hello",
        Some(ProviderFinishReason::Stop),
    ))]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let run = run_agent_run(
        request("run_cancel", true),
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel,
        },
        &mut sink,
    )
    .await
    .expect("run");
    assert_eq!(run.status, AgentRunStatus::Cancelled);
}

#[tokio::test]
async fn malformed_tool_arguments_fail_the_run() {
    let root = tmp_root("badargs");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![
        tool_delta("c1", "read_file", "{not-json")
            .into_iter()
            .map(Ok)
            .collect(),
    ]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let result = run_agent_run(
        request("run_bad", true),
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await;
    assert!(result.is_err());
    let loaded = persistence.load_run("run_bad").await.expect("load");
    assert_eq!(loaded.status, AgentRunStatus::Failed);
}

#[tokio::test]
async fn max_rounds_stops_repeat_tool_calls() {
    let root = tmp_root("rounds");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![
        tool_delta("c1", "read_file", "{\"path\":\"/tmp/x\"}")
            .into_iter()
            .map(Ok)
            .collect(),
        tool_delta("c2", "read_file", "{\"path\":\"/tmp/y\"}")
            .into_iter()
            .map(Ok)
            .collect(),
    ]);
    let mut executor = ToolExecutor::new();
    executor.register(ToolId::builtin("read_file").unwrap(), ok_handler());
    let mut sink = VecEventSink::default();
    let mut req = request("run_rounds", true);
    req.max_rounds = 1;
    let run = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");
    assert_eq!(run.status, AgentRunStatus::Failed);
}

#[tokio::test]
async fn persistence_rejects_unsafe_ids_and_keeps_event_order() {
    let root = tmp_root("persist");
    let persistence = RunPersistence::new(&root);
    let err = persistence
        .write_tool_args("run_ok", "../escape", &json!({}))
        .await
        .expect_err("unsafe id");
    assert!(matches!(err, PersistenceError::InvalidComponent(_)));

    let mut factory = EventFactory::new("run_ok");
    persistence
        .write_run(&harness_core::AgentRun {
            id: "run_ok".into(),
            workspace_id: "ws".into(),
            stable_chat_id: "chat".into(),
            chat_ref: AgentChatRef::Group {
                chat_id: "g".into(),
            },
            generation_type: GenerationType::Chat,
            profile_id: None,
            skill_scope_refs: Default::default(),
            persist_base_state_id: None,
            input_message_count: None,
            presentation: harness_core::AgentRunPresentation::Foreground,
            status: AgentRunStatus::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
    let first = factory.next("a", harness_core::AgentRunEventLevel::Info, json!({}));
    let second = factory.next("b", harness_core::AgentRunEventLevel::Info, json!({}));
    persistence.append_event("run_ok", &first).await.unwrap();
    persistence.append_event("run_ok", &second).await.unwrap();
    let events = persistence.load_events("run_ok").await.unwrap();
    assert_eq!(events[0].event_type, "a");
    assert_eq!(events[1].event_type, "b");
    assert_eq!(events[0].seq, 1);
    assert_eq!(events[1].seq, 2);
}

#[test]
fn delta_aggregator_joins_fragments_and_rejects_bad_json() {
    let mut agg = DeltaAggregator::default();
    agg.apply(&ProviderDelta {
        text: Some("he".into()),
        ..ProviderDelta::default()
    });
    agg.apply(&ProviderDelta {
        text: Some("llo".into()),
        tool_call: Some(ToolCallFragment {
            index: 0,
            call_id: Some("c1".into()),
            name: Some("read_file".into()),
            arguments: Some("{\"path\":".into()),
        }),
        ..ProviderDelta::default()
    });
    agg.apply(&ProviderDelta {
        tool_call: Some(ToolCallFragment {
            index: 0,
            call_id: None,
            name: None,
            arguments: Some("\"/tmp\"}".into()),
        }),
        ..ProviderDelta::default()
    });
    let finished = agg
        .finish(&|alias: &str| ToolId::builtin(alias).ok())
        .expect("ok json");
    assert_eq!(finished.text, "hello");
    assert_eq!(finished.tool_calls[0].arguments["path"], "/tmp");

    let mut bad = DeltaAggregator::default();
    bad.apply(&ProviderDelta {
        tool_call: Some(ToolCallFragment {
            index: 0,
            call_id: Some("c1".into()),
            name: Some("read_file".into()),
            arguments: Some("{".into()),
        }),
        ..ProviderDelta::default()
    });
    assert!(
        bad.finish(&|alias: &str| ToolId::builtin(alias).ok())
            .is_err()
    );
}

#[tokio::test]
async fn event_sinks_preserve_seq() {
    let mut factory = EventFactory::new("run");
    let mut vec_sink = VecEventSink::default();
    let (mut mpsc_sink, mut rx) = MpscEventSink::new();
    let event = factory.next("x", harness_core::AgentRunEventLevel::Info, json!({}));
    vec_sink.emit(event.clone());
    mpsc_sink.emit(event);
    assert_eq!(vec_sink.events[0].seq, 1);
    assert_eq!(rx.recv().await.unwrap().seq, 1);
}

#[tokio::test]
async fn cancel_token_wakes_waiters() {
    let token = CancellationToken::new();
    let waiter = token.clone();
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let handle = tokio::spawn(async move {
        waiter.cancelled().await;
        flag.store(true, Ordering::SeqCst);
    });
    token.cancel(CancelReason::UserRequested);
    handle.await.unwrap();
    assert!(done.load(Ordering::SeqCst));
    assert_eq!(token.reason().as_deref(), Some("user requested"));
}

#[tokio::test]
async fn provider_error_finish_marks_run_failed() {
    let root = tmp_root("provider-error");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Ok(ProviderDelta {
        finish_reason: Some(ProviderFinishReason::Error),
        ..ProviderDelta::default()
    })]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let result = run_agent_run(
        request("run_provider_error", true),
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        persistence
            .load_run("run_provider_error")
            .await
            .unwrap()
            .status,
        AgentRunStatus::Failed
    );
}

#[tokio::test]
async fn replayed_tool_call_executes_handler_once() {
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = count.clone();
    let handler: ToolHandler = Arc::new(move |invocation| {
        let counter = counter.clone();
        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            harness_tools::AgentToolResult {
                call_id: invocation.call_id,
                tool_id: invocation.tool_id,
                content: "ok".into(),
                structured: json!({}),
                is_error: false,
                error_code: None,
                resource_refs: Vec::new(),
            }
        })
    });
    let snapshot = snapshot();
    let turn =
        harness_tools::ToolTurnContract::all(&snapshot, harness_tools::ToolChoice::Auto).unwrap();
    let invocation = harness_tools::ToolInvocation {
        call_id: "replay".into(),
        tool_id: ToolId::builtin("read_file").unwrap(),
        arguments: json!({ "path": "/tmp/x" }),
        provider_metadata: json!(null),
    };
    let mut executor = ToolExecutor::new();
    executor.register(ToolId::builtin("read_file").unwrap(), handler);
    executor
        .execute(&snapshot, &turn, invocation.clone())
        .await
        .unwrap();
    executor
        .execute(&snapshot, &turn, invocation)
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn provider_failure_retries_then_succeeds() {
    let root = tmp_root("retry");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![
        vec![Err(harness_runtime::ProviderError::Failed("boom".into()))],
        vec![Ok(text_delta("hello", Some(ProviderFinishReason::Stop)))],
    ]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let mut req = request("run_retry", true);
    req.retry = AgentModelRetryPolicy {
        max_retries: 1,
        interval_ms: 0,
    };
    let run = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");
    assert_eq!(run.status, AgentRunStatus::Completed);
    let events = persistence.load_events("run_retry").await.unwrap();
    assert!(events.iter().any(|event| event.event_type == "model.retry"));
}

#[tokio::test]
async fn load_resumable_run_rejects_terminal_and_loads_active() {
    let root = tmp_root("resume");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Ok(text_delta(
        "hello",
        Some(ProviderFinishReason::Stop),
    ))]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let run = run_agent_run(
        request("run_resume", true),
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");
    assert!(load_resumable_run(&persistence, &run.id).await.is_err());

    let mut active = run.clone();
    active.status = AgentRunStatus::CallingModel;
    persistence.write_run(&active).await.unwrap();
    let (loaded, events) = load_resumable_run(&persistence, &active.id).await.unwrap();
    assert_eq!(loaded.status, AgentRunStatus::CallingModel);
    assert!(!events.is_empty());
}

#[tokio::test]
async fn retries_exhausted_marks_run_failed() {
    let root = tmp_root("retry-exhausted");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![
        vec![Err(harness_runtime::ProviderError::Failed("boom-1".into()))],
        vec![Err(harness_runtime::ProviderError::Failed("boom-2".into()))],
    ]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let mut req = request("run_retry_out", true);
    req.retry = AgentModelRetryPolicy {
        max_retries: 1,
        interval_ms: 0,
    };
    let result = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        persistence.load_run("run_retry_out").await.unwrap().status,
        AgentRunStatus::Failed
    );
}

#[tokio::test]
async fn malformed_tool_arguments_are_not_retried() {
    let root = tmp_root("no-retry-aggregate");
    let persistence = RunPersistence::new(&root);
    // Only one scripted round: a retry would panic the mock by popping an empty queue.
    let provider = ScriptedProvider::new(vec![
        tool_delta("c1", "read_file", "{not-json")
            .into_iter()
            .map(Ok)
            .collect(),
    ]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let mut req = request("run_no_retry", true);
    req.retry = AgentModelRetryPolicy {
        max_retries: 3,
        interval_ms: 0,
    };
    let result = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await;
    assert!(result.is_err());
    let events = persistence.load_events("run_no_retry").await.unwrap();
    assert!(
        !events.iter().any(|event| event.event_type == "model.retry"),
        "deterministic aggregate errors must not be retried"
    );
    assert_eq!(
        provider.calls(),
        1,
        "aggregate errors must not trigger another provider request"
    );
}

#[tokio::test]
async fn cancellation_interrupts_retry_backoff() {
    let root = tmp_root("retry-cancel");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Err(harness_runtime::ProviderError::Failed(
        "boom".into(),
    ))]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let cancel = CancellationToken::new();
    let waker = cancel.clone();
    let calls = provider.calls.clone();
    tokio::spawn(async move {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        waker.cancel(CancelReason::UserRequested);
    });
    let mut req = request("run_retry_cancel", true);
    req.retry = AgentModelRetryPolicy {
        max_retries: 1,
        // Long enough that an uninterruptible sleep would blow the assertion below.
        interval_ms: 30_000,
    };
    let started = std::time::Instant::now();
    let run = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel,
        },
        &mut sink,
    )
    .await
    .expect("cancelled run is not an error");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "cancellation must interrupt retry backoff"
    );
    assert_eq!(run.status, AgentRunStatus::Cancelled);
    assert_eq!(
        persistence
            .load_run("run_retry_cancel")
            .await
            .unwrap()
            .status,
        AgentRunStatus::Cancelled
    );
    assert_eq!(
        provider.calls(),
        1,
        "cancelled backoff must not re-issue the model request"
    );
}

#[tokio::test]
async fn retry_budget_is_per_run_not_per_round() {
    let root = tmp_root("retry-budget");
    let persistence = RunPersistence::new(&root);
    // Round 1 burns the single retry, then succeeds with a tool call. Round 2 fails
    // once more: with a per-run budget there is nothing left, so the run fails.
    let provider = ScriptedProvider::new(vec![
        vec![Err(harness_runtime::ProviderError::Failed("boom".into()))],
        tool_delta("c1", "read_file", "{\"path\":\"/tmp/x\"}")
            .into_iter()
            .map(Ok)
            .collect(),
        vec![Err(harness_runtime::ProviderError::Failed("boom".into()))],
    ]);
    let mut executor = ToolExecutor::new();
    executor.register(ToolId::builtin("read_file").unwrap(), ok_handler());
    let mut sink = VecEventSink::default();
    let mut req = request("run_budget", true);
    req.retry = AgentModelRetryPolicy {
        max_retries: 1,
        interval_ms: 0,
    };
    let result = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await;
    assert!(result.is_err(), "second-round failure has no retry left");
    let events = persistence.load_events("run_budget").await.unwrap();
    let retries = events
        .iter()
        .filter(|event| event.event_type == "model.retry")
        .count();
    assert_eq!(retries, 1, "retry budget must not reset per round");
    assert_eq!(provider.calls(), 3);
}

#[tokio::test]
async fn permanent_provider_error_is_not_retried() {
    // 401 / 模型不存在这类失败，重放同一请求必然再失败。gate 不得重试，
    // 也不得向 provider 发出第二次请求。
    let root = tmp_root("permanent");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Err(
        harness_runtime::ProviderError::Permanent("401 unauthorized".into()),
    )]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let mut req = request("run_permanent", true);
    req.retry = AgentModelRetryPolicy {
        max_retries: 3,
        interval_ms: 0,
    };
    let result = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        persistence.load_run("run_permanent").await.unwrap().status,
        AgentRunStatus::Failed
    );
    let events = persistence.load_events("run_permanent").await.unwrap();
    assert!(
        !events.iter().any(|event| event.event_type == "model.retry"),
        "permanent errors must not be retried"
    );
    assert_eq!(
        provider.calls(),
        1,
        "a rejected request must not be replayed"
    );
}

// ---------------------------------------------------------------------------
// GenerationType：工具 gate + continue 前缀
// ---------------------------------------------------------------------------

/// 非 chat 类型（quiet 为例）不得把 tools / tool_choice 发给 provider：
/// 对齐 ST `tool-calling.js canPerformToolCalls` 的 noToolCallTypes 语义。
#[tokio::test]
async fn quiet_generation_sends_no_tools() {
    let root = tmp_root("quiet");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Ok(text_delta(
        "background reply",
        Some(ProviderFinishReason::Stop),
    ))]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();

    let mut req = request("run_quiet", true);
    req.generation_type = GenerationType::Quiet;
    let run = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");

    assert_eq!(run.status, AgentRunStatus::Completed);
    assert_eq!(run.generation_type, GenerationType::Quiet);
    // run.json 持久化请求值
    let loaded = persistence.load_run("run_quiet").await.expect("load run");
    assert_eq!(loaded.generation_type, GenerationType::Quiet);

    let seen = provider.seen();
    assert_eq!(seen.len(), 1);
    assert!(
        seen[0].tools.is_empty(),
        "quiet 生成不得携带工具: {:?}",
        seen[0].tools
    );
    assert!(
        seen[0].tool_choice.is_none(),
        "quiet 生成不得下发 tool_choice: {:?}",
        seen[0].tool_choice
    );
}

/// chat 类型维持工具注册 + Auto 选择（回归保护）。
#[tokio::test]
async fn chat_generation_keeps_tools_auto() {
    let root = tmp_root("chat_tools");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Ok(text_delta(
        "ok",
        Some(ProviderFinishReason::Stop),
    ))]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();

    let req = request("run_chat", true);
    run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");

    let seen = provider.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].tools.len(), 1, "chat 保留 snapshot 绑定的工具");
    assert_eq!(seen[0].tool_choice, Some(ToolChoice::Auto));
}

/// continue 类型：末条 assistant 消息文本作为前缀与续写输出拼接，
/// 对齐 ST StreamingProcessor.continueMessage（最终消息 = 前缀 + 续写）。
#[tokio::test]
async fn continue_generation_prepends_assistant_prefix() {
    let root = tmp_root("continue");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Ok(text_delta(
        " continued tail",
        Some(ProviderFinishReason::Stop),
    ))]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();

    let mut req = request("run_continue", true);
    req.generation_type = GenerationType::Continue;
    req.prompt.messages.push(AgentModelMessage::text(
        AgentModelRole::Assistant,
        "the prefix text",
    ));
    let run = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");

    assert_eq!(run.status, AgentRunStatus::Completed);
    // 持久化的 model response text == 前缀 + 续写
    let events = persistence
        .load_events("run_continue")
        .await
        .expect("events");
    let delta = events
        .iter()
        .find(|event| event.event_type == "model.delta")
        .expect("model.delta event");
    let text = delta.payload["text"].as_str().expect("text payload");
    assert_eq!(text, "the prefix text continued tail");

    // continue 不带工具
    let seen = provider.seen();
    assert!(seen[0].tools.is_empty());
    assert!(seen[0].tool_choice.is_none());
}

/// impersonate 同样走无工具 gate（结果写用户消息位由调用方路由，harness 只透传类型）。
#[tokio::test]
async fn impersonate_generation_sends_no_tools() {
    let root = tmp_root("impersonate");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![vec![Ok(text_delta(
        "*waves*",
        Some(ProviderFinishReason::Stop),
    ))]]);
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();

    let mut req = request("run_imp", true);
    req.generation_type = GenerationType::Impersonate;
    run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");

    let seen = provider.seen();
    assert!(seen[0].tools.is_empty());
    assert!(seen[0].tool_choice.is_none());
}

// ---------------------------------------------------------------------------
// Stealth 注解 + response_format 透传
// ---------------------------------------------------------------------------

fn stealth_descriptor(id: ToolId) -> ToolDescriptor {
    // annotations.stealth 严格布尔 true 才生效（对齐 ST ToolDefinition.stealth）
    let mut descriptor = descriptor(id);
    descriptor.annotations = serde_json::json!({ "stealth": true });
    descriptor
}

fn stealth_snapshot() -> InvocationToolSnapshot {
    let id = ToolId::builtin("read_file").expect("tool id");
    InvocationToolSnapshot::try_new(
        ToolSnapshotId::parse("inv_root").expect("snapshot"),
        vec![ToolBinding::new(stealth_descriptor(id), "read_file", Some(4)).unwrap()],
        8,
    )
    .expect("snapshot")
}

fn mixed_snapshot() -> InvocationToolSnapshot {
    let read = ToolId::builtin("read_file").expect("tool id");
    let tell = ToolId::builtin("tell_secret").expect("tool id");
    InvocationToolSnapshot::try_new(
        ToolSnapshotId::parse("inv_root").expect("snapshot"),
        vec![
            ToolBinding::new(descriptor(read), "read_file", Some(4)).unwrap(),
            ToolBinding::new(stealth_descriptor(tell), "tell_secret", Some(4)).unwrap(),
        ],
        8,
    )
    .expect("snapshot")
}

fn tool_result_delta(call_id: &str) -> Vec<ProviderDelta> {
    tool_delta(call_id, "read_file", "{\"path\":\"/tmp/x\"}")
}

/// 纯 stealth 轮：执行 + 落盘 + 事件后直接终结 run，不触发下一轮模型调用
/// （对齐 ST「stealth 不触发后续生成」；provider.calls 断言防孤儿转写反复调用）。
#[tokio::test]
async fn all_stealth_round_ends_run_without_followup() {
    let root = tmp_root("stealth_all");
    let persistence = RunPersistence::new(&root);
    let provider =
        ScriptedProvider::new(vec![tool_result_delta("c1").into_iter().map(Ok).collect()]);
    let mut executor = ToolExecutor::new();
    executor.register(ToolId::builtin("read_file").expect("id"), ok_handler());
    let mut sink = VecEventSink::default();

    let mut req = request("run_stealth", true);
    req.snapshot = stealth_snapshot();
    let run = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");

    assert_eq!(run.status, AgentRunStatus::Completed);
    // 只调用一次模型：结果不触发后续生成
    assert_eq!(provider.calls(), 1);
    // tool.completed 事件带 stealth 标记（事件走 sink，不落 events.jsonl）
    let completed = sink
        .events
        .iter()
        .find(|event| event.event_type == "tool.completed")
        .expect("tool.completed");
    assert_eq!(completed.payload["stealth"], serde_json::json!(true));
    // tool-results/ 照常落盘
    let dir = root.join("run_stealth");
    assert!(
        std::fs::read_dir(dir.join("tool-results"))
            .expect("tool-results dir")
            .count()
            > 0
    );
}

/// 混合轮：stealth 结果仍回灌（OpenAI 转写要求 tool_call 与 result 一一配对），
/// 事件带 stealth 标记，run 正常走到下一轮模型调用。
#[tokio::test]
async fn mixed_stealth_round_still_feeds_back() {
    let root = tmp_root("stealth_mixed");
    let persistence = RunPersistence::new(&root);
    // 第一轮：两个 tool call（一个普通 read_file，一个 stealth tell_secret）
    // 第二轮：纯文本收尾
    // 两个 call 需要不同 fragment index（聚合器按 index 区分 call），
    // finish_reason 只在最后一个 delta 上发一次
    let round1 = vec![
        ProviderDelta {
            tool_call: Some(ToolCallFragment {
                index: 0,
                call_id: Some("c1".into()),
                name: Some("read_file".into()),
                arguments: Some("{\"path\":\"/tmp/x\"}".into()),
            }),
            ..ProviderDelta::default()
        },
        ProviderDelta {
            tool_call: Some(ToolCallFragment {
                index: 1,
                call_id: Some("c2".into()),
                name: Some("tell_secret".into()),
                arguments: Some("{\"path\":\"/tmp/y\"}".into()),
            }),
            finish_reason: Some(ProviderFinishReason::ToolCalls),
            ..ProviderDelta::default()
        },
    ];
    let provider = ScriptedProvider::new(vec![
        round1.into_iter().map(Ok).collect(),
        vec![Ok(text_delta("done", Some(ProviderFinishReason::Stop)))],
    ]);
    let mut executor = ToolExecutor::new();
    executor.register(ToolId::builtin("read_file").expect("id"), ok_handler());
    executor.register(ToolId::builtin("tell_secret").expect("id"), ok_handler());
    let mut sink = VecEventSink::default();

    let mut req = request("run_mixed", true);
    req.snapshot = mixed_snapshot();
    let run = run_agent_run(
        req,
        AgentRunDeps {
            provider: &provider,
            executor: &mut executor,
            persistence: &persistence,
            cancel: CancellationToken::new(),
        },
        &mut sink,
    )
    .await
    .expect("run");

    assert_eq!(run.status, AgentRunStatus::Completed);
    // 第二轮模型调用发生：混合轮结果照常回灌（转写配对合法）
    assert_eq!(provider.calls(), 2);
    let stealth_flags: Vec<_> = sink
        .events
        .iter()
        .filter(|event| event.event_type == "tool.completed")
        .map(|event| event.payload["stealth"] == serde_json::json!(true))
        .collect();
    assert_eq!(stealth_flags, vec![false, true]);
}

/// response_format 字段：Some 时序列化出现（camelCase responseFormat），None 时跳过。
#[test]
fn response_format_passthrough_serde() {
    use harness_runtime::ProviderRequest;
    let mut req: ProviderRequest = serde_json::from_str(
        r#"{"model":"m","messages":[],"responseFormat":{"type":"json_schema"}}"#,
    )
    .expect("deserialize with responseFormat");
    assert_eq!(
        req.response_format,
        Some(serde_json::json!({ "type": "json_schema" }))
    );
    // 序列化回带该字段
    let back = serde_json::to_value(&req).expect("serialize");
    assert_eq!(back["responseFormat"]["type"], "json_schema");
    // None 时不出现（skip_serializing_if）
    req.response_format = None;
    let back = serde_json::to_value(&req).expect("serialize none");
    assert!(back.get("responseFormat").is_none());
}
