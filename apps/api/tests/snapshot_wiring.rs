//! Wave 2 装配接线测试 —— 快照加载层的三个纯函数语义，不依赖 PG。
//!
//! 覆盖三条本 PR 新接的线：
//! 1. **多 key 渠道轮换**：`expand_models_json` 按 `(group × model × key_index)` 展开
//!    路由单元——2 keys × 2 models × 1 group = 4 条 unit（过去恒 `key_index = 0`
//!    只有 2 条），unit_key 带 key_index 后缀保持唯一，dispatch 健康状态机
//!    据此按 key 粒度熔断/轮换；
//! 2. **token 级白名单**：`parse_token_allowed_models` 把 `api_tokens.allowed_models`
//!    JSONB 译成 gate 的 `Option<Vec<String>>`（`[]`/NULL → None = 不限制，
//!    非空 → Some），并确认其进入 `TokenEntry::new` 后被 gate 链路可见；
//! 3. **组级白名单**：`build_group_snapshot` 把 `api_groups` 行拼成 `GroupSnapshot`，
//!    `GroupModelGate::check` 对 vip+命中通配放行 / vip+非白名单拒绝 /
//!    未配置组 fail-open。
//!
//! PG 读取链路本身由 e2e（`tests-e2e` 的 reload 用例）覆盖，这里只锁纯函数语义，
//! 与 `reload_counts.rs` 的离线测试策略一致。

use std::net::IpAddr;
use std::sync::Arc;

use api::snapshot::{build_group_snapshot, expand_models_json, parse_token_allowed_models};
use arc_swap::ArcSwap;
use bytes::Bytes;
use gateway_gate::chain::GateCtx;
use gateway_gate::snapshot::TokenEntry;
use gateway_gate::{Gate, GroupModelGate, Rejection};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. 多 key 渠道轮换：expand_models_json
// ---------------------------------------------------------------------------

/// 2 keys × 2 models × 1 group → 4 条 unit；unit_key 全唯一且带 key_index 后缀；
/// key_index 在 0/1 上正确轮换（每模型两把 key 各一）。
#[test]
fn expand_models_json_expands_one_unit_per_key() {
    let models = json!(["gpt-4o", "gpt-4o-mini"]);
    let groups = vec!["default".to_string()];

    let units = expand_models_json(&models, "ch-1", &groups, 2, 10, 5);

    assert_eq!(units.len(), 4, "2 keys × 2 models 应展成 4 条路由单元");

    // unit_key 全不重复，且以 key_index 结尾
    let keys: Vec<&str> = units.iter().map(|u| u.meta.key.as_str()).collect();
    let mut dedup = keys.clone();
    dedup.sort_unstable();
    dedup.dedup();
    assert_eq!(
        dedup.len(),
        4,
        "unit_key 必须唯一（含 key_index 后缀）: {keys:?}"
    );

    // 按 (model, key_index) 成对出现：每模型 key 0 与 key 1 各一条
    for model in ["gpt-4o", "gpt-4o-mini"] {
        for key_index in 0..2u32 {
            let unit = units
                .iter()
                .find(|u| u.public_model == model && u.key_index == key_index)
                .unwrap_or_else(|| panic!("缺 ({model}, key_index={key_index}) 的 unit"));
            assert_eq!(
                unit.meta.key,
                format!("ch-1:default:{model}:{key_index}"),
                "unit_key 格式须为 channel:group:model:key_index"
            );
            assert_eq!(unit.group, "default");
            assert_eq!(unit.upstream_model, model, "字符串模型 → upstream = public");
            assert_eq!(unit.priority, 10);
            assert_eq!(unit.weight, 5);
            assert_eq!(unit.status, 1);
        }
    }

    // 遍历顺序：model 外层 → group → key_index 内层（0 在前 1 在后）
    let order: Vec<(String, u32)> = units
        .iter()
        .map(|u| (u.public_model.clone(), u.key_index))
        .collect();
    assert_eq!(
        order,
        vec![
            ("gpt-4o".to_string(), 0),
            ("gpt-4o".to_string(), 1),
            ("gpt-4o-mini".to_string(), 0),
            ("gpt-4o-mini".to_string(), 1),
        ],
        "展开顺序应让同模型的 key 相邻且 key_index 递增"
    );
}

/// 渠道无 key（key_count == 0）→ 不产生 unit：候选解析取不到凭据，
/// 展开也是永远 NoCandidate 的死单元。
#[test]
fn expand_models_json_zero_keys_yields_no_units() {
    let models = json!(["gpt-4o"]);
    let groups = vec!["default".to_string()];
    let units = expand_models_json(&models, "ch-1", &groups, 0, 1, 1);
    assert!(units.is_empty(), "无 key 渠道不应产生路由单元");
}

/// 单 key 渠道保持旧行为等价：每 (group, model) 恰一条 key_index=0 的 unit。
#[test]
fn expand_models_json_single_key_matches_legacy_shape() {
    let models = json!([{"alias": "gpt-4o", "upstream": "gpt-4o-2024-08-06"}]);
    let groups = vec!["default".to_string(), "vip".to_string()];
    let units = expand_models_json(&models, "ch-9", &groups, 1, 0, 1);
    assert_eq!(units.len(), 2, "1 key × 1 model × 2 groups");
    for u in &units {
        assert_eq!(u.key_index, 0);
        assert!(
            u.meta.key.ends_with(":0"),
            "单 key 时 unit_key 后缀为 :0: {}",
            u.meta.key
        );
    }
}

// ---------------------------------------------------------------------------
// 2. token 级白名单：parse_token_allowed_models
// ---------------------------------------------------------------------------

/// `["gpt-4o"]` → Some(["gpt-4o"])；`[]` 与 NULL → None（零限制）。
/// 再经 TokenEntry::new 进快照——这是 gate（ModelGate）实际读到的形状。
#[test]
fn token_allowed_models_json_to_gate_option() {
    // 非空数组 → Some，进入 TokenEntry 后 gate 可见
    let parsed = parse_token_allowed_models(&json!(["gpt-4o"]));
    assert_eq!(parsed, Some(vec!["gpt-4o".to_string()]));

    // 空数组 → None（不限制），NULL → None，非法形状（对象/字符串）→ None
    assert_eq!(parse_token_allowed_models(&json!([])), None);
    assert_eq!(parse_token_allowed_models(&serde_json::Value::Null), None);
    assert_eq!(parse_token_allowed_models(&json!({"a": 1})), None);
    // 含非字符串元素的脏数组：合法元素保留，其余丢弃
    assert_eq!(
        parse_token_allowed_models(&json!(["gpt-4*", 42, null])),
        Some(vec!["gpt-4*".to_string()])
    );

    // 与真实 TokenEntry 接线同形状：allowed_models 作为第二参数进入快照条目
    let record = contract::records::TokenRecord {
        meta: contract::records::SyncMeta {
            key: "tok-1".into(),
            schema_version: contract::SCHEMA_VERSION,
            logical_version: 1,
            origin: "test".into(),
            updated_at: chrono::Utc::now(),
        },
        user_key: "u-1".into(),
        name: "t".into(),
        key_hash: "00".repeat(32),
        key_preview: "sk-****".into(),
        group: None,
        quota: 100,
        unlimited_quota: false,
        used_quota: 0,
        expires_at: None,
        status: 1,
    };
    let entry = TokenEntry::new(
        record.clone(),
        parse_token_allowed_models(&json!(["gpt-4o"])),
    );
    assert_eq!(entry.allowed_models, Some(vec!["gpt-4o".to_string()]));
    let entry_open = TokenEntry::new(record, parse_token_allowed_models(&json!([])));
    assert_eq!(entry_open.allowed_models, None, "空数组应译为零限制");
}

// ---------------------------------------------------------------------------
// 3. 组级白名单：build_group_snapshot + GroupModelGate
// ---------------------------------------------------------------------------

/// 最小 GateCtx：只关心 group / requested_model 两个输入字段（同 gate crate
/// 自身测试的构造方式；body 为空 JSON，GroupModelGate 不解析 body）。
fn gate_ctx(group: Option<&str>, model: Option<&str>) -> GateCtx {
    let client_ip: IpAddr = "10.0.0.5".parse().unwrap();
    GateCtx {
        request_meta: RequestMeta {
            method: "POST".into(),
            path: "/v1/chat/completions".into(),
            headers: axum::http::HeaderMap::new(),
            body: BodySource::InMemory(Bytes::from_static(b"{}")),
            client_ip,
            request_id: uuid::Uuid::now_v7(),
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

/// api_groups 行 → GroupSnapshot：白名单与倍率都落进快照。
#[test]
fn build_group_snapshot_maps_whitelist_and_ratio() {
    let snapshot = build_group_snapshot(&[
        ("vip".into(), 0.8, json!(["gpt-4*"])),
        ("free".into(), 1.5, json!([])),
    ]);

    assert_eq!(
        snapshot.allowed_models("vip"),
        Some(["gpt-4*".to_string()].as_slice()),
        "vip 白名单应取自 model_whitelist JSONB"
    );
    assert!(
        (snapshot.multiplier("vip") - 0.8).abs() < f64::EPSILON,
        "vip 倍率应取自 ratio 列"
    );
    // 空白名单组：存在但 allowed_models 为空（语义 = 该组不限模型）
    assert_eq!(
        snapshot.allowed_models("free"),
        Some(Vec::<String>::new().as_slice())
    );
    assert!((snapshot.multiplier("free") - 1.5).abs() < f64::EPSILON);
    // 未配置组 → None / 中性 1.0（gate 据此 fail-open）
    assert_eq!(snapshot.allowed_models("ghost"), None);
    assert!((snapshot.multiplier("ghost") - 1.0).abs() < f64::EPSILON);
}

/// GroupModelGate 接线语义：vip+命中通配 → 放行；vip+白名单外 →
/// ModelNotAllowedForGroup；未配置组 → fail-open 放行。
#[tokio::test]
async fn group_model_gate_enforces_snapshot() {
    let snapshot = build_group_snapshot(&[("vip".into(), 0.8, json!(["gpt-4*"]))]);
    let gate = GroupModelGate::new(Arc::new(ArcSwap::from_pointee(snapshot)));

    // vip + gpt-4o：`gpt-4*` 通配命中 → 放行
    let mut c = gate_ctx(Some("vip"), Some("gpt-4o"));
    gate.check(&mut c).await.expect("vip 通配命中应放行");

    // vip + claude-3.5：白名单存在但不命中 → 拒绝并带 model+group
    let mut c = gate_ctx(Some("vip"), Some("claude-3.5"));
    let err = gate.check(&mut c).await.expect_err("白名单外应拒绝");
    assert!(
        matches!(
            err,
            Rejection::ModelNotAllowedForGroup { ref model, ref group }
                if model == "claude-3.5" && group == "vip"
        ),
        "拒绝变体应为 ModelNotAllowedForGroup(vip, claude-3.5)，实得 {err:?}"
    );

    // 未配置组（不在快照）→ fail-open：任何模型都放行
    let mut c = gate_ctx(Some("ghost-group"), Some("claude-3.5"));
    gate.check(&mut c).await.expect("未配置组应放行");
    // 空组名同样 fail-open
    let mut c = gate_ctx(Some(""), Some("claude-3.5"));
    gate.check(&mut c).await.expect("空组名应放行");
}
