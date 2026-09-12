//! `gateway-gate` 组级模型门禁集成测试 —— GroupSnapshot 查询 + GroupModelGate 判定
//!
//! 覆盖：组白名单命中/通配命中、不命中拒绝、未配置 fail-open、缺 model、
//! 快照查询回落，以及新 Rejection 变体的 HTTP 映射（403 + code）。

use std::net::IpAddr;
use std::sync::Arc;

use arc_swap::ArcSwap;
use bytes::Bytes;
use gateway_gate::chain::GateCtx;
use gateway_gate::snapshot::{GroupEntry, GroupSnapshot};
use gateway_gate::{Gate, GroupModelGate, Rejection, SharedGroupSnapshot, rejection_to_response};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta};
use http::HeaderMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// 构造测试快照："vip" 只允许 `gpt-4*`（倍率 2.5），"public" 白名单为空（不限模型）。
fn group_snapshot() -> SharedGroupSnapshot {
    let mut groups = GroupSnapshot::default();
    groups.upsert(
        "vip".into(),
        GroupEntry {
            allowed_models: vec!["gpt-4*".into()],
            multiplier: 2.5,
        },
    );
    groups.upsert(
        "public".into(),
        GroupEntry {
            allowed_models: vec![],
            multiplier: 1.0,
        },
    );
    Arc::new(ArcSwap::from_pointee(groups))
}

/// 最小 GateCtx：只关心 group / requested_model 两个输入字段。
fn ctx(group: Option<&str>, model: Option<&str>) -> GateCtx {
    let client_ip: IpAddr = "10.0.0.5".parse().unwrap();
    GateCtx {
        request_meta: RequestMeta {
            method: "POST".into(),
            path: "/v1/chat/completions".into(),
            headers: HeaderMap::new(),
            body: BodySource::InMemory(Bytes::from_static(b"{}")),
            client_ip,
            request_id: Uuid::now_v7(),
            inbound_protocol: ProtocolKind::OpenAI,
        },
        raw_key: None,
        user_key: None,
        token: None,
        user: None,
        group: group.map(str::to_string),
        estimated_cost: None,
        requested_model: model.map(str::to_string),
        requested_max_tokens: None,
    }
}

// ---------------------------------------------------------------------------
// GroupModelGate::check
// ---------------------------------------------------------------------------

#[tokio::test]
async fn vip_group_allows_wildcard_match() {
    let gate = GroupModelGate::new(group_snapshot());
    // `gpt-4*` 通配命中 gpt-4o → 放行
    let mut c = ctx(Some("vip"), Some("gpt-4o"));
    gate.check(&mut c)
        .await
        .expect("gpt-4o allowed via wildcard");
}

#[tokio::test]
async fn vip_group_rejects_non_matching_model() {
    let gate = GroupModelGate::new(group_snapshot());
    let mut c = ctx(Some("vip"), Some("claude-x"));
    let r = gate.check(&mut c).await.unwrap_err();
    // 拒绝体应带上具体 model + group，便于排障
    assert!(
        matches!(
            r,
            Rejection::ModelNotAllowedForGroup { ref model, ref group }
                if model == "claude-x" && group == "vip"
        ),
        "unexpected rejection: {r:?}"
    );
}

#[tokio::test]
async fn empty_whitelist_group_fails_open() {
    let gate = GroupModelGate::new(group_snapshot());
    // "public" 白名单为空 = 该组不限模型
    let mut c = ctx(Some("public"), Some("anything-at-all"));
    gate.check(&mut c)
        .await
        .expect("empty whitelist = unrestricted");
}

#[tokio::test]
async fn unknown_group_and_missing_group_fail_open() {
    let gate = GroupModelGate::new(group_snapshot());
    // 组不存在（未配置）→ fail-open
    let mut c = ctx(Some("no-such-group"), Some("claude-x"));
    gate.check(&mut c).await.expect("unknown group fail-open");
    // ctx 无 group → 同样 fail-open
    let mut c = ctx(None, Some("claude-x"));
    gate.check(&mut c).await.expect("no group fail-open");
    // 空字符串 group 视作未配置（对齐 chain.rs 的 group 提升语义）
    let mut c = ctx(Some(""), Some("claude-x"));
    gate.check(&mut c).await.expect("empty group fail-open");
}

#[tokio::test]
async fn missing_requested_model_is_model_not_specified() {
    let gate = GroupModelGate::new(group_snapshot());
    let mut c = ctx(Some("vip"), None);
    let r = gate.check(&mut c).await.unwrap_err();
    assert!(matches!(r, Rejection::ModelNotSpecified), "got {r:?}");
}

// ---------------------------------------------------------------------------
// GroupSnapshot 查询
// ---------------------------------------------------------------------------

#[test]
fn snapshot_queries_and_fallbacks() {
    let mut groups = GroupSnapshot::default();
    groups.upsert(
        "vip".into(),
        GroupEntry {
            allowed_models: vec!["gpt-4*".into()],
            multiplier: 2.5,
        },
    );

    // 命中：白名单原样返回（零拷贝借用）
    assert_eq!(
        groups.allowed_models("vip").map(Vec::as_slice),
        Some(&["gpt-4*".to_string()][..])
    );
    assert_eq!(groups.multiplier("vip"), 2.5);
    // 缺失回落：None / 1.0（中性倍率）
    assert!(groups.allowed_models("missing").is_none());
    assert_eq!(groups.multiplier("missing"), 1.0);

    // upsert 覆盖同名组
    groups.upsert("vip".into(), GroupEntry::default());
    assert_eq!(groups.allowed_models("vip").map(Vec::len), Some(0));
    assert_eq!(groups.multiplier("vip"), 1.0);
    // 确认默认值本身
    let d = GroupEntry::default();
    assert!(d.allowed_models.is_empty());
    assert_eq!(d.multiplier, 1.0);
}

// ---------------------------------------------------------------------------
// Rejection → HTTP 映射
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rejection_maps_to_403_with_code() {
    let resp = rejection_to_response(Rejection::ModelNotAllowedForGroup {
        model: "claude-x".into(),
        group: "vip".into(),
    });
    assert_eq!(resp.status(), 403);
    let bytes = axum::body::to_bytes(resp.into_body(), 4096)
        .await
        .expect("read body");
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("json body");
    assert_eq!(v["error"]["code"], "model_not_allowed_for_group");
    // message 里应同时带 model 与 group，供客户端排障
    let msg = v["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("claude-x") && msg.contains("vip"),
        "msg = {msg}"
    );
}
