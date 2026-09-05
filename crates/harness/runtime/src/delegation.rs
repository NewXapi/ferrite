//! Agent 委派 / 子 run（handover §二 H4）。
//!
//! 无 SillyTavern 参照（单角色），语义参照 TauriTavern `loop_runner.rs` 的
//! `AgentLoopExit`：宿主通过 `delegated_task` 工具把子任务交给目标 profile
//! 驱动的子 run，子 run 完成后结果按预算截断回灌父 run。
//!
//! MVP 约束（ponytail：顺序执行，不做并发调度——
//! `max_concurrent_invocations` 记录进 gate 语义但执行为逐个串行）：
//! - 深度 gate：父链深度（invocation 链长）≥ `max_handoff_depth` → 拒绝
//! - 数量 gate：本 run 已发起委托数 ≥ `max_invocations_per_run` → 拒绝
//! - 拒绝以工具错误结果返回（模型可感知并可改道），不 panic、不中断父 run
//!
//! 持久化：`AgentInvocation` / `AgentTaskRecord` 追加到
//! `invocations.jsonl` / `tasks.jsonl`（run 目录内，与 events 同层）。
//! tasks.jsonl 为 append-log：每个 task id 的最新记录即当前状态
//! （注册时 Queued，子 run 结束后追加 Completed/Failed 终态行）。

use chrono::Utc;
use harness_core::{
    AgentDelegationContinuation, AgentDelegationPolicy, AgentInvocation, AgentInvocationKind,
    AgentInvocationStatus, AgentRunStatus, AgentTaskRecord, AgentTaskStatus,
};
use harness_tools::{AgentToolResult, ToolId};
use serde_json::json;

use crate::loop_engine::{AgentRunDeps, AgentRunRequest};
use crate::persistence::RunPersistence;

/// 一次委托请求的全部输入（由 `delegated_task` 工具调用的 arguments 解出）。
#[derive(Debug, Clone)]
pub struct DelegationRequest {
    /// 目标 profile id（子 run 用它构造自己的 profile_id）。
    pub target_profile_id: String,
    /// 子任务描述，成为子 run 的初始 user 消息。
    pub task: String,
    /// 结果回灌方式。
    pub continuation: AgentDelegationContinuation,
}

/// 委派 gate 校验错误；转为工具错误结果回灌模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelegationError {
    /// 委派深度超过 `max_handoff_depth`。
    DepthExceeded,
    /// 本 run 委派数已达 `max_invocations_per_run`。
    BudgetExceeded,
    /// 目标 profile 不允许被委托（策略未开启）。
    TargetNotAllowed,
    /// 委派被策略禁用。
    DelegationDisabled,
}

impl DelegationError {
    /// 工具错误码（与 persistence `error_code` 语义一致）。
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::DepthExceeded => "delegation_depth_exceeded",
            Self::BudgetExceeded => "delegation_budget_exceeded",
            Self::TargetNotAllowed => "delegation_target_not_allowed",
            Self::DelegationDisabled => "delegation_disabled",
        }
    }

    /// 面向模型的可读错误文本。
    pub fn message(&self) -> String {
        match self {
            Self::DepthExceeded => {
                "delegation depth exceeded: cannot nest deeper than the profile allows".into()
            }
            Self::BudgetExceeded => {
                "delegation budget exceeded: this run has started its maximum number of sub-runs"
                    .into()
            }
            Self::TargetNotAllowed => "target profile does not accept delegated tasks".into(),
            Self::DelegationDisabled => "this profile is not allowed to delegate tasks".into(),
        }
    }
}

/// 委派 gate：开关 / 深度 / 数量 / 目标允许校验。
///
/// - `chain_depth`：当前 invocation 链深度（root = 0）
/// - `delegations_so_far`：本 run 已发起的委托次数
/// - `policy`：**发起方** profile 的委派策略
/// - `target_allows`：目标 profile 的 `allow_as_subagent`（handoff 另算）
/// - `is_handoff`：handoff 语义额外要求 `can_handoff`
pub fn check_delegation(
    chain_depth: usize,
    delegations_so_far: usize,
    policy: &AgentDelegationPolicy,
    target_allows: bool,
    is_handoff: bool,
) -> Result<(), DelegationError> {
    if !policy.can_delegate {
        return Err(DelegationError::DelegationDisabled);
    }
    if is_handoff && !policy.can_handoff {
        return Err(DelegationError::DelegationDisabled);
    }
    if !target_allows {
        return Err(DelegationError::TargetNotAllowed);
    }
    if chain_depth + 1 > policy.max_handoff_depth {
        return Err(DelegationError::DepthExceeded);
    }
    if delegations_so_far >= policy.max_invocations_per_run {
        return Err(DelegationError::BudgetExceeded);
    }
    Ok(())
}

/// 为一次委托构造子 run 的 `AgentRunRequest`。
///
/// 子 run 沿用父 run 的 model / workspace / chat 上下文与生成类型，
/// profile_id 换成目标 profile；任务描述成为初始 user 消息。
pub fn build_child_request(
    parent: &AgentRunRequest,
    child_run_id: &str,
    req: &DelegationRequest,
) -> AgentRunRequest {
    let mut request = parent.clone();
    request.run_id = child_run_id.to_string();
    request.profile_id = Some(req.target_profile_id.clone());
    request.prompt.messages = vec![harness_prompt::AgentModelMessage::text(
        harness_prompt::AgentModelRole::User,
        req.task.clone(),
    )];
    request
}

/// 把一次委托登记为 invocation + task record 并落盘（status = Queued，
/// 随后由 [`run_delegated_task`] 驱动子 run 并更新状态）。
pub async fn register_delegation(
    parent_run: &RunPersistence,
    parent_run_id: &str,
    parent_invocation_id: &str,
    child_invocation_id: &str,
    req: &DelegationRequest,
) -> AgentTaskRecord {
    let now = Utc::now();
    let invocation = AgentInvocation {
        id: child_invocation_id.to_string(),
        run_id: parent_run_id.to_string(),
        parent_invocation_id: Some(parent_invocation_id.to_string()),
        profile_id: req.target_profile_id.clone(),
        kind: AgentInvocationKind::Subagent,
        status: AgentInvocationStatus::Created,
        exit_policy: harness_core::AgentInvocationExitPolicy::RunFinishAllowed,
        created_at: now,
        updated_at: now,
    };
    let task = AgentTaskRecord {
        id: format!("task-{child_invocation_id}"),
        run_id: parent_run_id.to_string(),
        parent_invocation_id: parent_invocation_id.to_string(),
        child_invocation_id: child_invocation_id.to_string(),
        target_profile_id: req.target_profile_id.clone(),
        workspace_key: String::new(),
        continuation: req.continuation,
        status: AgentTaskStatus::Queued,
        task: json!({ "task": req.task }),
        created_by_tool_call_id: None,
        result_ref: None,
        error: None,
        created_at: now,
        updated_at: now,
    };
    parent_run
        .append_jsonl(parent_run_id, "invocations.jsonl", &invocation)
        .await
        .ok();
    parent_run
        .append_jsonl(parent_run_id, "tasks.jsonl", &task)
        .await
        .ok();
    task
}

/// 按预算截断子 run 结果，回灌父 run 的工具结果文本。
///
/// 预算按 token 计，字符上限 = `budget_tokens * 4`（与 guesstimate 同源）；
/// 截断点向回退到 utf8 边界并追加 `[truncated]` 标记。
pub fn truncate_result(text: &str, budget_tokens: usize) -> String {
    let max_chars = budget_tokens.saturating_mul(4).max(1);
    if text.len() <= max_chars {
        return text.to_string();
    }
    let mut cut = max_chars;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}\n[truncated]", &text[..cut])
}

/// 驱动一次委托：登记 → 驱动子 run → 更新 task 状态 → 返回回灌工具结果。
///
/// 借用契约：executor 以 `&mut` 显式传入（宿主的 delegated_task handler 闭包
/// 自行决定 executor 归属——新建实例或经锁共享；子 run 入口会重置 gate 预算）。
/// 子 run 的失败不传染父 run：失败转成工具错误结果文本（模型可重试或改道）。
#[allow(clippy::too_many_arguments)]
pub async fn run_delegated_task<P: crate::provider::ChatProvider>(
    parent_persistence: &RunPersistence,
    parent_run_id: &str,
    parent_invocation_id: &str,
    chain_depth: usize,
    delegations_so_far: usize,
    policy: &AgentDelegationPolicy,
    target_allows: bool,
    result_budget_tokens: usize,
    call_id: String,
    req: DelegationRequest,
    child_request: AgentRunRequest,
    provider: &P,
    executor: &mut crate::tool_exec::ToolExecutor,
    persistence: &RunPersistence,
    cancel: crate::cancel::CancellationToken,
    sink: &mut impl crate::event_sink::EventSink,
) -> AgentToolResult {
    let error_result = |code: &str, message: String| AgentToolResult {
        call_id: call_id.clone(),
        tool_id: ToolId::builtin("delegated_task").expect("builtin id"),
        content: message,
        structured: json!({ "code": code }),
        is_error: true,
        error_code: Some(code.to_string()),
        resource_refs: Vec::new(),
    };

    if let Err(error) = check_delegation(
        chain_depth,
        delegations_so_far,
        policy,
        target_allows,
        false,
    ) {
        return error_result(error.error_code(), error.message());
    }

    let child_invocation_id = format!("inv-{parent_run_id}-{}", delegations_so_far + 1);
    let mut task = register_delegation(
        parent_persistence,
        parent_run_id,
        parent_invocation_id,
        &child_invocation_id,
        &req,
    )
    .await;

    async fn persist_task(
        parent_persistence: &RunPersistence,
        parent_run_id: &str,
        task: &AgentTaskRecord,
    ) {
        parent_persistence
            .append_jsonl(parent_run_id, "tasks.jsonl", task)
            .await
            .ok();
    }
    let child_run = crate::loop_engine::run_agent_run(
        child_request,
        AgentRunDeps {
            provider,
            executor,
            persistence,
            cancel,
        },
        sink,
    )
    .await;

    match child_run {
        Ok(child) => {
            let result_text = if child.status == AgentRunStatus::Failed {
                format!("child run {} failed", child.id)
            } else {
                format!("child run {} completed ({:?})", child.id, child.status)
            };
            let feedback = truncate_result(&result_text, result_budget_tokens);
            task.status = if child.status == AgentRunStatus::Failed {
                AgentTaskStatus::Failed
            } else {
                AgentTaskStatus::Completed
            };
            task.result_ref = Some(format!("runs/{}/run.json", child.id));
            persist_task(parent_persistence, parent_run_id, &task).await;
            AgentToolResult {
                call_id,
                tool_id: ToolId::builtin("delegated_task").expect("builtin id"),
                content: feedback,
                structured: json!({ "childRunId": child.id }),
                is_error: false,
                error_code: None,
                resource_refs: Vec::new(),
            }
        }
        Err(error) => {
            task.status = AgentTaskStatus::Failed;
            task.error = Some(error.to_string());
            persist_task(parent_persistence, parent_run_id, &task).await;
            error_result(
                "delegation_child_failed",
                format!("child run failed: {error}"),
            )
        }
    }
}
