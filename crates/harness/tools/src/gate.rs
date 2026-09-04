//! 工具调用预算与授权闸门。
//!
//! 整文件照抄自 TauriTavern
//! `tt-application/src/services/tool_request_gate.rs:1-113`。

use std::collections::HashMap;

use thiserror::Error;

use crate::spec::{
    InvocationToolSnapshot, ToolChoice, ToolId, ToolInvocation, ToolSnapshotId, ToolTurnContract,
};

#[derive(Debug, Default)]
pub struct ToolRequestGate {
    total_calls: usize,
    calls_per_tool: HashMap<ToolId, usize>,
    // Provider 重放同一 call id 时，只有同一个 snapshot 内的同一个工具才幂等。
    // 新 snapshot 有独立预算，不能复用旧 snapshot 的 reservation。
    reserved_calls: HashMap<(ToolSnapshotId, String), ToolId>,
}

impl ToolRequestGate {
    pub fn authorize_and_reserve(
        &mut self,
        snapshot: &InvocationToolSnapshot,
        turn: &ToolTurnContract,
        invocation: &ToolInvocation,
    ) -> Result<(), ToolRequestGateError> {
        if turn.snapshot_id() != snapshot.id() {
            return Err(ToolRequestGateError::TurnSnapshotMismatch {
                turn_snapshot_id: turn.snapshot_id().clone(),
                invocation_snapshot_id: snapshot.id().clone(),
            });
        }

        let binding = snapshot.binding(&invocation.tool_id).ok_or_else(|| {
            ToolRequestGateError::ToolNotInSnapshot {
                tool_id: invocation.tool_id.clone(),
                snapshot_id: snapshot.id().clone(),
            }
        })?;

        match turn.choice() {
            ToolChoice::None => {
                return Err(ToolRequestGateError::ToolChoiceNone {
                    tool_id: invocation.tool_id.clone(),
                });
            }
            ToolChoice::Specific(required_tool_id) if required_tool_id != &invocation.tool_id => {
                return Err(ToolRequestGateError::ToolChoiceSpecific {
                    tool_id: invocation.tool_id.clone(),
                    required_tool_id: required_tool_id.clone(),
                });
            }
            ToolChoice::Auto | ToolChoice::Required | ToolChoice::Specific(_) => {}
        }

        let reservation_key = (snapshot.id().clone(), invocation.call_id.clone());
        if self.reserved_calls.get(&reservation_key) == Some(&invocation.tool_id) {
            return Ok(());
        }

        let max_calls = snapshot.max_calls_per_invocation();
        if self.total_calls >= max_calls {
            return Err(ToolRequestGateError::InvocationBudgetExhausted { max_calls });
        }

        let tool_calls = self
            .calls_per_tool
            .get(&invocation.tool_id)
            .copied()
            .unwrap_or(0);
        if let Some(max_calls) = binding.max_calls()
            && tool_calls >= max_calls
        {
            return Err(ToolRequestGateError::ToolBudgetExhausted {
                tool_id: invocation.tool_id.clone(),
                max_calls,
            });
        }

        self.reserved_calls
            .insert(reservation_key, invocation.tool_id.clone());
        self.total_calls += 1;
        *self
            .calls_per_tool
            .entry(invocation.tool_id.clone())
            .or_insert(0) += 1;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolRequestGateError {
    #[error(
        "tool.turn_snapshot_mismatch: turn references snapshot `{turn_snapshot_id}` but invocation uses `{invocation_snapshot_id}`"
    )]
    TurnSnapshotMismatch {
        turn_snapshot_id: ToolSnapshotId,
        invocation_snapshot_id: ToolSnapshotId,
    },
    #[error(
        "model.unknown_tool_call: tool `{tool_id}` is not available in snapshot `{snapshot_id}`"
    )]
    ToolNotInSnapshot {
        tool_id: ToolId,
        snapshot_id: ToolSnapshotId,
    },
    #[error(
        "model.tool_choice_violation: tool `{tool_id}` is forbidden by the current tool choice"
    )]
    ToolChoiceNone { tool_id: ToolId },
    #[error(
        "model.tool_choice_violation: current tool choice requires `{required_tool_id}`, not `{tool_id}`"
    )]
    ToolChoiceSpecific {
        tool_id: ToolId,
        required_tool_id: ToolId,
    },
    #[error(
        "agent.tool_budget_exhausted: invocation tool call budget is exhausted (max {max_calls})"
    )]
    InvocationBudgetExhausted { max_calls: usize },
    #[error(
        "agent.tool_budget_exhausted: tool `{tool_id}` call budget is exhausted (max {max_calls})"
    )]
    ToolBudgetExhausted { tool_id: ToolId, max_calls: usize },
}
