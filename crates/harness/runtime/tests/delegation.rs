//! delegation 模块集成测试。
//!
//! 覆盖：gate 校验（深度/数量/开关/目标允许）、子 run 驱动与状态回写、
//! 结果预算截断（utf8 边界）、gate 拒绝时不驱动子 run。

use harness_core::{
    AgentChatRef, AgentDelegationContinuation, AgentDelegationPolicy, AgentModelRetryPolicy,
    GenerationType,
};
use harness_prompt::{AgentModelMessage, AgentModelRequest, AgentModelRole};
use harness_runtime::{
    AgentRunRequest, CancellationToken, ChatProvider, DelegationRequest, ProviderDelta,
    ProviderFinishReason, ProviderRequest, ProviderStream, RunPersistence, ToolExecutor,
    VecEventSink, check_delegation, truncate_result,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct ScriptedProvider {
    rounds: Mutex<Vec<Vec<Result<ProviderDelta, harness_runtime::ProviderError>>>>,
    calls: Arc<AtomicUsize>,
}

impl ScriptedProvider {
    fn new(rounds: Vec<Vec<Result<ProviderDelta, harness_runtime::ProviderError>>>) -> Self {
        Self {
            rounds: Mutex::new(rounds),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl ChatProvider for ScriptedProvider {
    fn stream(&self, _r: ProviderRequest, _c: CancellationToken) -> ProviderStream {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut rounds = self.rounds.lock().expect("mutex");
        let deltas = if rounds.is_empty() {
            Vec::new()
        } else {
            rounds.remove(0)
        };
        drop(rounds);
        Box::pin(futures_util::stream::iter(deltas))
    }
}

fn policy() -> AgentDelegationPolicy {
    AgentDelegationPolicy {
        can_delegate: true,
        ..AgentDelegationPolicy::default()
    }
}

fn parent_request(run_id: &str) -> AgentRunRequest {
    AgentRunRequest {
        run_id: run_id.into(),
        workspace_id: "ws".into(),
        stable_chat_id: "chat".into(),
        chat_ref: AgentChatRef::Character {
            character_id: "alice".into(),
            file_name: "alice".into(),
        },
        profile_id: Some("parent".into()),
        generation_type: GenerationType::Chat,
        model: "m".into(),
        prompt: AgentModelRequest {
            system: Some("sys".into()),
            messages: vec![AgentModelMessage::text(
                AgentModelRole::User,
                "delegate please",
            )],
            tools: vec![],
            metadata: None,
        },
        snapshot: harness_tools::InvocationToolSnapshot::try_new(
            harness_tools::ToolSnapshotId::parse("inv_root").unwrap(),
            vec![],
            8,
        )
        .unwrap(),
        max_rounds: 2,
        retry: AgentModelRetryPolicy {
            max_retries: 0,
            interval_ms: 0,
        },
    }
}

fn tmp_root(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ferrite-deleg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

// gate：开关 / 深度 / 数量 / 目标允许，逐条断言错误码
#[test]
fn check_delegation_gates() {
    let p = policy();
    // 正常路径
    assert!(check_delegation(0, 0, &p, true, false).is_ok());
    // 禁用委派
    let off = AgentDelegationPolicy {
        can_delegate: false,
        ..policy()
    };
    assert_eq!(
        check_delegation(0, 0, &off, true, false)
            .unwrap_err()
            .error_code(),
        "delegation_disabled"
    );
    // 目标不允许
    assert_eq!(
        check_delegation(0, 0, &p, false, false)
            .unwrap_err()
            .error_code(),
        "delegation_target_not_allowed"
    );
    // 深度超限：chain_depth 8 + 1 > max 8
    assert_eq!(
        check_delegation(8, 0, &p, true, false)
            .unwrap_err()
            .error_code(),
        "delegation_depth_exceeded"
    );
    // 数量超限
    assert_eq!(
        check_delegation(0, 8, &p, true, false)
            .unwrap_err()
            .error_code(),
        "delegation_budget_exceeded"
    );
}

/// 结果预算截断：超预算按 token×4 折算字符截断并加标记，utf8 边界安全。
#[test]
fn truncate_result_respects_budget_and_char_boundary() {
    assert_eq!(truncate_result("short", 8), "short");
    let long = "汉".repeat(100); // 300 bytes
    let out = truncate_result(&long, 10); // 40 bytes 上限
    assert!(out.ends_with("[truncated]"));
    assert!(out.chars().count() < 100);
    // utf8 边界：截断点落在多字节字符中间时向回退，不 panic
    let text = "汉汉汉"; // 每字 3 bytes
    let out = truncate_result(text, 1); // 4 bytes 上限 → 截到 1 字符（3 bytes）
    assert!(out.starts_with("汉"));
}

/// 端到端：register 落盘 + 子 run 驱动 + task 状态回写 + 子 run.json 完成。
#[tokio::test]
async fn delegation_drives_child_run_and_persists_records() {
    let root = tmp_root("drive");
    let persistence = RunPersistence::new(&root);

    let child_deltas = vec![ProviderDelta {
        text: Some("child work done".into()),
        finish_reason: Some(ProviderFinishReason::Stop),
        ..ProviderDelta::default()
    }];
    let provider = ScriptedProvider::new(vec![
        child_deltas.into_iter().map(Ok).collect(), // 子 run
        vec![Ok(ProviderDelta {
            text: Some("parent continues".into()),
            finish_reason: Some(ProviderFinishReason::Stop),
            ..ProviderDelta::default()
        })], // 父 run 后续轮（模拟宿主把回灌文本拼回父上下文后继续）
    ]);

    let req = DelegationRequest {
        target_profile_id: "child-profile".into(),
        task: "summarize the archive".into(),
        continuation: AgentDelegationContinuation::ReturnToParent,
    };
    let child_request =
        harness_runtime::build_child_request(&parent_request("child-run-1"), "child-run-1", &req);

    let policy = policy();
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let tool_result = harness_runtime::run_delegated_task(
        &persistence,
        "parent-run-1",
        "inv-parent",
        0,
        0,
        &policy,
        true,
        1000,
        "call-1".into(),
        req,
        child_request,
        &provider,
        &mut executor,
        &persistence,
        CancellationToken::new(),
        &mut sink,
    )
    .await;

    // 回灌文本非错误、含完成语义、call_id 原样带回（转写配对需要）
    assert!(!tool_result.is_error);
    assert!(tool_result.content.contains("completed"));
    assert_eq!(tool_result.call_id, "call-1");
    assert_eq!(tool_result.structured["childRunId"], "child-run-1");

    // invocations.jsonl / tasks.jsonl 落盘且字段正确
    let invocations = std::fs::read_to_string(root.join("parent-run-1").join("invocations.jsonl"))
        .expect("invocations.jsonl");
    assert!(invocations.contains("\"kind\":\"subagent\""));
    assert!(invocations.contains("child-profile"));
    let tasks = std::fs::read_to_string(root.join("parent-run-1").join("tasks.jsonl"))
        .expect("tasks.jsonl");
    assert!(tasks.contains("\"status\":\"completed\""));

    // 子 run run.json 落盘且 Completed（pretty JSON，解析断言避免格式耦合）
    let child_run_json =
        std::fs::read_to_string(root.join("child-run-1").join("run.json")).expect("child run.json");
    let child_json: serde_json::Value = serde_json::from_str(&child_run_json).expect("valid json");
    assert_eq!(child_json["status"], "completed");

    // 只驱动了子 run：一次模型调用（父续轮由宿主拼回回灌文本后另行驱动）
    assert_eq!(provider.calls(), 1);
}

/// gate 拒绝路径：预算超限 → 工具错误结果，不驱动子 run（子 run.json 不落盘）。
#[tokio::test]
async fn delegation_gate_rejection_skips_child_run() {
    let root = tmp_root("gate");
    let persistence = RunPersistence::new(&root);
    let provider = ScriptedProvider::new(vec![]);

    let req = DelegationRequest {
        target_profile_id: "child-profile".into(),
        task: "task".into(),
        continuation: AgentDelegationContinuation::ReturnToParent,
    };
    let child_request =
        harness_runtime::build_child_request(&parent_request("child-run-x"), "child-run-x", &req);
    let policy = policy();
    let mut executor = ToolExecutor::new();
    let mut sink = VecEventSink::default();
    let tool_result = harness_runtime::run_delegated_task(
        &persistence,
        "parent-run-1",
        "inv-parent",
        0,
        8, // 已达上限（默认 max_invocations_per_run = 8）
        &policy,
        true,
        1000,
        "call-2".into(),
        req,
        child_request,
        &provider,
        &mut executor,
        &persistence,
        CancellationToken::new(),
        &mut sink,
    )
    .await;

    assert!(tool_result.is_error);
    assert_eq!(
        tool_result.error_code,
        Some("delegation_budget_exceeded".into())
    );
    assert_eq!(tool_result.call_id, "call-2");
    assert_eq!(provider.calls(), 0);
    assert!(!root.join("child-run-x").exists());
}

/// append_jsonl 的 file_name 是路径拼接输入（信任边界）：路径穿越必须被拒绝
/// （ocr security·high 回归）。
#[tokio::test]
async fn append_jsonl_rejects_path_traversal() {
    let root = tmp_root("traversal");
    let persistence = RunPersistence::new(&root);
    let record = serde_json::json!({ "k": "v" });
    let legit = persistence
        .append_jsonl("parent-run-1", "tasks.jsonl", &record)
        .await;
    assert!(legit.is_ok());
    // `..` 穿越 → 拒绝
    let traversal = persistence
        .append_jsonl("parent-run-1", "../tasks.jsonl", &record)
        .await;
    assert!(traversal.is_err());
    // 绝对路径 → 拒绝
    let absolute = persistence
        .append_jsonl("parent-run-1", "/etc/evil.jsonl", &record)
        .await;
    assert!(absolute.is_err());
    // 穿越目标文件不存在（拒绝发生在写盘前）
    assert!(!root.join("tasks.jsonl").exists());
    assert!(!std::path::Path::new("/etc/evil.jsonl").exists());
}
