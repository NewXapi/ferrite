//! reload 热更纯逻辑离线单测 —— 不依赖 PG。
//!
//! 测什么：[`api::snapshot::apply_snapshot_reload`] 把加载好的纯值快照 store 进
//! `Shared*` 与 `Dispatcher` 之后：
//! 1. 返回计数与输入规模一致（channels / route_units / tokens / users）；
//! 2. 四个运行时组件真的看到新数据（token 可鉴权、user 可查、quota 口径与
//!    gate 一致、dispatcher 能从新路由单元选出候选）；
//! 3. reload 是**整表替换**而非合并 —— 二次 reload 后旧 token 必须消失。
//!
//! 为什么能离线跑：`apply_snapshot_reload` 是不碰 PG 的纯函数，store 逻辑本身
//! 无外部依赖；boot/reload 的 PG 读取链路由 `tests-e2e` 的
//! `e2e_reload_picks_up_new_token_without_restart` 端到端覆盖。

use std::sync::Arc;

use api::billing::NameDirectory;
use api::snapshot::{ReloadInput, Snapshots, apply_snapshot_reload};
use arc_swap::ArcSwap;
use chrono::Utc;
use contract::records::{
    ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta, TokenRecord, UserRecord,
};
use dispatch::{Dispatch, Dispatcher, MemoryHealthTable, Snapshot as DispatchSnapshot};
use gateway_gate::snapshot::{
    GroupSnapshot, QuotaSnapshot, TokenEntry, TokenSnapshot, UserSnapshot,
};

/// 构造 SyncMeta（schema_version 用真实常量，其余字段对本测试无语义）。
fn sync_meta(key: &str) -> SyncMeta {
    SyncMeta {
        key: key.to_string(),
        schema_version: contract::SCHEMA_VERSION,
        logical_version: 1,
        origin: "test".into(),
        updated_at: Utc::now(),
    }
}

/// 构造一个 status=1、单 key、default 分组的渠道（dispatch 只选启用渠道）。
fn channel(key: &str) -> ChannelRecord {
    ChannelRecord {
        meta: sync_meta(key),
        name: format!("ch-{key}"),
        provider_type: "openai".into(),
        base_url: "http://upstream".into(),
        keys: vec![ChannelKey {
            index: 0,
            secret: "sk-upstream".into(),
            rpm_limit: 0,
        }],
        max_concurrency: 8,
        status: 1,
        groups: vec!["default".into()],
        settings: serde_json::Value::Null,
    }
}

/// 构造一个 status=1 的路由单元（status!=1 会被 dispatch 过滤，必须为 1）。
fn route_unit(channel_key: &str, model: &str) -> RouteUnitRecord {
    RouteUnitRecord {
        meta: sync_meta(&format!("{channel_key}:default:{model}")),
        group: "default".into(),
        public_model: model.into(),
        channel_key: channel_key.into(),
        key_index: 0,
        upstream_model: format!("{model}-upstream"),
        priority: 1,
        weight: 1,
        status: 1,
    }
}

/// 构造一个 status=1 的 token 记录（quota/used_quota 用于验证 quota 快照口径）。
fn token_record(key: &str, quota: i64, used: i64) -> TokenRecord {
    TokenRecord {
        meta: sync_meta(key),
        user_key: "u-1".into(),
        name: "tk".into(),
        key_hash: "00".repeat(32), // apply 路径不解码 hash，保持形状即可
        key_preview: "sk-****".into(),
        group: None,
        quota,
        unlimited_quota: false,
        used_quota: used,
        expires_at: None,
        status: 1,
    }
}

/// 构造一个 status=1 的用户记录。
fn user_record(key: &str, name: &str) -> UserRecord {
    UserRecord {
        meta: sync_meta(key),
        username: name.into(),
        display_name: name.into(),
        email: String::new(),
        quota: 100,
        used_quota: 0,
        request_count: 0,
        group: "default".into(),
        status: 1,
        role: 1,
        created_at: Utc::now(),
    }
}

/// 构造空的目标运行时（等价 boot 前状态）+ 空快照 Dispatcher。
fn empty_target() -> (Snapshots, Arc<Dispatcher>) {
    let target = Snapshots {
        dispatch: DispatchSnapshot::default(),
        token_snapshot: Arc::new(ArcSwap::from_pointee(TokenSnapshot::default())),
        user_snapshot: Arc::new(ArcSwap::from_pointee(UserSnapshot::default())),
        quota_snapshot: Arc::new(ArcSwap::from_pointee(QuotaSnapshot::default())),
        group_snapshot: Arc::new(ArcSwap::from_pointee(GroupSnapshot::default())),
        // 计费快照：boot 等价的空价表 / 空名单，reload 必须能 store 换新
        price_rows: Arc::new(ArcSwap::from_pointee(Vec::new())),
        name_directory: Arc::new(ArcSwap::from_pointee(NameDirectory::default())),
    };
    // boot 时 Dispatcher 可能拿 None 快照（SnapshotNotReady）；reload 必须让它就绪
    let dispatcher = Arc::new(Dispatcher::new(None, Arc::new(MemoryHealthTable::new())));
    (target, dispatcher)
}

/// reload 后计数正确，且四个运行时组件都看到新数据。
#[test]
fn apply_snapshot_reload_stores_new_values_and_counts() {
    let (target, dispatcher) = empty_target();

    // 输入规模：1 渠道 × 2 模型 = 2 路由单元；2 token；2 用户
    let channels = vec![channel("ch-1")];
    let units = vec![
        route_unit("ch-1", "gpt-4o"),
        route_unit("ch-1", "gpt-4o-mini"),
    ];
    let tokens = vec![
        token_record("1001", 1000, 100), // quota 口径：1000 - 100 = 900
        token_record("1002", 500, 0),
    ];
    let users = vec![user_record("u-1", "alice"), user_record("u-2", "bob")];

    let token_snapshot = TokenSnapshot::default();
    // 与真实链路同构：sha256 索引；这里用固定 hash 数组代替（apply 不解码）
    let hash_a: [u8; 32] = [7; 32];
    let hash_b: [u8; 32] = [8; 32];
    token_snapshot.upsert(hash_a, TokenEntry::new(tokens[0].clone(), None));
    token_snapshot.upsert(hash_b, TokenEntry::new(tokens[1].clone(), None));

    let user_snapshot = UserSnapshot::default();
    for u in &users {
        user_snapshot.upsert(u.clone());
    }

    // 组快照：1 个 vip 组 —— reload 后 group_snapshot 应可见且计数上报
    let group_snapshot =
        api::snapshot::build_group_snapshot(&[("vip".into(), 0.8, serde_json::json!(["gpt-4*"]))]);

    let counts = apply_snapshot_reload(
        &target,
        &dispatcher,
        ReloadInput {
            channels,
            route_units: units,
            token_records: tokens.clone(),
            token_snapshot,
            user_records: users.clone(),
            user_snapshot,
            group_snapshot,
            group_count: 1,
            // 计费快照输入：与真实链路同构（价格一行、名单由 token/user 记录构建）
            price_rows: vec![("gpt-4o".into(), 15.0, 60.0, 0.0)],
            name_directory: NameDirectory::new(&tokens, &users),
        },
    );

    // 1. 计数与输入规模一致
    assert_eq!(counts.channels, 1, "channels 计数");
    assert_eq!(counts.route_units, 2, "route_units 计数");
    assert_eq!(counts.tokens, 2, "tokens 计数");
    assert_eq!(counts.users, 2, "users 计数");
    assert_eq!(counts.groups, 1, "groups 计数");

    // 2. store 生效：token/user/group 可查；quota 口径与 QuotaGate 一致（meta.key 直接作 key）
    assert!(
        target.token_snapshot.load().lookup(&hash_a).is_some(),
        "新 token 快照应可按 hash 查到"
    );
    assert!(
        target.user_snapshot.load().lookup("u-2").is_some(),
        "新 user 快照应可按 key 查到"
    );
    assert_eq!(
        target
            .group_snapshot
            .load()
            .allowed_models("vip")
            .map(<[String]>::len),
        Some(1),
        "新 group 快照应带 vip 白名单"
    );
    assert_eq!(
        target.quota_snapshot.load().remaining("1001"),
        900,
        "quota 应等于 quota - used_quota"
    );

    // 2b. 计费快照换新生效：价格行与展示名目录 store 后立即可读（价格表
    //     ArcSwap 化前的 Suspect 只涉及 ForwardStage 手里的 clone，不涉及此处）
    assert_eq!(
        target.price_rows.load().len(),
        1,
        "reload 后价格行应整表替换"
    );
    assert_eq!(
        target
            .price_rows
            .load()
            .first()
            .map(|(m, _, _, _)| m.as_str()),
        Some("gpt-4o")
    );
    assert_eq!(
        target.name_directory.load().username("u-1"),
        "alice",
        "reload 后展示名目录应可查"
    );
    assert_eq!(target.name_directory.load().token_name("1002"), "tk");

    // 3. Dispatcher 换上新快照：boot 时是 None（SnapshotNotReady），reload 后能选中
    let cand = dispatcher
        .select("default", "gpt-4o-mini", &[])
        .expect("reload 后 dispatcher 应能选中新路由单元");
    assert_eq!(cand.upstream_model, "gpt-4o-mini-upstream");
    assert_eq!(cand.secret, "sk-upstream", "凭据应取自渠道 keys[key_index]");
}

/// reload 是整表替换不是合并：二次 reload 后旧 token 消失、计数取新值。
/// 这是「直接 store 新建快照、不改旧快照」语义的直接验证。
#[test]
fn reload_replaces_snapshot_wholesale() {
    let (target, dispatcher) = empty_target();

    // 第一次 reload：token A + 1 渠道 1 单元
    let tokens_a = vec![token_record("1001", 1000, 0)];
    let token_snapshot_a = TokenSnapshot::default();
    let hash_a: [u8; 32] = [7; 32];
    token_snapshot_a.upsert(hash_a, TokenEntry::new(tokens_a[0].clone(), None));
    let counts_a = apply_snapshot_reload(
        &target,
        &dispatcher,
        ReloadInput {
            channels: vec![channel("ch-1")],
            route_units: vec![route_unit("ch-1", "gpt-4o")],
            token_records: tokens_a,
            token_snapshot: token_snapshot_a,
            user_records: vec![],
            user_snapshot: UserSnapshot::default(),
            group_snapshot: GroupSnapshot::default(),
            group_count: 0,
            price_rows: Vec::new(),
            name_directory: NameDirectory::default(),
        },
    );
    assert_eq!(counts_a.tokens, 1);
    assert!(target.token_snapshot.load().lookup(&hash_a).is_some());

    // 第二次 reload：只带 token B —— A 必须消失（整表替换），单元数取新值
    let tokens_b = vec![token_record("1002", 1000, 0)];
    let token_snapshot_b = TokenSnapshot::default();
    let hash_b: [u8; 32] = [9; 32];
    token_snapshot_b.upsert(hash_b, TokenEntry::new(tokens_b[0].clone(), None));
    let counts_b = apply_snapshot_reload(
        &target,
        &dispatcher,
        ReloadInput {
            channels: vec![channel("ch-2"), channel("ch-3")],
            route_units: vec![
                route_unit("ch-2", "gpt-4o"),
                route_unit("ch-3", "gpt-4o"),
                route_unit("ch-3", "gpt-4o-mini"),
            ],
            token_records: tokens_b,
            token_snapshot: token_snapshot_b,
            user_records: vec![],
            user_snapshot: UserSnapshot::default(),
            group_snapshot: GroupSnapshot::default(),
            group_count: 0,
            price_rows: Vec::new(),
            name_directory: NameDirectory::default(),
        },
    );
    assert_eq!(counts_b.tokens, 1, "tokens 计数应是新输入规模");
    assert!(
        target.token_snapshot.load().lookup(&hash_a).is_none(),
        "被删除的旧 token 不应残留在快照里"
    );
    assert!(target.token_snapshot.load().lookup(&hash_b).is_some());
    assert_eq!(counts_b.channels, 2, "channels 计数应是新输入规模");
    assert_eq!(counts_b.route_units, 3, "route_units 计数应是新输入规模");
}
