//! 工具调用授权闸门测试。
//!
//! 覆盖：
//! - snapshot 不匹配 → 拒绝
//! - `ToolChoice::None` → 拒绝
//! - 超 `max_calls` → 拒绝
//! - `ToolChoice::Specific` 调其他工具 → 拒绝

use harness_tools::{
    InvocationToolSnapshot, ToolBinding, ToolChoice, ToolDescriptor, ToolError, ToolId,
    ToolInvocation, ToolRequestGate, ToolRequestGateError, ToolSnapshotId, ToolTurnContract,
};
use serde_json::{Value, json};

fn descriptor(id: ToolId) -> ToolDescriptor {
    ToolDescriptor {
        id,
        title: None,
        description: None,
        input_schema: json!({ "type": "object" }),
        output_schema: None,
        annotations: json!({}),
    }
}

fn build_snapshot(
    snapshot_id: &str,
    bindings: Vec<ToolBinding>,
    max_calls_per_invocation: usize,
) -> InvocationToolSnapshot {
    InvocationToolSnapshot::try_new(
        ToolSnapshotId::parse(snapshot_id).expect("snapshot id"),
        bindings,
        max_calls_per_invocation,
    )
    .expect("snapshot")
}

#[test]
fn snapshot_mismatch_is_rejected() {
    let read_id = ToolId::builtin("read_file").expect("tool id");
    let read_binding = ToolBinding::new(descriptor(read_id.clone()), "read", Some(2)).unwrap();

    let snapshot = build_snapshot("inv_root", vec![read_binding], 8);
    // Turn refers to a different snapshot than the invocation uses.
    let turn = ToolTurnContract::all(
        &build_snapshot("inv_other", vec![], 8),
        ToolChoice::Auto,
    )
    .expect("turn");

    let mut gate = ToolRequestGate::default();
    let invocation = ToolInvocation {
        call_id: "c1".to_string(),
        tool_id: read_id,
        arguments: json!({}),
        provider_metadata: Value::Null,
    };

    let err = gate
        .authorize_and_reserve(&snapshot, &turn, &invocation)
        .expect_err("must reject mismatched snapshot");
    assert!(matches!(
        err,
        ToolRequestGateError::TurnSnapshotMismatch { .. }
    ));
}

#[test]
fn tool_choice_none_rejects_any_invocation() {
    let read_id = ToolId::builtin("read_file").expect("tool id");
    let read_binding = ToolBinding::new(descriptor(read_id.clone()), "read", Some(2)).unwrap();
    let snapshot = build_snapshot("inv_root", vec![read_binding], 4);
    let turn = ToolTurnContract::all(&snapshot, ToolChoice::None).expect("turn");

    let mut gate = ToolRequestGate::default();
    let invocation = ToolInvocation {
        call_id: "c1".to_string(),
        tool_id: read_id,
        arguments: json!({}),
        provider_metadata: Value::Null,
    };

    let err = gate
        .authorize_and_reserve(&snapshot, &turn, &invocation)
        .expect_err("ToolChoice::None must reject");
    assert!(matches!(err, ToolRequestGateError::ToolChoiceNone { .. }));
}

#[test]
fn per_tool_max_calls_budget_is_enforced() {
    let read_id = ToolId::builtin("read_file").expect("tool id");
    let read_binding = ToolBinding::new(descriptor(read_id.clone()), "read", Some(1)).unwrap();
    let snapshot = build_snapshot("inv_root", vec![read_binding], 8);
    let turn = ToolTurnContract::all(&snapshot, ToolChoice::Auto).expect("turn");

    let mut gate = ToolRequestGate::default();
    let make = |call_id: &str| ToolInvocation {
        call_id: call_id.to_string(),
        tool_id: read_id.clone(),
        arguments: json!({}),
        provider_metadata: Value::Null,
    };

    // First call goes through.
    gate.authorize_and_reserve(&snapshot, &turn, &make("c1"))
        .expect("first call allowed");
    // Second call exhausts the per-tool budget.
    let err = gate
        .authorize_and_reserve(&snapshot, &turn, &make("c2"))
        .expect_err("per-tool budget must reject");
    assert!(matches!(
        err,
        ToolRequestGateError::ToolBudgetExhausted { max_calls: 1, .. }
    ));
}

#[test]
fn invocation_wide_max_calls_budget_is_enforced() {
    let a_id = ToolId::builtin("a").expect("tool id");
    let b_id = ToolId::builtin("b").expect("tool id");
    let a_binding = ToolBinding::new(descriptor(a_id.clone()), "a", None).unwrap();
    let b_binding = ToolBinding::new(descriptor(b_id.clone()), "b", None).unwrap();
    let snapshot = build_snapshot("inv_root", vec![a_binding, b_binding], 1);
    let turn = ToolTurnContract::all(&snapshot, ToolChoice::Auto).expect("turn");

    let mut gate = ToolRequestGate::default();
    let inv_a = ToolInvocation {
        call_id: "c1".to_string(),
        tool_id: a_id,
        arguments: json!({}),
        provider_metadata: Value::Null,
    };
    let inv_b = ToolInvocation {
        call_id: "c2".to_string(),
        tool_id: b_id,
        arguments: json!({}),
        provider_metadata: Value::Null,
    };

    gate.authorize_and_reserve(&snapshot, &turn, &inv_a)
        .expect("first call allowed");
    let err = gate
        .authorize_and_reserve(&snapshot, &turn, &inv_b)
        .expect_err("invocation budget must reject");
    assert!(matches!(
        err,
        ToolRequestGateError::InvocationBudgetExhausted { max_calls: 1 }
    ));
}

#[test]
fn tool_choice_specific_rejects_other_tools() {
    let a_id = ToolId::builtin("a").expect("tool id");
    let b_id = ToolId::builtin("b").expect("tool id");
    let a_binding = ToolBinding::new(descriptor(a_id.clone()), "a", None).unwrap();
    let b_binding = ToolBinding::new(descriptor(b_id.clone()), "b", None).unwrap();
    let snapshot = build_snapshot("inv_root", vec![a_binding, b_binding], 4);
    let turn = ToolTurnContract::all(&snapshot, ToolChoice::Specific(a_id.clone())).expect("turn");

    let mut gate = ToolRequestGate::default();
    let inv_b = ToolInvocation {
        call_id: "c1".to_string(),
        tool_id: b_id,
        arguments: json!({}),
        provider_metadata: Value::Null,
    };

    let err = gate
        .authorize_and_reserve(&snapshot, &turn, &inv_b)
        .expect_err("specific tool choice must reject other tools");
    assert!(matches!(
        err,
        ToolRequestGateError::ToolChoiceSpecific { .. }
    ));

    // The matching tool itself is accepted.
    let inv_a = ToolInvocation {
        call_id: "c2".to_string(),
        tool_id: a_id,
        arguments: json!({}),
        provider_metadata: Value::Null,
    };
    gate.authorize_and_reserve(&snapshot, &turn, &inv_a)
        .expect("matching tool accepted");
}

#[test]
fn tool_requested_outside_snapshot_is_rejected() {
    let read_id = ToolId::builtin("read_file").expect("tool id");
    let write_id = ToolId::builtin("write_file").expect("tool id");
    let read_binding = ToolBinding::new(descriptor(read_id.clone()), "read", Some(2)).unwrap();
    let snapshot = build_snapshot("inv_root", vec![read_binding], 4);
    let turn = ToolTurnContract::all(&snapshot, ToolChoice::Auto).expect("turn");

    let mut gate = ToolRequestGate::default();
    let inv = ToolInvocation {
        call_id: "c1".to_string(),
        tool_id: write_id,
        arguments: json!({}),
        provider_metadata: Value::Null,
    };

    let err = gate
        .authorize_and_reserve(&snapshot, &turn, &inv)
        .expect_err("unknown tool must be rejected");
    assert!(matches!(err, ToolRequestGateError::ToolNotInSnapshot { .. }));
}

#[test]
fn snapshot_rejects_zero_budget_and_duplicate_aliases() {
    let read_id = ToolId::builtin("read").expect("tool id");
    let binding = ToolBinding::new(descriptor(read_id.clone()), "read", None).unwrap();

    let zero_budget = InvocationToolSnapshot::try_new(
        ToolSnapshotId::parse("inv").unwrap(),
        vec![binding.clone()],
        0,
    );
    assert!(matches!(zero_budget, Err(ToolError::InvalidData(_))));

    let dup = InvocationToolSnapshot::try_new(
        ToolSnapshotId::parse("inv").unwrap(),
        vec![
            binding.clone(),
            ToolBinding::new(descriptor(ToolId::builtin("other").unwrap()), "read", None).unwrap(),
        ],
        4,
    );
    assert!(matches!(dup, Err(ToolError::Conflict(_))));
}