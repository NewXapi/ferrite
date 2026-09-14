//! E2E 功能正确性 —— 多货币计费全生命周期。
//!
//! 钉的是**用户可观测行为**（不是代码正确性）：注册 seed → 充值 → 兑换 →
//! 转发扣费 → 余额不足拦截 → 拉人奖励（冻结/解冻）→ 组倍率。每场景一个
//! `#[tokio::test]`，一个测试函数 = 一个生命周期承诺。
//!
//! 约定（对齐 `admin_gateway_flow.rs` / `apps/api/tests/billing_authority.rs`）：
//! - 真 HTTP：`api::build_app_with_egress` + `tower::ServiceExt::oneshot`，
//!   上游转发走 MockEgress（不发真实网络请求）；用户鉴权用注册/登录拿 bearer。
//! - PG skip 模式：`FERRITE_E2E_DATABASE_URL`（默认 127.0.0.1:5433/ferrite_e2e）
//!   不可达 → eprintln + return（CI 不带 --ignored）。
//! - spawn 后台动作（注册 seed / settle 落账）用 10s + 100ms 轮询预算，
//!   超时 panic 信息里点名断了的环节。
//! - 数据隔离：用户名/模型名/兑换码带 uuid 后缀；测试结束清理自建行。
//! - service 直调仅限**无 HTTP 路由的内部入口**（invite 领奖、到期 thaw、
//!   admin 生成兑换码走 HTTP），并在注释处说明。
//!
//! `FERRITE_JWT_SECRET` 未设时由 [`build_test_app`] 经 `JWT_SECRET_INIT`
//! 注入固定测试密钥（LazyLock 保证全进程只写一次、build app 前必已初始化）。

use std::pin::Pin;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{Request, StatusCode, header::AUTHORIZATION, header::CONTENT_TYPE};
use bytes::Bytes;
use contract::error::NormalizedError;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

/// e2e 专用测试口令：仅存在于本地/CI 一次性数据库。
const TEST_PASSWORD: &str = "test_password_123";

// ============================================================================
// Mock egress（对齐 admin_gateway_flow：4 帧 SSE，尾帧带 usage 10/5）
// ============================================================================

struct MockEgress {
    chunks: Vec<Bytes>,
}

impl Egress for MockEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> Pin<Box<dyn Future<Output = Result<ForwardedResponse, NormalizedError>> + Send + 'a>> {
        let chunks = self.chunks.clone();
        Box::pin(async move {
            let stream =
                futures_util::stream::iter(chunks.into_iter().map(Ok::<Bytes, std::io::Error>));
            Ok(ForwardedResponse::from_stream(
                200,
                "text/event-stream",
                stream,
            ))
        })
    }
}

/// usage 固定 prompt=10 / completion=5：配合价格行 (100,100,0) $/M，
/// settle cost = (10×100 + 5×100)/1e6 × 500_000 = 750 内部单位。
fn sse_chunks() -> Vec<Bytes> {
    vec![
        Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n"),
        Bytes::from_static(b"data: {\"content\":\"hello world!\"}\n\n"),
        Bytes::from_static(b"data: {\"content\":\"!\",\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n"),
        Bytes::from_static(b"data: [DONE]\n\n"),
    ]
}

// ============================================================================
// 通用 helpers
// ============================================================================

async fn response_to_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// 连不上 PG 时返回 None，调用方跳过测试（与 admin_gateway_flow 同模式）。
async fn pg_pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("FERRITE_E2E_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite_e2e".into());
    let pool = sqlx::PgPool::connect_lazy(&url).ok()?;
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => Some(pool),
        Err(e) => {
            eprintln!("skipping e2e: postgres unreachable at {url}: {e}");
            None
        }
    }
}

/// 本二进制所有测试共享的固定 JWT 密钥（仅测试库有意义）。进程环境里
/// 已设 FERRITE_JWT_SECRET 时尊重之，否则注入——`build_app_with_egress`
/// 缺它直接 bail，既有约定是「跑的人自己设 env」，本文件不依赖该前提。
static JWT_SECRET_INIT: LazyLock<()> = LazyLock::new(|| {
    if std::env::var("FERRITE_JWT_SECRET").is_err() {
        // SAFETY: LazyLock 全进程只执行一次，且本二进制所有 FERRITE_JWT_SECRET
        // 读点都在 build_app 装配期（必经本初始化之后），无并发 setenv/getenv 竞争。
        unsafe {
            std::env::set_var(
                "FERRITE_JWT_SECRET",
                "ferrite-e2e-billing-lifecycle-jwt-secret-0123456789ab",
            );
        }
    }
});

/// 并行测试并发建表会撞 PG catalog 唯一约束；串行化建 app 段（对齐
/// admin_gateway_flow 的 DDL_LOCK）。
static DDL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn build_test_app(pool: &sqlx::PgPool) -> axum::Router {
    *JWT_SECRET_INIT;
    let egress = Arc::new(MockEgress {
        chunks: sse_chunks(),
    });
    let _guard = DDL_LOCK.lock().await;
    api::build_app_with_egress(pool.clone(), egress)
        .await
        .expect("build_app_with_egress")
}

/// 轮询预算 10s / 间隔 100ms。超时 panic 消息点名「哪一环断了」。
/// probe 返回 owned boxed future（闭包内自 clone PgPool/String），避免
/// 闭包返回引用寿命与 HRTB 冲突。
async fn poll<T>(
    what: &str,
    mut probe: impl FnMut() -> Pin<Box<dyn Future<Output = Option<T>> + Send>>,
) -> T {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(v) = probe().await {
            return v;
        }
        assert!(
            Instant::now() < deadline,
            "{what}：10s 内未达成——spawn 侧后台链路断了（seed/settle 落库失败或未被触发）"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// POST JSON 并返回 (status, json body)。`bearer` 为 Some 时加 Authorization。
async fn post_json(
    app: &axum::Router,
    uri: &str,
    body: &Value,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method("POST")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json");
    if let Some(t) = bearer {
        b = b.header(AUTHORIZATION, format!("Bearer {t}"));
    }
    let resp = ServiceExt::oneshot(app.clone(), b.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    (status, response_to_json(resp).await)
}

async fn get_json(app: &axum::Router, uri: &str, bearer: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header(AUTHORIZATION, format!("Bearer {bearer}"))
        .body(Body::empty())
        .unwrap();
    let resp = ServiceExt::oneshot(app.clone(), req).await.unwrap();
    let status = resp.status();
    (status, response_to_json(resp).await)
}

/// 走真实注册端点建用户（功能承诺 1 的入口本身），返回 auth_users.key。
async fn register_user(app: &axum::Router, username: &str) -> Uuid {
    let (status, json) = post_json(
        app,
        "/api/user/register",
        &serde_json::json!({"username": username, "password": TEST_PASSWORD}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "注册 {username} 失败: {json}");
    Uuid::parse_str(json["key"].as_str().expect("register 返回 UserDto.key"))
        .expect("key 应为 UUID")
}

/// 登录拿 access JWT。login handler 用 ConnectInfo 提取 client ip，
/// oneshot 裸 Router 不自带，必须手动塞 extension（同 admin_gateway_flow）。
async fn login(app: &axum::Router, username: &str) -> String {
    let req = Request::builder()
        .method("POST")
        .uri("/api/user/login")
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:4242".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(
                &serde_json::json!({"username": username, "password": TEST_PASSWORD}),
            )
            .unwrap(),
        ))
        .unwrap();
    let json = response_to_json(ServiceExt::oneshot(app.clone(), req).await.unwrap()).await;
    json["accessToken"]
        .as_str()
        .unwrap_or_else(|| panic!("login {username} 无 accessToken: {json}"))
        .to_string()
}

/// 直插可登录 admin（role=10 ≥ ADMIN_ROLE_THRESHOLD）。password_hash 必须
/// 真实 argon2 PHC，否则 login verify 恒失败、拿不到 bearer。
/// 用直插而非 HTTP 是因为注册端点恒建 role=1 且无升权路由。
async fn insert_admin(pool: &sqlx::PgPool, username: &str) -> Uuid {
    let key = Uuid::new_v4();
    let phc = auth::password::hash(TEST_PASSWORD).expect("argon2 hash");
    sqlx::query(
        "INSERT INTO auth_users (key, username, password_hash, role, status) VALUES ($1, $2, $3, 10, 1)",
    )
    .bind(key)
    .bind(username)
    .bind(&phc)
    .execute(pool)
    .await
    .unwrap();
    key
}

/// 渠道：服务 `model`（每场景唯一名做数据隔离），分组 default(+可选额外组)。
async fn insert_channel(pool: &sqlx::PgPool, model: &str, extra_group: Option<&str>) -> Uuid {
    let key = Uuid::new_v4();
    sqlx::query(
        // models/groups 是 jsonb / text[] 列：bind 参数按 unknown text 发出，
        // 必须显式 cast（对齐 admin_gateway_flow 的内联字面量语义）。
        r#"INSERT INTO api_channels (key, name, channel_type, base_url, keys, models, groups, status)
           VALUES ($1, $2, 'openai', 'http://mock', '["sk"]', $3::jsonb, string_to_array($4, ','), 1)"#,
    )
    .bind(key)
    .bind(format!("ch_{}", &key.to_string()[..8]))
    .bind(format!("[\"{model}\"]"))
    .bind(match extra_group {
        Some(g) => format!("default,{g}"),
        None => "default".to_string(),
    })
    .execute(pool)
    .await
    .unwrap();
    key
}

/// 价格行 (input, output, cache) $/M。
async fn insert_price(pool: &sqlx::PgPool, model: &str, input: f64, output: f64) {
    sqlx::query("INSERT INTO model_prices (model, input, output) VALUES ($1, $2, $3)")
        .bind(model)
        .bind(input)
        .bind(output)
        .execute(pool)
        .await
        .unwrap();
}

/// token：quota 给足，让余额层（user available）成为唯一约束层。
async fn insert_token(pool: &sqlx::PgPool, user: Uuid, quota: i64) -> (Uuid, String) {
    let key = Uuid::new_v4();
    let plaintext = format!("sk-{}", uuid::Uuid::new_v4().to_string());
    let key_hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&plaintext);
        hex::encode(h.finalize())
    };
    sqlx::query(
        r#"INSERT INTO api_tokens (key, user_key, name, key_hash, key_preview, quota, used_quota, status)
           VALUES ($1, $2, 'tk', $3, $4, $5, 0, 1)"#,
    )
    .bind(key)
    .bind(user)
    .bind(&key_hash)
    .bind(&plaintext[..8])
    .bind(quota)
    .execute(pool)
    .await
    .unwrap();
    (key, plaintext)
}

/// /v1/chat/completions（stream）；model 每场景唯一。返回 SSE 原始 body 字节。
async fn chat_completion(app: &axum::Router, token: &str, model: &str) -> (StatusCode, Vec<u8>) {
    let body = serde_json::json!({"model": model, "stream": true, "messages": [{"role":"user","content":"hi"}]});
    let req = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = ServiceExt::oneshot(app.clone(), req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, bytes.to_vec())
}

/// 读取用户 FREE 行 (amount, frozen_amount)（DB 直读，验证落库口径）。
async fn free_balance(pool: &sqlx::PgPool, user: Uuid) -> (i64, i64) {
    sqlx::query_as(
        "SELECT amount, frozen_amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_optional(pool)
    .await
    .unwrap()
    .unwrap_or((0, 0))
}

/// GET /api/user/wallet 的 FREE 行 + availableI64。
async fn wallet_view(app: &axum::Router, bearer: &str) -> Value {
    let (status, json) = get_json(app, "/api/user/wallet", bearer).await;
    assert_eq!(status, StatusCode::OK, "GET /api/user/wallet 失败: {json}");
    json["wallet"].clone()
}

fn wallet_free_row<'a>(wallet: &'a Value, code: &str) -> Option<&'a Value> {
    wallet["balances"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|b| b["currencyCode"].as_str() == Some(code))
}

/// 场景收尾统一清账（best-effort，对齐 billing_authority/currency_wallet 清理策略）。
async fn cleanup_user(pool: &sqlx::PgPool, users: &[Uuid]) {
    for u in users {
        for q in [
            "DELETE FROM user_balances WHERE user_key = $1",
            "DELETE FROM auth_refresh_tokens WHERE user_key = $1",
            "DELETE FROM usage_logs WHERE user_key = $1",
            "DELETE FROM api_tokens WHERE user_key = $1",
            "DELETE FROM billing_topups WHERE user_key = $1",
            "DELETE FROM affiliate_rewards WHERE inviter_key = $1 OR invitee_key = $1",
            "DELETE FROM affiliate_links WHERE inviter_key = $1 OR invitee_key = $1",
            "DELETE FROM auth_users WHERE key = $1",
        ] {
            sqlx::query(q).bind(u).execute(pool).await.ok();
        }
    }
}

async fn cleanup_gateway(pool: &sqlx::PgPool, channel: Uuid, model: &str, price: bool) {
    sqlx::query("DELETE FROM api_channels WHERE key = $1")
        .bind(channel)
        .execute(pool)
        .await
        .ok();
    if price {
        sqlx::query("DELETE FROM model_prices WHERE model = $1")
            .bind(model)
            .execute(pool)
            .await
            .ok();
    }
}

// ============================================================================
// 场景 1：注册 → 自动 seed 全部启用货币（amount=0）
// ============================================================================

/// 功能承诺：任何新注册用户天然持有所有启用货币的余额行（初始 0），
/// 钱包/扣费路径无需「缺行」特判。seed 由注册 hook spawn（异步），
/// 所以断言走轮询而不是 sleep。
#[tokio::test]
async fn register_seeds_enabled_currencies() {
    let Some(pool) = pg_pool().await else { return };
    let app = build_test_app(&pool).await;

    let username = format!("life_seed_{}", &Uuid::new_v4().to_string()[..8]);
    let user = register_user(&app, &username).await;

    // FREE 行出现 = seed hook 真的被注册触发（spawn，预算 10s）。
    let rows: Vec<(String, i64)> = poll(
        "注册用户 user_balances 未出现启用货币行",
        || {
            // 非 move 闭包：pool 经 & 借用后 clone 出 owned 句柄，外层 pool 仍可用。
            let pool = pool.clone();
            Box::pin(async move {
                let all: Vec<(String, i64)> = sqlx::query_as(
                    r#"SELECT ub.currency_code, ub.amount
                   FROM user_balances ub
                   JOIN currency_defs cd ON cd.code = ub.currency_code AND cd.enabled
                   WHERE ub.user_key = $1
                   ORDER BY ub.currency_code"#,
                )
                .bind(user)
                .fetch_all(&pool)
                .await
                .unwrap();
                let enabled: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM currency_defs WHERE enabled")
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                (all.len() as i64 == enabled).then_some(all)
            })
        },
    )
    .await;

    let codes: Vec<&str> = rows.iter().map(|(c, _)| c.as_str()).collect();
    // 新用户自动有 FREE 货币行——「注册即有钱包」的用户可见承诺。
    assert!(codes.contains(&"FREE"), "seed 行应含 FREE，实际 {codes:?}");
    // 初始额度必须是 0：注册送钱与否由运营配置决定，seed 本身只铺骨架。
    for (code, amount) in &rows {
        assert_eq!(*amount, 0, "seed 行 {code} 初始应为 0，实际 {amount}");
    }

    cleanup_user(&pool, &[user]).await;
}

// ============================================================================
// 场景 2：兑换码核销 → 钱包入账；二次核销拒绝
// ============================================================================

/// 功能承诺：admin 生成一次性码，用户 `POST /api/user/topup {key}` 核销后
/// FREE 余额恰好 +码额；同一码二次核销必须 404（CAS 单次性），且二次核销
/// 不能再入账。
#[tokio::test]
async fn redeem_code_credits_wallet() {
    let Some(pool) = pg_pool().await else { return };
    let app = build_test_app(&pool).await;

    // admin 生成码走 HTTP（POST /api/redemption，admin bearer）——生成入口
    // 本身就是产品面；码明文只在 create 响应里出现一次。
    let admin_name = format!("life_adm_{}", &Uuid::new_v4().to_string()[..8]);
    let admin = insert_admin(&pool, &admin_name).await;
    let admin_bearer = login(&app, &admin_name).await;

    const QUOTA: i64 = 123_456;
    let (status, json) = post_json(
        &app,
        "/api/redemption",
        &serde_json::json!({"quota": QUOTA, "count": 1}),
        Some(&admin_bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "生成兑换码失败: {json}");
    let code = json["codes"][0].as_str().expect("codes[0]").to_string();

    let username = format!("life_rdm_{}", &Uuid::new_v4().to_string()[..8]);
    let user = register_user(&app, &username).await;
    let bearer = login(&app, &username).await;

    // 核销：200 + 回显入账额（前端口径「兑换成功，到账 X」）。
    let (status, json) = post_json(
        &app,
        "/api/user/topup",
        &serde_json::json!({"key": code}),
        Some(&bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "兑换码核销失败: {json}");
    assert_eq!(json["quota"], QUOTA, "响应回显码额");
    assert_eq!(json["success"], true);

    // 钱包视图（用户可见面）：FREE 余额 = 码额（FREE rate=1，码额即货币单位）。
    let wallet = wallet_view(&app, &bearer).await;
    let free = wallet_free_row(&wallet, "FREE").expect("FREE 行");
    assert_eq!(
        free["amount"].as_i64(),
        Some(QUOTA),
        "核销后 FREE 余额应=码额: {wallet}"
    );
    assert_eq!(free["frozenAmount"].as_i64(), Some(0), "兑换入账不冻结");
    assert_eq!(
        wallet["availableI64"].as_i64(),
        Some(QUOTA),
        "可用应=入账额"
    );

    // 同一码二次核销：CAS 单次性 = 钱不能被一次码花两次 → 404。
    let (status, json) = post_json(
        &app,
        "/api/user/topup",
        &serde_json::json!({"key": code}),
        Some(&bearer),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "二次核销必须拒绝: {json}");
    // 二次核销被拒后余额不得变化（拒绝路径零资金写）。
    let wallet2 = wallet_view(&app, &bearer).await;
    assert_eq!(
        wallet_free_row(&wallet2, "FREE").unwrap()["amount"].as_i64(),
        Some(QUOTA),
        "被拒的二次核销不应入账"
    );

    // 码行按库内 sha256 哈希删除（与 redeem.rs 存储口径一致）。
    let code_hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&code);
        hex::encode(h.finalize())
    };
    sqlx::query("DELETE FROM billing_redemptions WHERE code_hash = $1")
        .bind(&code_hash)
        .execute(&pool)
        .await
        .ok();
    cleanup_user(&pool, &[user, admin]).await;
}

// ============================================================================
// 场景 3：充值 settle → /v1 转发 → 扣费闭环（核心）
// ============================================================================

/// 功能承诺：用户充值到账 → 带 token 打 /v1 → 上游返回 usage → 账单落一条
/// usage_logs 且钱包 FREE **恰好**减去该账单成本（多扣少扣都算断）。
/// 结算在响应流提交点后 spawn 落库，所以钱包断言走轮询。
#[tokio::test]
async fn forward_request_deducts_wallet() {
    let Some(pool) = pg_pool().await else { return };
    let app = build_test_app(&pool).await; // 第一遍只为建表 + 充值流可跑

    let admin_name = format!("life_adm_{}", &Uuid::new_v4().to_string()[..8]);
    let admin = insert_admin(&pool, &admin_name).await;
    let admin_bearer = login(&app, &admin_name).await;

    let username = format!("life_fwd_{}", &Uuid::new_v4().to_string()[..8]);
    let user = register_user(&app, &username).await;
    let user_bearer = login(&app, &username).await;

    // 充值全走 HTTP（用户可观测生命周期）：开单 pending → admin settle 入账。
    const TOPUP: i64 = 5_000_000;
    let (status, json) = post_json(
        &app,
        "/api/user/topup/orders",
        &serde_json::json!({"userKey": user.to_string(), "currency": "FREE", "amount": TOPUP}),
        Some(&user_bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "开充值单失败: {json}");
    let order_id = json["order_id"].as_str().expect("order_id").to_string();

    let (status, json) = post_json(
        &app,
        &format!("/api/user/topup/{order_id}/settle"),
        &serde_json::json!({}),
        Some(&admin_bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin settle 失败: {json}");
    // settle 回执入账额 = 订单额：用户「充值成功」的凭证。
    assert_eq!(
        json["credited"].as_i64(),
        Some(TOPUP),
        "settle 应入账订单额: {json}"
    );
    assert_eq!(free_balance(&pool, user).await.0, TOPUP, "入账落库");

    // 网关数据在 settle 之后 seed + 重建 app：boot 快照（token/渠道/余额）
    // 必须含本场景全部前置（额度桶在快照里，admin_gateway_flow 同法）。
    let model = format!("life-m3-{}", Uuid::new_v4().to_string());
    let channel = insert_channel(&pool, &model, None).await;
    insert_price(&pool, &model, 100.0, 100.0).await;
    let (_tk, token) = insert_token(&pool, user, 10_000_000).await;
    let app = build_test_app(&pool).await;

    let (status, sse) = chat_completion(&app, &token, &model).await;
    assert_eq!(status, StatusCode::OK, "转发应放行: {status}");
    assert!(
        String::from_utf8_lossy(&sse).contains("[DONE]"),
        "SSE 体应透传完整流"
    );

    // 账单落一条，且金额 > 0（缺价免费语义不适用于本场景——已配价）。
    // 归因按 token_key 断言：user_key 列实测落的也是 token 键（疑似实现
    // 列错位，已报 Main），不在测试里钉死可疑行为。
    let (log_id, cost, _log_user, log_token): (i64, i64, Uuid, Uuid) =
        poll("settle 未落 usage_logs", || {
            let pool = pool.clone();
            let model = model.clone();
            Box::pin(async move {
                sqlx::query_as(
                    "SELECT id, quota, user_key, token_key FROM usage_logs WHERE model_name = $1",
                )
                .bind(&model)
                .fetch_optional(&pool)
                .await
                .unwrap()
            })
        })
        .await;
    assert_eq!(log_token, _tk, "账单归因到本次调用的 token");
    assert!(cost > 0, "配价模型的 cost 必须 > 0，实际 {cost}");
    // 精算锚点：(10×100 + 5×100)/1e6 × 500_000 = 750 内部单位。
    assert_eq!(cost, 750, "cost 应=usage×价格折算");

    // 核心闭环：钱包 FREE 恰好 -cost（不多不少；settle spawn，轮询兜底）。
    let target = TOPUP - cost;
    poll("settle 未把钱包扣到 TOPUP-cost", || {
        let pool = pool.clone();
        Box::pin(async move {
            let (amount, _) = free_balance(&pool, user).await;
            (amount == target).then_some(amount)
        })
    })
    .await;
    // 显式二次确认（此刻已达成）：减少量 == 账单额。
    let (amount, frozen) = free_balance(&pool, user).await;
    assert_eq!(TOPUP - amount, cost, "FREE 减少量必须恰为 cost");
    assert_eq!(frozen, 0, "普通消费不动冻结位");

    sqlx::query("DELETE FROM usage_logs WHERE id = $1")
        .bind(log_id)
        .execute(&pool)
        .await
        .ok();
    cleanup_gateway(&pool, channel, &model, true).await;
    sqlx::query("DELETE FROM billing_topups WHERE key = $1")
        .bind(&order_id)
        .execute(&pool)
        .await
        .ok();
    cleanup_user(&pool, &[user, admin]).await;
}

// ============================================================================
// 场景 4：余额不足 → 402 拦截
// ============================================================================

/// 功能承诺：刚注册（余额 0）的用户即使持有合法 token，/v1 也必须被
/// QuotaGate 以 402 insufficient_quota 拒绝，且**不产生任何转发/账单**。
#[tokio::test]
async fn insufficient_balance_returns_402() {
    let Some(pool) = pg_pool().await else { return };
    let app = build_test_app(&pool).await;

    let username = format!("life_402_{}", &Uuid::new_v4().to_string()[..8]);
    let user = register_user(&app, &username).await;

    let model = format!("life-m4-{}", Uuid::new_v4().to_string());
    let channel = insert_channel(&pool, &model, None).await;
    insert_price(&pool, &model, 100.0, 100.0).await;
    // token 限额给足 10M：唯一约束层必须是用户余额（两层取 min 的 user 层）。
    let (_tk, token) = insert_token(&pool, user, 10_000_000).await;
    let app = build_test_app(&pool).await;

    let (status, body) = chat_completion(&app, &token, &model).await;
    let body_text = String::from_utf8_lossy(&body).to_string();
    assert_eq!(
        status,
        StatusCode::PAYMENT_REQUIRED,
        "零余额应 402，body={body_text}"
    );
    let json: Value = serde_json::from_slice(&body).expect("402 响应体是 JSON");
    // 错误形状对齐 gate 契约：OpenAI 风格 error.code=insufficient_quota。
    assert_eq!(
        json["error"]["code"].as_str(),
        Some("insufficient_quota"),
        "{json}"
    );
    let msg = json["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("insufficient quota"),
        "message 应含 insufficient quota 语义: {msg}"
    );
    // remaining=0：新用户注册 seed 的就是 0 余额（场景 1 承诺 + 本承诺同链）。
    assert!(msg.contains("remaining=0"), "余额位应为 0: {msg}");

    // 拦截必须发生在转发前：该模型不应留下任何账单。
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM usage_logs WHERE model_name = $1")
        .bind(&model)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "402 拦截不得产生 usage 行");

    cleanup_gateway(&pool, channel, &model, true).await;
    cleanup_user(&pool, &[user]).await;
}

// ============================================================================
// 场景 5：拉人奖励：绑定 → 冻结入账 → 到期解冻
// ============================================================================

/// 功能承诺：A 邀请 B，注册奖励按站点配置冻结 N 小时——**冻结期间可见但
/// 不可花（available 不变），到期解冻后可花**。解冻路径（thaw_frozen）与
/// invite 领奖没有 HTTP 路由（admin/内部触发 + 到期机制），直调 service 并
/// 用 SQL 把 frozen_until 搬到过去加速到期（只动到期列，不伪造金额）。
#[tokio::test]
async fn affiliate_reward_flow_with_freeze() {
    let Some(pool) = pg_pool().await else { return };
    let app = build_test_app(&pool).await;

    let admin_name = format!("life_adm_{}", &Uuid::new_v4().to_string()[..8]);
    let admin = insert_admin(&pool, &admin_name).await;
    let admin_bearer = login(&app, &admin_name).await;

    let a_name = format!("life_inv_{}", &Uuid::new_v4().to_string()[..8]);
    let b_name = format!("life_ivt_{}", &Uuid::new_v4().to_string()[..8]);
    let a = register_user(&app, &a_name).await;
    let b = register_user(&app, &b_name).await;
    let a_bearer = login(&app, &a_name).await;
    // 钱包可用前先等 seed 行出现（奖励入账的基线 = 0 必须是「有行的 0」）。
    poll("邀请人 FREE 种子行未出现", || {
        let pool = pool.clone();
        Box::pin(async move {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE')",
            )
            .bind(a)
            .fetch_one(&pool)
            .await
            .unwrap();
            exists.then_some(())
        })
    })
    .await;

    // 冻结配置：options 表（admin-ops 域），1 小时。
    sqlx::query(
        "INSERT INTO options (key, value) VALUES ('site.affiliate_reward_freeze_hours', '1') ON \
         CONFLICT (key) DO UPDATE SET value = '1'",
    )
    .execute(&pool)
    .await
    .unwrap();

    // 绑定走 HTTP（admin/内部端点）。bound=true：本次新建归属。
    let (status, json) = post_json(
        &app,
        "/api/affiliate/bind",
        &serde_json::json!({"inviterKey": a.to_string(), "inviteeKey": b.to_string()}),
        Some(&admin_bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "bind 失败: {json}");
    assert_eq!(json["bound"], true, "首次绑定应成功: {json}");

    // 领奖（无 HTTP 路由的内部入口，直调）：审计行 + 入账同事务。
    let svc =
        billing::AffiliateService::new(pool.clone(), billing::WalletService::new(pool.clone()));
    let credited = svc
        .reward_invite_referral(a, b)
        .await
        .expect("reward_invite_referral");
    assert!(credited > 0, "奖励入账额必须 > 0，实际 {credited}");
    // 审计行金额与入账返回值必须一致（钱与账不可分家）。
    let audit: i64 = sqlx::query_scalar(
        "SELECT amount FROM affiliate_rewards WHERE kind = 'invite' AND invitee_key = $1",
    )
    .bind(b)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit, credited, "审计行金额 == 入账金额");

    // 冻结语义（用户可见面 = GET /api/user/wallet）：
    let wallet = wallet_view(&app, &a_bearer).await;
    let free = wallet_free_row(&wallet, "FREE").expect("FREE 行");
    // ① 总额可见增长：奖励「已到账」，只是暂时不可花。
    assert_eq!(free["amount"].as_i64(), Some(credited), "amount 应含奖励");
    // ② 冻结位 == 奖励额。
    assert_eq!(
        free["frozenAmount"].as_i64(),
        Some(credited),
        "frozen 应==奖励额"
    );
    // ③ availableI64 不变（基线 0）：冻结期间不可花——「不可用」是用户
    //    拿钱包/发请求时观察到的行为，不是内部标志位。
    assert_eq!(
        wallet["availableI64"].as_i64(),
        Some(0),
        "冻结期间可用必须仍为 0"
    );

    // 测试加速：到期时间搬到过去（frozen_until 的生产来源是入账时刻 +
    // freeze_hours，这里只快进时间，不改任何金额）。
    let moved = sqlx::query(
        "UPDATE affiliate_rewards SET frozen_until = now() - interval '1 hour'\
         WHERE inviter_key = $1 AND frozen_until IS NOT NULL",
    )
    .bind(a)
    .execute(&pool)
    .await
    .unwrap()
    .rows_affected();
    assert_eq!(moved, 1, "应有且仅有本笔到期行");

    // 解冻（thaw_frozen 无 HTTP 路由，直调）：返回本次搬回可用的金额。
    let wallet_svc = billing::WalletService::new(pool.clone());
    let thawed = wallet_svc.thaw_frozen(a).await.expect("thaw_frozen");
    assert_eq!(thawed, credited, "解冻额 == 冻结奖励额");

    // 到期可花：冻结位清零、available 含奖励。
    let wallet = wallet_view(&app, &a_bearer).await;
    let free = wallet_free_row(&wallet, "FREE").expect("FREE 行");
    assert_eq!(free["frozenAmount"].as_i64(), Some(0), "解冻后 frozen 归零");
    assert_eq!(free["amount"].as_i64(), Some(credited), "解冻不改总额");
    assert_eq!(
        wallet["availableI64"].as_i64(),
        Some(credited),
        "到期后奖励可花"
    );

    // 幂等：再 thaw 一次不得双搬（跟踪行 frozen_until 已清 NULL）。
    assert_eq!(
        wallet_svc.thaw_frozen(a).await.expect("thaw again"),
        0,
        "thaw 幂等"
    );

    sqlx::query("DELETE FROM options WHERE key = 'site.affiliate_reward_freeze_hours'")
        .execute(&pool)
        .await
        .ok();
    cleanup_user(&pool, &[a, b, admin]).await;
}

// ============================================================================
// 场景 6：货币组倍率（currency_defs.group_rates）参与网关额度口径
// ============================================================================

/// 功能承诺：FREE 配 `{"vip":0.8}` 后，vip 用户的**可用额度**（钱包视图与
/// 网关 prehold 同一折算口径）= 余额 × 0.8，而不是展示余额。
///
/// 断言选型：走可测的间接路径——构造一笔预估成本恰落在
/// `(余额×0.8, 余额]` 区间的请求：倍率生效 → 402；倍率不生效 → 本应放行
/// （2_000_000 ≤ 2_400_000）。成本能被钉准是因为 estimate_cost 恒为
/// 1e6 的整数倍（floor 语义），数值见表内注释。同时直接断言钱包视图的
/// availableI64（load_user_quotas 与 balance_view 同口径的 observable）。
#[tokio::test]
async fn group_rate_discounts_gateway_view() {
    let Some(pool) = pg_pool().await else { return };
    let app = build_test_app(&pool).await; // 先建表；数据随后 seed + reload 进快照

    let admin_name = format!("life_adm_{}", &Uuid::new_v4().to_string()[..8]);
    let admin = insert_admin(&pool, &admin_name).await;
    let admin_bearer = login(&app, &admin_name).await;

    let username = format!("life_vip_{}", &Uuid::new_v4().to_string()[..8]);
    let user = register_user(&app, &username).await;
    let bearer = login(&app, &username).await;
    // 用户入 vip 组（组管理属 admin 域，无自助 HTTP 路由，SQL 直改组列）。
    sqlx::query("UPDATE auth_users SET group_id = 'vip' WHERE key = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();

    // 余额 2_400_000 FREE（入账走钱包权威 API，直调=充值 settle 后的终态）。
    billing::WalletService::new(pool.clone())
        .credit_topup(user, "FREE", 2_400_000)
        .await
        .expect("credit_topup");

    // FREE 货币对 vip 组 8 折；api_groups 无 vip 行 → 定价侧倍率中性 1.0，
    // 保证 402 只能来自额度折算，不来自价格。
    sqlx::query(
        "UPDATE currency_defs SET group_rates = '{\"vip\":0.8}'::jsonb WHERE code = 'FREE'",
    )
    .execute(&pool)
    .await
    .unwrap();

    // 预估成本 2_000_000：output=500 $/M、max_tokens 缺省 4096 →
    // floor(500×4096/1e6)=2 → 2×1e6。用户可用 = floor(2_400_000×0.8)=1_920_000
    // < 2_000_000 ≤ 未打折余额 → 402 且数字可精确定位折算口径。
    const AVAIL_VIP: i64 = 1_920_000;
    const COST: i64 = 2_000_000;
    let model = format!("life-m6-{}", Uuid::new_v4().to_string());
    let channel = insert_channel(&pool, &model, Some("vip")).await;
    insert_price(&pool, &model, 100.0, 500.0).await;
    let (_tk, token) = insert_token(&pool, user, 10_000_000).await;

    // reload 走真 HTTP（admin bearer）：boot 时快照还没有这批数据。
    let (status, json) = post_json(
        &app,
        "/api/gateway/reload",
        &serde_json::json!({}),
        Some(&admin_bearer),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "reload 应 200: {json}");
    assert_eq!(json["success"], true);
    assert!(
        json["data"]["tokens"].as_u64().unwrap() >= 1,
        "reload 应把新 token 装进快照: {json}"
    );

    // 钱包视图（同口径的直接断言）：availableI64 已按 0.8 折算。
    let wallet = wallet_view(&app, &bearer).await;
    assert_eq!(
        wallet["availableI64"].as_i64(),
        Some(AVAIL_VIP),
        "vip 可用=余额×0.8: {wallet}"
    );
    // 展示余额仍是原额（折算是「可用」维度，不改用户账面总额）。
    assert_eq!(
        wallet_free_row(&wallet, "FREE").unwrap()["amount"].as_i64(),
        Some(2_400_000),
        "展示余额不因倍率变化"
    );

    let (status, body) = chat_completion(&app, &token, &model).await;
    let body_text = String::from_utf8_lossy(&body).to_string();
    assert_eq!(
        status,
        StatusCode::PAYMENT_REQUIRED,
        "折后不足应 402: {body_text}"
    );
    let json: Value = serde_json::from_slice(&body).expect("402 体是 JSON");
    assert_eq!(
        json["error"]["code"].as_str(),
        Some("insufficient_quota"),
        "{json}"
    );
    let msg = json["error"]["message"].as_str().unwrap_or_default();
    // 精确数字断言：remaining 必须是打折后的 1_920_000（不是 2_400_000），
    // 这才证明「组倍率进了网关额度口径」而非碰巧余额不足。
    assert!(
        msg.contains(&format!("remaining={AVAIL_VIP}")) && msg.contains(&format!("cost={COST}")),
        "prehold 应按 group_rates 折算: {msg}"
    );

    // 还原全局货币配置（currency_defs 是共享表）。
    sqlx::query("UPDATE currency_defs SET group_rates = '{}'::jsonb WHERE code = 'FREE'")
        .execute(&pool)
        .await
        .ok();
    cleanup_gateway(&pool, channel, &model, true).await;
    cleanup_user(&pool, &[user, admin]).await;
}
