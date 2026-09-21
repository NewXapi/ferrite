//! `gateway-gate` 集成测试 —— 七道闸 + chain 顺序

use std::net::IpAddr;
use std::sync::Arc;

use arc_swap::ArcSwap;
use bytes::Bytes;
use chrono::Utc;
use contract::records::{SyncMeta, TokenRecord, UserRecord};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta};
use gateway_pipeline::{RequestCtx, SelectedRoute, Stage, StageError, StageOutcome};
use http::HeaderMap;
use uuid::Uuid;

use gateway_gate::chain::GateCtx;
use gateway_gate::snapshot::adapt::{TokenView, now_unix};
use gateway_gate::snapshot::{
    IpPolicy, PriceRow, PricingSnapshot, QuotaSnapshot, TokenEntry, TokenSnapshot, UserSnapshot,
};
use gateway_gate::{
    AuthGate, ConcurrencyGate, ConcurrencyState, Gate, GateChain, GrayListGate, GrayListState,
    ModelGate, QuotaGate, RateLimitGate, RateLimiter, Rejection, StateGate, graylist, sha256,
};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn meta(key: &str) -> SyncMeta {
    SyncMeta {
        key: key.to_string(),
        schema_version: 1,
        logical_version: 1,
        origin: "test".into(),
        updated_at: Utc::now(),
    }
}

fn user_record(key: &str, group: &str, enabled: bool) -> UserRecord {
    UserRecord {
        meta: meta(key),
        username: format!("u-{key}"),
        display_name: format!("User {key}"),
        email: format!("{key}@test.io"),
        quota: 100_000_000,
        used_quota: 0,
        request_count: 0,
        group: group.to_string(),
        status: if enabled { 1 } else { 2 },
        role: 1,
        created_at: Utc::now(),
    }
}

fn token_record(key: &str, user_key: &str, group: Option<&str>) -> TokenRecord {
    TokenRecord {
        meta: meta(key),
        user_key: user_key.to_string(),
        name: format!("tok-{key}"),
        key_hash: "abcd".into(),
        key_preview: "sk-ab****cd".into(),
        group: group.map(String::from),
        quota: 10_000_000,
        unlimited_quota: false,
        used_quota: 0,
        expires_at: None,
        status: 1,
    }
}

fn make_meta(headers: HeaderMap, body: Vec<u8>) -> RequestMeta {
    let client_ip: IpAddr = "10.0.0.5".parse().unwrap();
    RequestMeta {
        method: "POST".into(),
        path: "/v1/chat/completions".into(),
        headers,
        body: BodySource::InMemory(Bytes::from(body)),
        client_ip,
        request_id: Uuid::now_v7(),
        inbound_protocol: ProtocolKind::OpenAI,
    }
}

fn make_ctx(meta: RequestMeta) -> GateCtx {
    GateCtx {
        request_meta: meta,
        raw_key: None,
        user_key: None,
        token: None,
        user: None,
        group: None,
        estimated_cost: None,
        requested_model: None,
        requested_max_tokens: None,
    }
}

fn bearer_header(key: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("authorization", format!("Bearer {key}").parse().unwrap());
    h
}

fn ctx_with_token(raw: &str, user_key: &str, snapshot: &TokenSnapshot) -> GateCtx {
    let hash = sha256(raw);
    let entry = snapshot.lookup(&hash).expect("token present in snapshot");
    let token = entry.record;
    let mut ctx = make_ctx(make_meta(bearer_header(raw), b"{}".to_vec()));
    ctx.raw_key = Some(raw.into());
    ctx.user_key = Some(user_key.into());
    ctx.token = Some(gateway_gate::TokenInfo {
        id: token.meta.key.clone(),
        user_key: "user-test".to_string(),
        user_id: 0,
        id_hash: hash,
        group: token.group().unwrap_or("").to_string(),
        enabled: token.is_enabled(),
        expires_at: token.expires_at_unix(),
        allowed_models: None,
        auth_version: token.auth_version(),
    });
    ctx
}

// ---------------------------------------------------------------------------
// 1. auth — Bearer / x-api-key / x-goog-api-key + sha256 查表 + 无效 key
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auth_extracts_bearer_and_looks_up_by_sha256() {
    let raw_key = "sk-test-auth-1";
    let hash = sha256(raw_key);

    let snapshot = TokenSnapshot::default();
    snapshot.upsert(
        hash,
        TokenEntry::new(token_record("tok-1", "user-1", Some("vip")), None),
    );

    let snap = Arc::new(ArcSwap::from_pointee(snapshot));
    let gate = AuthGate::new(snap);

    let mut ctx = make_ctx(make_meta(bearer_header(raw_key), b"{}".to_vec()));
    gate.check(&mut ctx).await.expect("auth ok");

    assert!(ctx.token.is_some());
    assert_eq!(ctx.raw_key.as_deref(), Some(raw_key));
    assert_eq!(ctx.user_key.as_deref(), Some("user-1"));
}

#[tokio::test]
async fn auth_extracts_x_api_key_header() {
    let raw = "sk-xapi";
    let hash = sha256(raw);
    let snapshot = TokenSnapshot::default();
    snapshot.upsert(hash, TokenEntry::new(token_record("k1", "u1", None), None));

    let snap = Arc::new(ArcSwap::from_pointee(snapshot));
    let gate = AuthGate::new(snap);

    let mut h = HeaderMap::new();
    h.insert("x-api-key", raw.parse().unwrap());
    let mut ctx = make_ctx(make_meta(h, b"{}".to_vec()));
    gate.check(&mut ctx).await.expect("auth x-api-key ok");
    assert!(ctx.token.is_some());
}

#[tokio::test]
async fn auth_extracts_x_goog_api_key_header() {
    let raw = "goog-key-1";
    let hash = sha256(raw);
    let snapshot = TokenSnapshot::default();
    snapshot.upsert(hash, TokenEntry::new(token_record("k2", "u2", None), None));

    let snap = Arc::new(ArcSwap::from_pointee(snapshot));
    let gate = AuthGate::new(snap);

    let mut h = HeaderMap::new();
    h.insert("x-goog-api-key", raw.parse().unwrap());
    let mut ctx = make_ctx(make_meta(h, b"{}".to_vec()));
    gate.check(&mut ctx).await.expect("auth x-goog ok");
    assert!(ctx.token.is_some());
}

#[tokio::test]
async fn auth_rejects_invalid_key() {
    let snapshot = TokenSnapshot::default();
    let snap = Arc::new(ArcSwap::from_pointee(snapshot));
    let gate = AuthGate::new(snap);

    let mut ctx = make_ctx(make_meta(bearer_header("nope"), b"{}".to_vec()));
    let r = gate.check(&mut ctx).await.unwrap_err();
    assert!(matches!(r, Rejection::InvalidApiKey));
}

// ---------------------------------------------------------------------------
// 2. state — 过期 / 禁用 / IP 白名单
// ---------------------------------------------------------------------------

#[tokio::test]
async fn state_rejects_disabled_user_and_expired_token_and_blocked_ip() {
    let raw = "sk-state-1";
    let hash = sha256(raw);
    let snapshot = TokenSnapshot::default();
    snapshot.upsert(
        hash,
        TokenEntry::new(token_record("tok-state", "user-state", None), None),
    );

    let users = Arc::new(ArcSwap::from_pointee(UserSnapshot::default()));
    users
        .load()
        .upsert(user_record("user-state", "default", true));

    let policy = Arc::new(ArcSwap::from_pointee(IpPolicy::from_cidrs(&[
        "10.0.0.0/24".into(),
    ])));
    let state = StateGate::new(users.clone(), policy.clone());

    // a) disabled user → UserDisabled
    users
        .load()
        .upsert(user_record("user-state", "default", false));
    let mut ctx = ctx_with_token(raw, "user-state", &snapshot);
    let r = state.check(&mut ctx).await.unwrap_err();
    assert!(matches!(r, Rejection::UserDisabled));

    // b) token 过期 → TokenExpired
    users
        .load()
        .upsert(user_record("user-state", "default", true));
    let mut ctx = ctx_with_token(raw, "user-state", &snapshot);
    ctx.token.as_mut().unwrap().expires_at = Some(now_unix() - 10);
    let r = state.check(&mut ctx).await.unwrap_err();
    assert!(matches!(r, Rejection::TokenExpired));

    // c) IP 不在白名单
    let mut ctx = ctx_with_token(raw, "user-state", &snapshot);
    ctx.request_meta.client_ip = "8.8.8.8".parse().unwrap();
    let r = state.check(&mut ctx).await.unwrap_err();
    assert!(matches!(r, Rejection::IpNotAllowed));

    // d) happy path
    let mut ctx = ctx_with_token(raw, "user-state", &snapshot);
    state.check(&mut ctx).await.expect("state ok");
    assert_eq!(ctx.group.as_deref(), Some("default"));
}

// ---------------------------------------------------------------------------
// 3. quota — 余额不足拒绝 + pricing fallback
// ---------------------------------------------------------------------------

#[tokio::test]
async fn quota_rejects_insufficient_remaining() {
    let pricing = PricingSnapshot::default();
    pricing.upsert(
        "gpt-4o".into(),
        "default".into(),
        PriceRow {
            input_per_m: 10_000_000.0,
            output_per_m: 15_000_000.0,
            cache_per_m: 0.0,
        },
    );
    let quotas = QuotaSnapshot::default();
    quotas.upsert("tok-q".into(), 1_000_000);

    let gate = QuotaGate::new(
        Arc::new(ArcSwap::from_pointee(quotas)),
        Arc::new(ArcSwap::from_pointee(pricing)),
    );

    let mut ctx = make_ctx(make_meta(HeaderMap::new(), b"{}".to_vec()));
    ctx.token = Some(gateway_gate::TokenInfo {
        id: "tok-1".into(),
        user_key: "user-test".to_string(),
        user_id: 1,
        id_hash: [0; 32],
        group: "default".into(),
        enabled: true,
        expires_at: None,
        allowed_models: None,
        auth_version: 1,
    });
    ctx.user_key = Some("u".into());
    ctx.user = Some(gateway_gate::UserInfo {
        id: 1,
        enabled: true,
        group: "default".into(),
        auth_version: 1,
    });
    ctx.requested_model = Some("gpt-4o".into());
    ctx.requested_max_tokens = Some(1024);

    // 15_000_000 * 1024 / 1_000_000 = 15_360 -> ceil = 15_360
    let r = gate.check(&mut ctx).await.unwrap_err();
    assert!(matches!(r, Rejection::InsufficientQuota { cost, .. } if cost == 15_360));
}

#[test]
fn pricing_lookup_falls_back_to_default_group() {
    let p = PricingSnapshot::default();
    p.upsert(
        "gpt-4o".into(),
        "default".into(),
        PriceRow {
            input_per_m: 1.0,
            output_per_m: 1.0,
            cache_per_m: 1.0,
        },
    );
    assert!(
        p.lookup("gpt-4o", "vip").is_some(),
        "should fall back to default"
    );
    assert!(p.lookup("unknown-model", "default").is_none());
}

// ---------------------------------------------------------------------------
// 4. ratelimit — 滑动窗口
// ---------------------------------------------------------------------------

#[test]
fn ratelimit_sliding_window_blocks_after_limit() {
    let limiter = RateLimiter::new(2, 1); // 2 req / 1s
    assert!(limiter.try_acquire(gateway_gate::LimitScope::PerKey, "1"));
    assert!(limiter.try_acquire(gateway_gate::LimitScope::PerKey, "1"));
    assert!(
        !limiter.try_acquire(gateway_gate::LimitScope::PerKey, "1"),
        "third should fail"
    );
    assert!(limiter.try_acquire(gateway_gate::LimitScope::PerKey, "2"));
}

#[tokio::test]
async fn ratelimit_gate_rejects_with_rejection() {
    let limiter = Arc::new(RateLimiter::new(1, 60));
    let gate = RateLimitGate::new(limiter);

    let mut ctx = make_ctx(make_meta(HeaderMap::new(), b"{}".to_vec()));
    ctx.token = Some(gateway_gate::TokenInfo {
        id: "tok-1".into(),
        user_key: "user-test".to_string(),
        user_id: 1,
        id_hash: [0; 32],
        group: "g".into(),
        enabled: true,
        expires_at: None,
        allowed_models: None,
        auth_version: 1,
    });
    gate.check(&mut ctx).await.expect("first ok");
    let r = gate.check(&mut ctx).await.unwrap_err();
    assert!(matches!(r, Rejection::RateLimited));
}

// ---------------------------------------------------------------------------
// 5. model — 白名单 + 通配符
// ---------------------------------------------------------------------------

#[tokio::test]
async fn model_allows_whitelisted_and_blocks_others() {
    let gate = ModelGate;

    let allowed = Some(vec!["gpt-4*".into(), "claude-3-haiku".into()]);

    let mut ctx = make_ctx(make_meta(
        HeaderMap::new(),
        br#"{"model":"gpt-4o","max_tokens":1024}"#.to_vec(),
    ));
    ctx.token = Some(gateway_gate::TokenInfo {
        id: "tok-1".into(),
        user_key: "user-test".to_string(),
        user_id: 1,
        id_hash: [0; 32],
        group: "g".into(),
        enabled: true,
        expires_at: None,
        allowed_models: allowed.clone(),
        auth_version: 1,
    });
    gate.check(&mut ctx)
        .await
        .expect("gpt-4o allowed via wildcard");

    let mut ctx = make_ctx(make_meta(
        HeaderMap::new(),
        br#"{"model":"claude-3-haiku","max_tokens":1024}"#.to_vec(),
    ));
    ctx.token = Some(gateway_gate::TokenInfo {
        id: "tok-1".into(),
        user_key: "user-test".to_string(),
        user_id: 1,
        id_hash: [0; 32],
        group: "g".into(),
        enabled: true,
        expires_at: None,
        allowed_models: allowed.clone(),
        auth_version: 1,
    });
    gate.check(&mut ctx).await.expect("claude allowed exact");

    let mut ctx = make_ctx(make_meta(
        HeaderMap::new(),
        br#"{"model":"gemini-1.5","max_tokens":1024}"#.to_vec(),
    ));
    ctx.token = Some(gateway_gate::TokenInfo {
        id: "tok-1".into(),
        user_key: "user-test".to_string(),
        user_id: 1,
        id_hash: [0; 32],
        group: "g".into(),
        enabled: true,
        expires_at: None,
        allowed_models: allowed,
        auth_version: 1,
    });
    let r = gate.check(&mut ctx).await.unwrap_err();
    assert!(matches!(r, Rejection::ModelForbidden { .. }));
}

// ---------------------------------------------------------------------------
// 6. graylist — 连续失败封禁
// ---------------------------------------------------------------------------

#[tokio::test]
async fn graylist_blocks_after_streak_threshold() {
    let state = Arc::new(ArcSwap::from_pointee(GrayListState::default()));
    let gate = GrayListGate::new(state.clone());
    let hash = [7u8; 32];

    for _ in 0..graylist::FAIL_STREAK_THRESHOLD {
        gate.record(hash, false);
    }
    assert!(state.load().blocked_until.get(&hash).is_some());

    let mut ctx = make_ctx(make_meta(HeaderMap::new(), b"{}".to_vec()));
    ctx.token = Some(gateway_gate::TokenInfo {
        id: "tok-1".into(),
        user_key: "user-test".to_string(),
        user_id: 1,
        id_hash: hash,
        group: "g".into(),
        enabled: true,
        expires_at: None,
        allowed_models: None,
        auth_version: 1,
    });
    let r = gate.check(&mut ctx).await.unwrap_err();
    assert!(matches!(r, Rejection::Graylisted));

    gate.record(hash, true);
    assert!(state.load().blocked_until.get(&hash).is_none());
}

// ---------------------------------------------------------------------------
// 7. concurrency — RAII 槽位：ctx drop 归还 / 槽满 RateLimited / 未注册放行
// ---------------------------------------------------------------------------

/// 构造一个带 SelectedRoute 的 RequestCtx（concurrency 是 post-dispatch gate，
/// handle 需要 ctx.route 才能知道占哪个渠道的槽）。
fn ctx_with_channel(channel_key: &str) -> RequestCtx {
    let mut ctx = RequestCtx::new(make_meta(HeaderMap::new(), b"{}".to_vec()));
    ctx.route = Some(SelectedRoute {
        unit: contract::records::RouteUnitRecord {
            meta: meta(channel_key),
            group: "default".into(),
            public_model: "gpt-4o".into(),
            channel_key: channel_key.into(),
            key_index: 0,
            upstream_model: "gpt-4o".into(),
            priority: 0,
            weight: 1,
            status: 1,
        },
        secret: "sk-upstream".into(),
        base_url: "http://127.0.0.1:9999".into(),
        upstream_model: "gpt-4o".into(),
        provider_type: "openai".into(),
        settings: serde_json::Value::Null,
    });
    ctx
}

/// max=1 全流程：占槽 → 槽满拒绝 → drop ctx（请求结束）归还 → 重新可占。
/// 验证核心不变量：permit 的生命周期完全跟随 ctx，无需任何显式 release。
#[tokio::test]
async fn concurrency_permit_released_when_ctx_drops() {
    let state = Arc::new(ConcurrencyState::default());
    let gate = ConcurrencyGate::new(state.clone());
    // 槽位按 channel_key 索引（对齐 RouteUnitRecord.channel_key），不是数字 id。
    let channel = "ch-42";
    gate.register_channel(channel, 1);

    // 请求 1：抢到唯一槽位，Continue；permit 挂在 ctx.drop_guards 上。
    let mut ctx1 = ctx_with_channel(channel);
    assert!(matches!(
        gate.handle(&mut ctx1).await.unwrap(),
        StageOutcome::Continue
    ));
    assert_eq!(gate.available_permits(channel), 0, "唯一槽位已被占");

    // 请求 1 尚未结束、请求 2 并发进入（两个 ctx 同时存活）：
    // 槽满 → try_acquire 失败 → RateLimited（不排队、不阻塞）。
    let mut ctx2 = ctx_with_channel(channel);
    assert!(matches!(
        gate.handle(&mut ctx2).await.unwrap_err(),
        StageError::RateLimited
    ));

    // 请求 1 结束：ctx drop → RAII permit drop → 槽位归还。
    drop(ctx1);
    assert_eq!(gate.available_permits(channel), 1, "ctx drop 后槽位应归还");

    // 请求 3 可以重新占用归还后的槽位。
    let mut ctx3 = ctx_with_channel(channel);
    assert!(matches!(
        gate.handle(&mut ctx3).await.unwrap(),
        StageOutcome::Continue
    ));
    assert_eq!(gate.available_permits(channel), 0);
}

/// 未注册渠道：直接放行不限并发；available_permits 查询返回 0 且无 panic。
/// （回归护栏：旧实现 or_insert_with(Semaphore::new(0)) + acquire_owned 会让
/// 首个请求永久死锁——这里第二次请求仍秒回 Continue 即证明不再挂起。）
#[tokio::test]
async fn concurrency_unregistered_channel_passes_through() {
    let state = Arc::new(ConcurrencyState::default());
    let gate = ConcurrencyGate::new(state.clone());

    let mut ctx = ctx_with_channel("ghost-channel");
    assert!(matches!(
        gate.handle(&mut ctx).await.unwrap(),
        StageOutcome::Continue
    ));
    // 未注册渠道不产生 Semaphore entry，查询 0 不 panic。
    assert_eq!(gate.available_permits("ghost-channel"), 0);
    assert!(
        !state.semaphores.contains_key("ghost-channel"),
        "handle 不得为未注册渠道插入 Semaphore entry（旧实现的死锁根源）"
    );

    let mut ctx2 = ctx_with_channel("ghost-channel");
    assert!(matches!(
        gate.handle(&mut ctx2).await.unwrap(),
        StageOutcome::Continue
    ));
}

/// StageError::RateLimited → HTTP 429 映射（router 已有实现，这里钉住契约：
/// concurrency 槽满复用该变体，不得悄悄降级成 500）。
#[test]
fn stage_error_rate_limited_maps_to_429() {
    let resp = gateway_pipeline::error_to_response(StageError::RateLimited);
    assert_eq!(resp.status(), http::StatusCode::TOO_MANY_REQUESTS);
}

// ---------------------------------------------------------------------------
// 8. chain — 顺序短路 + happy path 提升 token
// ---------------------------------------------------------------------------

#[tokio::test]
async fn chain_short_circuits_on_first_failure() {
    let snapshot = TokenSnapshot::default();
    let snap = Arc::new(ArcSwap::from_pointee(snapshot));
    let auth = AuthGate::new(snap);

    let chain = GateChain::new().push(auth);
    let req = make_meta(HeaderMap::new(), b"{}".to_vec());
    let mut ctx = RequestCtx::new(req);

    let outcome = chain.handle(&mut ctx).await.expect("stage ok");
    match outcome {
        StageOutcome::ShortCircuit(resp) => {
            assert_eq!(resp.status(), 401);
        }
        _ => panic!("expected ShortCircuit"),
    }
    assert!(ctx.token.is_none());
}

#[tokio::test]
async fn chain_full_happy_path_lifts_token_to_ctx() {
    let raw = "sk-chain-1";
    let hash = sha256(raw);

    // token 表（quota 按 token.id == meta.key 查）
    let tokens = Arc::new(ArcSwap::from_pointee(TokenSnapshot::default()));
    tokens.load().upsert(
        hash,
        TokenEntry::new(
            token_record("1", "u1", Some("vip")),
            Some(vec!["gpt-4*".into()]),
        ),
    );

    let users = Arc::new(ArcSwap::from_pointee(UserSnapshot::default()));
    users.load().upsert(user_record("u1", "default", true));

    let pricing = Arc::new(ArcSwap::from_pointee(PricingSnapshot::default()));
    pricing.load().upsert(
        "gpt-4o".into(),
        "default".into(),
        PriceRow {
            input_per_m: 5.0,
            output_per_m: 15.0,
            cache_per_m: 1.0,
        },
    );
    // quota key 必须等于 token.id（== token 的 meta.key）== "1"
    let quotas = Arc::new(ArcSwap::from_pointee(QuotaSnapshot::default()));
    quotas.load().upsert("1".into(), 100_000_000);

    let ip_policy = Arc::new(ArcSwap::from_pointee(IpPolicy::allows_everyone()));
    let limiter = Arc::new(RateLimiter::new(1000, 60));

    let chain = GateChain::new()
        .push(AuthGate::new(tokens))
        .push(StateGate::new(users, ip_policy))
        .push(QuotaGate::new(quotas, pricing))
        .push(RateLimitGate::new(limiter))
        .push(ModelGate);

    let mut headers = HeaderMap::new();
    headers.insert("authorization", format!("Bearer {raw}").parse().unwrap());
    let req = make_meta(headers, br#"{"model":"gpt-4o","max_tokens":256}"#.to_vec());
    let mut ctx = RequestCtx::new(req);

    let outcome = chain.handle(&mut ctx).await.expect("stage ok");
    match outcome {
        StageOutcome::Continue => {}
        StageOutcome::ShortCircuit(resp) => {
            panic!(
                "expected Continue, got ShortCircuit: status={}",
                resp.status()
            );
        }
        _ => panic!("expected Continue"),
    }
    let promoted = ctx.token.as_ref().expect("token promoted");
    assert_eq!(promoted.group, "vip");
    assert!(
        promoted
            .allowed_models
            .as_ref()
            .unwrap()
            .contains(&"gpt-4*".to_string())
    );
}

// ---------------------------------------------------------------------------
// 9. IpPolicy — CIDR v4 + v6
// ---------------------------------------------------------------------------

#[test]
fn ip_policy_cidr_v4_and_v6() {
    let p = IpPolicy::from_cidrs(&["10.0.0.0/24".into(), "2001:db8::/32".into()]);
    let v4_in: IpAddr = "10.0.0.5".parse().unwrap();
    let v4_out: IpAddr = "10.0.1.5".parse().unwrap();
    let v6_in: IpAddr = "2001:db8::1".parse().unwrap();
    let v6_out: IpAddr = "2001:dead::1".parse().unwrap();
    assert!(p.allows(&v4_in));
    assert!(!p.allows(&v4_out));
    assert!(p.allows(&v6_in));
    assert!(!p.allows(&v6_out));

    let any = IpPolicy::allows_everyone();
    assert!(any.allows(&v4_in));
    assert!(any.allows(&v6_out));
}
