//! E2E: 总揽页（admin-page-overview）四端点 wire 契约 —— 真 Router + 真 PG，
//! 按前端实际发出的请求钉死响应形状。
//!
//! `web_wire_contract.rs` 文件头声称覆盖「前端消费的每个 /api 端点」，但总揽页
//! 四端点恰好是漏网之鱼（见 `todo/page-audit-overview.md` §2.3-1）：
//! - `GET /api/dashboard`
//! - `GET /api/log/top?by=user|model`
//! - `GET /api/log/trend?granularity=hour|day|month`
//! - `GET /api/monitor?days=7`
//!
//! 前端消费形状（`crates/web/admin-page-overview/src/api.rs`，本文件的钉子）：
//! - `get_dashboard_summary_api` 把响应**整体**反序列化为 `DashboardSummaryDto`
//!   —— 裸对象、无 `items` 包装（与 top/trend/monitor 的 `Items<T>` 剥壳不同）；
//! - `top_usage_api` / `trend_api` / `monitor_api` 走 `Items<T>` 剥壳：
//!   `{"items":[...]}`，行按 camelCase 解码（trend 行是 `modelName` 而非
//!   `model_name`；monitor 行是 `channelKey`/`okCount`/`avgLatencyMs`）。
//! 任何一侧漂移只会在浏览器里静默变空，这里的断言当场炸。
//!
//! 额度单位口径（全仓统一，new-api 语义）：内部整数 `500_000 = $1`
//! （`db/migrations/0002_usage_logs.sql`、`contract/src/api/usage.rs`）。
//! 断言一律用整数相等，不断言小数。
//!
//! 模式与 web_wire_contract.rs / usage_log_type.rs 一致：PG 不可达即 skip；
//! `build_app_with_egress` 建真 Router；tower oneshot 发前端同款请求。
//! 消费行种子走 `observe::logs::LogService::record`（写侧构造器同源，复用
//! usage_log_type.rs 的手法）；需要控制 `created_at` 的趋势种子用裸 INSERT。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bytes::Bytes;
use chrono::{DateTime, Datelike, SecondsFormat, Timelike, Utc};
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use observe::logs::{LogService, UsageEvent};
use observe::monitor::{ProbeOutcome, record_probe};
use serde_json::{Value, json};
use tower::ServiceExt;

/// e2e 专用测试口令：仅存在于本地/CI 一次性数据库，命名常量避免裸字面量。
const TEST_PASSWORD: &str = "test_password_123";

/// 额度单位锚：内部整数 500_000 = $1（new-api 语义）。
const QUOTA_PER_USD: i64 = 500_000;

struct MockEgress;

impl Egress for MockEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<ForwardedResponse, contract::error::NormalizedError>,
                > + Send
                + 'a,
        >,
    > {
        unreachable!("wire-contract tests never hit the forward plane; MockEgress is a placeholder")
    }
}

async fn response_to_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// 连不上 PG 时返回 None，调用方跳过测试（CI 无 postgres service）。
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

/// 本文件各测试共用一套建表段，且 dashboard 断言依赖**全局表计数**（不是唯一
/// 锚可隔离的），全部测试体串行执行：避免并行测试互相污染计数增量。
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 建真 app（触发 migrations + 全部子路由装配，与生产 build_app 同源）。
///
/// `build_app_with_egress` 硬性要求 `FERRITE_JWT_SECRET`（组装关注点，见
/// apps/api/src/lib.rs），缺失即 bail。测试进程自己兜底注入 e2e 专用密钥：
/// 仅用于本进程签发/校验测试 JWT，不接触任何真实环境。已由外部设置时不覆盖。
async fn build_test_app(pool: &sqlx::PgPool) -> axum::Router {
    ensure_jwt_secret();
    api::build_app_with_egress(pool.clone(), Arc::new(MockEgress))
        .await
        .expect("build_app_with_egress")
}

/// 进程级一次性注入 JWT secret。
///
/// `set_var` 在多线程下是 unsafe（Rust 2024）：用 `Once` 保证只在任何 app
/// 构建之前写一次，且写入点被 SERIAL 串行化，不与其他线程的 env 读并发。
fn ensure_jwt_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var("FERRITE_JWT_SECRET").is_err() {
            // SAFETY: 仅此一次写入，发生在所有 build_test_app 之前（Once +
            // SERIAL 串行化），此后全程只读。
            unsafe {
                std::env::set_var(
                    "FERRITE_JWT_SECRET",
                    "ferrite-e2e-overview-wire-contract-secret-2026",
                );
            }
        }
    });
}

/// 插入指定角色用户。role >= ADMIN_ROLE_THRESHOLD(10) 过 admin 白名单
/// （admin 用 100）；role=1 用来钉非管理员的 403。列类型 SMALLINT，绑 i16。
/// password_hash 必须是真实 argon2 PHC，否则 login 恒失败拿不到 JWT。
async fn insert_user(pool: &sqlx::PgPool, role: i16) -> (uuid::Uuid, String) {
    let user_key = uuid::Uuid::new_v4();
    let username = format!("ovw_{}_{}", role, &user_key.to_string()[..8]);
    let phc = auth::password::hash(TEST_PASSWORD).expect("argon2 hash");
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, password_hash, role, status)
           VALUES ($1, $2, $3, $4, 1)"#,
    )
    .bind(user_key)
    .bind(&username)
    .bind(&phc)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
    (user_key, username)
}

/// 走前端 AuthModal 的同款调用 `POST /api/user/login` 换 accessToken。
///
/// login handler 用 ConnectInfo 提取 client ip；oneshot 裸 Router 不自带，
/// 必须手动塞 extension，否则提取拒绝 → 500 空 body。
async fn login(app: &axum::Router, username: &str) -> String {
    let login_body = json!({"username": username, "password": TEST_PASSWORD});
    let req = Request::builder()
        .method("POST")
        .uri("/api/user/login")
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:4242".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
        .unwrap();
    let resp = ServiceExt::oneshot(app.clone(), req).await.unwrap();
    let status = resp.status();
    let body = response_to_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "login must succeed; status={status} body={body}"
    );
    body["accessToken"]
        .as_str()
        .unwrap_or_else(|| panic!("login response missing accessToken: {body}"))
        .to_string()
}

/// 带 Bearer 发 GET/PUT/POST/DELETE，返回响应（调用方按需断言状态码/形状）。
async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"));
    let req = match body {
        Some(v) => builder
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&v).unwrap()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    ServiceExt::oneshot(app.clone(), req).await.unwrap()
}

/// 插入启用渠道（status=1：dashboard 的 channelsEnabled 口径）。
/// keys/models/groups 与 web_wire_contract::insert_channel 同构。
async fn insert_channel(pool: &sqlx::PgPool) -> (uuid::Uuid, String) {
    let key = uuid::Uuid::new_v4();
    let name = format!("ch_{}", &key.to_string()[..8]);
    sqlx::query(
        r#"INSERT INTO api_channels (key, name, channel_type, base_url, keys, models, groups, status, test_model, remark)
           VALUES ($1, $2, 'openai', 'http://mock', '["sk-e2e-plaintext"]',
                   '[{"alias":"gpt-4o","upstream":"gpt-4o-up"}]', '{default}', 1, 'gpt-4o-mini', 'overview-wire-it')"#,
    )
    .bind(key)
    .bind(&name)
    .execute(pool)
    .await
    .unwrap();
    (key, name)
}

/// 插入一枚 API 密钥（dashboard 的 tokens 计数来自 api_tokens 表）。
async fn insert_api_token(pool: &sqlx::PgPool, user_key: uuid::Uuid) -> uuid::Uuid {
    let key = uuid::Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO api_tokens (key, user_key, name, key_hash)
           VALUES ($1, $2, $3, $4)"#,
    )
    .bind(key)
    .bind(user_key)
    .bind(format!("tk_{}", &key.to_string()[..8]))
    .bind(format!("sha256-{}", uuid::Uuid::new_v4().simple()))
    .execute(pool)
    .await
    .unwrap();
    key
}

/// 走写侧构造器 + `LogService::record` 落一条消费行（与 usage_log_type.rs
/// 同一手法的 wire 版）。返回 (model, request_id) 供断言定位。
async fn seed_consume(
    svc: &LogService,
    user_key: uuid::Uuid,
    username: &str,
    model: &str,
    prompt: i32,
    completion: i32,
    quota: i64,
) -> (String, String) {
    let mut e = UsageEvent::consume(user_key, username, model);
    e.prompt_tokens = prompt;
    e.completion_tokens = completion;
    e.quota = quota;
    let request_id = format!("ovw-{}", uuid::Uuid::new_v4().simple());
    e.request_id = request_id.clone();
    svc.record(&e).await.expect("record consume event");
    (model.to_string(), request_id)
}

/// 裸 INSERT 一条 `created_at` 受控的用量行 —— 只有趋势测试需要精确桶归属
/// 时才用；log_type 语义契约由 usage_log_type.rs 钉，这里允许手写枚举以模拟
/// 「写侧漂移行」（充值行/空模型行）对读侧过滤的回归。
#[allow(clippy::too_many_arguments)]
async fn seed_usage_row_at(
    pool: &sqlx::PgPool,
    log_type: i16,
    user_key: uuid::Uuid,
    username: &str,
    model: &str,
    prompt: i32,
    completion: i32,
    quota: i64,
    created_at: DateTime<Utc>,
) {
    let request_id = format!("ovw-{}", uuid::Uuid::new_v4().simple());
    sqlx::query(
        r#"INSERT INTO usage_logs
           (log_type, user_key, username, model_name, prompt_tokens,
            completion_tokens, quota, request_id, created_at)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"#,
    )
    .bind(log_type)
    .bind(user_key)
    .bind(username)
    .bind(model)
    .bind(prompt)
    .bind(completion)
    .bind(quota)
    .bind(request_id)
    .bind(created_at)
    .execute(pool)
    .await
    .unwrap();
}

/// 从库中读回某条记录的真实 created_at（record() 用 now() 默认值，与测试
/// 捕获的时钟有微差；桶断言以库内值为准，消除跨边界抖动）。
async fn fetch_created_at(pool: &sqlx::PgPool, request_id: &str) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT created_at FROM usage_logs WHERE request_id = $1")
        .bind(request_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

// date_trunc('hour'|'day'|'month', t) 的测试侧等价实现（UTC 口径，
// 与后端 date_trunc 对齐；timestamptz 截断按 PG 会话时区，e2e 库为 UTC）。
fn trunc_hour(t: DateTime<Utc>) -> DateTime<Utc> {
    t.date_naive()
        .and_hms_opt(t.hour(), 0, 0)
        .unwrap()
        .and_utc()
}

fn trunc_day(t: DateTime<Utc>) -> DateTime<Utc> {
    t.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
}

fn trunc_month(t: DateTime<Utc>) -> DateTime<Utc> {
    t.date_naive()
        .with_day(1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

/// 前端同款 RFC3339（秒精度 + Z 后缀，`api.rs::window_start` 格式）。
fn rfc3339(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

// ============================================================================
// 用例
// ============================================================================

/// 鉴权契约：总揽四端点全部在 admin 白名单内 —— 无 token → 401，
/// 登录但 role < 10（ADMIN_ROLE_THRESHOLD）→ 403。与 gateway/health 同规。
#[tokio::test]
async fn overview_endpoints_require_admin_token() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let _guard = SERIAL.lock().await;
    let app = build_test_app(&pool).await;

    // 无 token：全部 401（admin 白名单，非公开端点）。
    for uri in [
        "/api/dashboard",
        "/api/log/top",
        "/api/log/trend",
        "/api/monitor",
    ] {
        let anon = ServiceExt::oneshot(
            app.clone(),
            Request::builder().uri(uri).body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(
            anon.status(),
            StatusCode::UNAUTHORIZED,
            "{uri} 未带 token 必须 401"
        );
    }

    // 非 admin（role=1 < 阈值 10）：全部 403。
    let (_user, username) = insert_user(&pool, 1).await;
    let token = login(&app, &username).await;
    for uri in [
        "/api/dashboard",
        "/api/log/top",
        "/api/log/trend",
        "/api/monitor",
    ] {
        let resp = call(&app, "GET", uri, &token, None).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "{uri} 普通用户必须 403"
        );
    }
}

/// dashboard 契约：响应是**裸对象**（前端 `DashboardSummaryDto` 直接反序列化，
/// 无 items 包装）；9 个 camelCase 字段齐全且为整数；数值口径用 before/after
/// 差值钉死 —— 插 1 用户 + 1 密钥 + 1 启用渠道 + 1 分组 + 1 条消费
/// （quota=500_000 即 $1、tokens=1000）后逐字段对账。
#[tokio::test]
async fn dashboard_flat_envelope_fields_and_unit_deltas() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let _guard = SERIAL.lock().await;
    let app = build_test_app(&pool).await;
    let svc = LogService::new(pool.clone());
    let (user_key, admin_name) = insert_user(&pool, 100).await;
    let token = login(&app, &admin_name).await;

    // before 快照：此后到 after 之间的每一行种子都必须恰好对应下面一个差值断言。
    let before = {
        let resp = call(&app, "GET", "/api/dashboard", &token, None).await;
        assert_eq!(resp.status(), StatusCode::OK, "dashboard must be 200");
        response_to_json(resp).await
    };

    // 裸对象信封：任何 items 包装都会让前端 DashboardSummaryDto 解码炸掉。
    assert!(
        before.get("items").is_none(),
        "dashboard 必须是裸对象（无 items 包装），前端直接反序列化 DTO: {before}"
    );
    // 9 字段齐全且为整数（quotaToday = 500_000 ⇒ $1 的整数口径）。
    for field in [
        "users",
        "tokens",
        "channels",
        "channelsEnabled",
        "groups",
        "quotaToday",
        "requestsToday",
        "rpm",
        "tpm",
    ] {
        assert!(
            before[field].is_i64(),
            "dashboard.{field} 必须存在且为整数（camelCase）: {before}"
        );
    }

    // ---- 种子：每类恰好 +1，消费行带 $1 额度与 1000 tokens ----
    insert_user(&pool, 100).await; // users +1
    insert_api_token(&pool, user_key).await; // tokens +1
    insert_channel(&pool).await; // channels +1, channelsEnabled +1
    let group_name = format!("ovw_grp_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let created = call(
        &app,
        "POST",
        "/api/group",
        &token,
        Some(json!({"name": group_name, "ratio": 0.8, "modelWhitelist": [], "remark": "overview-wire"})),
    )
    .await;
    assert_eq!(
        created.status(),
        StatusCode::OK,
        "group create: {created:?}"
    ); // groups +1
    let (_model, _rid) = seed_consume(
        &svc,
        user_key,
        &admin_name,
        "ovw-dash-model",
        600,
        400,
        QUOTA_PER_USD,
    )
    .await;

    let after = {
        let resp = call(&app, "GET", "/api/dashboard", &token, None).await;
        assert_eq!(resp.status(), StatusCode::OK);
        response_to_json(resp).await
    };

    // 逐字段对账（before/after 都在本测试体内捕获且全程持 SERIAL 锁，
    // 其余测试的种子落在 before 之前，不进差值）。
    let delta = |field: &str| after[field].as_i64().unwrap() - before[field].as_i64().unwrap();
    assert_eq!(delta("users"), 1, "users 差值: {before} → {after}");
    assert_eq!(
        delta("tokens"),
        1,
        "api_tokens 计数差值: {before} → {after}"
    );
    assert_eq!(delta("channels"), 1, "channels 差值: {before} → {after}");
    assert_eq!(
        delta("channelsEnabled"),
        1,
        "status=1 渠道进 channelsEnabled: {before} → {after}"
    );
    assert_eq!(delta("groups"), 1, "groups 差值: {before} → {after}");
    assert_eq!(
        delta("quotaToday"),
        QUOTA_PER_USD,
        "quotaToday 差值必须是整数 500_000（=$1 口径，不接受小数折算）: {before} → {after}"
    );
    assert_eq!(
        delta("requestsToday"),
        1,
        "requestsToday 按 usage_logs 行数计（不分 log_type）: {before} → {after}"
    );
    // rpm/tpm 是近 60s 滑动窗：本测试体亚秒级完成，窗口内既有行不会老化
    // 出窗，差值应为精确 +1 / +1000。
    assert_eq!(delta("rpm"), 1, "rpm 近 60s 请求数差值: {before} → {after}");
    assert_eq!(
        delta("tpm"),
        1000,
        "tpm 近 60s tokens（prompt+completion）差值: {before} → {after}"
    );
}

/// log/top 契约：`{"items":[{name,tokens,quota,calls}]}`；by=user 按用户聚合、
/// by=model 按模型聚合、缺省 by 即 user；tokens 降序；空窗返回空数组（不是
/// 错误/ null）；limit 生效。tokens 口径 = prompt + completion 之和。
#[tokio::test]
async fn log_top_dimensions_ordering_empty_window_and_limit() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let _guard = SERIAL.lock().await;
    let app = build_test_app(&pool).await;
    let svc = LogService::new(pool.clone());
    let (user_key, admin_name) = insert_user(&pool, 100).await;
    let token = login(&app, &admin_name).await;

    // 榜眼 u1：两条消费行 → tokens 42 / quota 300 / calls 2；
    // 榜首 u2：一条消费行 → tokens 100。两条都挂在唯一 model 上供 by=model 复用。
    let user_a = format!("ovw_top_a_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let user_b = format!("ovw_top_b_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let model_a = format!("ovw-top-a-{}", uuid::Uuid::new_v4().simple());
    let model_b = format!("ovw-top-b-{}", uuid::Uuid::new_v4().simple());
    seed_consume(&svc, user_key, &user_a, &model_a, 10, 5, 100).await;
    seed_consume(&svc, user_key, &user_a, &model_a, 20, 7, 200).await;
    seed_consume(&svc, user_key, &user_b, &model_b, 60, 40, QUOTA_PER_USD).await;

    // by=user：聚合到用户名维度。
    let body = {
        let resp = call(&app, "GET", "/api/log/top?by=user&limit=50", &token, None).await;
        assert_eq!(resp.status(), StatusCode::OK);
        response_to_json(resp).await
    };
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("log/top 必须走 items 信封（前端 Items<T> 剥壳）: {body}"));
    let row = items
        .iter()
        .find(|r| r["name"] == user_a.as_str())
        .unwrap_or_else(|| panic!("by=user 必须聚合到用户名维度: {body}"));
    assert_eq!(row["tokens"], 42, "tokens = prompt + completion 两条求和");
    assert_eq!(row["quota"], 300, "quota 整数求和（内部单位）");
    assert_eq!(row["calls"], 2, "calls = 行数");
    let pos_a = items
        .iter()
        .position(|r| r["name"] == user_a.as_str())
        .unwrap();
    let pos_b = items
        .iter()
        .position(|r| r["name"] == user_b.as_str())
        .unwrap();
    assert!(
        pos_b < pos_a,
        "tokens 降序：user_b(100) 必须排在 user_a(42) 前"
    );

    // 缺省 by 参数 → 后端按 user 维度（`unwrap_or("user")`）。
    let body = {
        let resp = call(&app, "GET", "/api/log/top?limit=50", &token, None).await;
        assert_eq!(resp.status(), StatusCode::OK);
        response_to_json(resp).await
    };
    assert!(
        body["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == user_a.as_str()),
        "缺省 by 必须等价 by=user: {body}"
    );

    // by=model：同一批种子换模型维度聚合。
    let body = {
        let resp = call(&app, "GET", "/api/log/top?by=model&limit=50", &token, None).await;
        assert_eq!(resp.status(), StatusCode::OK);
        response_to_json(resp).await
    };
    let items = body["items"].as_array().unwrap();
    let row = items
        .iter()
        .find(|r| r["name"] == model_a.as_str())
        .unwrap_or_else(|| panic!("by=model 必须聚合到模型名维度: {body}"));
    assert_eq!(row["tokens"], 42);
    assert_eq!(row["calls"], 2);
    let pos_a = items
        .iter()
        .position(|r| r["name"] == model_a.as_str())
        .unwrap();
    let pos_b = items
        .iter()
        .position(|r| r["name"] == model_b.as_str())
        .unwrap();
    assert!(pos_b < pos_a, "by=model 同样 tokens 降序");

    // 空窗：窗口落在未来 → items 为空数组（不是错误、不是 null）。前端
    // 空态（empty_window）依赖这一行为。
    let start = rfc3339(Utc::now() + chrono::Duration::hours(48));
    let end = rfc3339(Utc::now() + chrono::Duration::hours(72));
    let body = {
        let resp = call(
            &app,
            "GET",
            &format!("/api/log/top?by=user&start={start}&end={end}"),
            &token,
            None,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK, "空窗也是 200: {resp:?}");
        response_to_json(resp).await
    };
    assert_eq!(
        body["items"].as_array().unwrap().len(),
        0,
        "窗口无数据 → items 空数组: {body}"
    );

    // limit 生效：limit=1 只回 1 行（后端 clamp 1..=50，默认 10）。
    let body = {
        let resp = call(&app, "GET", "/api/log/top?by=user&limit=1", &token, None).await;
        assert_eq!(resp.status(), StatusCode::OK);
        response_to_json(resp).await
    };
    assert_eq!(
        body["items"].as_array().unwrap().len(),
        1,
        "limit=1 截到 1 行: {body}"
    );
}

/// log/trend 契约：`{"items":[{bucket,modelName,tokens,quota,calls}]}`（camelCase
/// `modelName`）；hour/day/month 三种 granularity 的桶起始分别截到时/日/月；
/// 未知 granularity 回落 hour；只聚合消费行且 model_name <> ''（充值行、
/// 空模型行、错误行全部排除）；空窗返回空数组——服务端不补零，补零是前端
/// pivot_trend 的职责。
#[tokio::test]
async fn log_trend_granularities_exclusions_and_empty_window() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let _guard = SERIAL.lock().await;
    let app = build_test_app(&pool).await;
    let svc = LogService::new(pool.clone());
    let (user_key, admin) = insert_user(&pool, 100).await;
    let token = login(&app, &admin).await;

    let model_now = format!("ovw-trend-now-{}", uuid::Uuid::new_v4().simple());
    let model_past = format!("ovw-trend-past-{}", uuid::Uuid::new_v4().simple());
    let model_topup = format!("ovw-trend-topup-{}", uuid::Uuid::new_v4().simple());

    // 本小时行：走写侧构造器（visibility 契约由 usage_log_type.rs 钉，这里复用）。
    let (_m, rid_now) = seed_consume(&svc, user_key, "ovw_trend", &model_now, 10, 5, 100).await;
    let created_now = fetch_created_at(&pool, &rid_now).await;
    // 3 小时前的行：裸 INSERT 控制时间，落在不同小时桶。
    seed_usage_row_at(
        &pool,
        2,
        user_key,
        "ovw_trend",
        &model_past,
        7,
        3,
        50,
        created_now - chrono::Duration::hours(3),
    )
    .await;
    // 排除面：充值行（log_type=1，模拟写侧漂移）与空模型消费行不进趋势。
    seed_usage_row_at(
        &pool,
        1,
        user_key,
        "ovw_trend",
        &model_topup,
        999,
        999,
        999,
        created_now,
    )
    .await;
    seed_usage_row_at(
        &pool,
        2,
        user_key,
        "ovw_trend",
        "",
        500,
        500,
        500,
        created_now,
    )
    .await;

    // granularity=hour：桶起始截到整点，行按模型分列。
    let rows = trend_rows(&app, &token, "hour").await;
    let now_row = rows
        .iter()
        .find(|r| r["modelName"] == model_now.as_str())
        .unwrap_or_else(|| panic!("hour 桶缺本小时行（注意 camelCase modelName）: {rows:?}"));
    assert_eq!(
        now_row["bucket"].as_str().unwrap(),
        rfc3339(trunc_hour(created_now)),
        "hour 桶 = date_trunc('hour', created_at)"
    );
    assert_eq!(now_row["tokens"], 15, "tokens = prompt + completion");
    assert_eq!(now_row["quota"], 100);
    assert_eq!(now_row["calls"], 1);
    let past_row = rows
        .iter()
        .find(|r| r["modelName"] == model_past.as_str())
        .unwrap_or_else(|| panic!("hour 桶缺 3 小时前行: {rows:?}"));
    assert_eq!(
        past_row["bucket"].as_str().unwrap(),
        rfc3339(trunc_hour(created_now - chrono::Duration::hours(3))),
        "不同小时的行落不同桶，不被合并"
    );

    // granularity=day：桶起始截到 UTC 当日 0 点。
    let rows = trend_rows(&app, &token, "day").await;
    let now_row = rows
        .iter()
        .find(|r| r["modelName"] == model_now.as_str())
        .unwrap_or_else(|| panic!("day 桶缺行: {rows:?}"));
    assert_eq!(
        now_row["bucket"].as_str().unwrap(),
        rfc3339(trunc_day(created_now)),
        "day 桶 = 当日 00:00:00Z"
    );

    // granularity=month：桶起始截到当月 1 号 0 点。
    let rows = trend_rows(&app, &token, "month").await;
    let now_row = rows
        .iter()
        .find(|r| r["modelName"] == model_now.as_str())
        .unwrap_or_else(|| panic!("month 桶缺行: {rows:?}"));
    assert_eq!(
        now_row["bucket"].as_str().unwrap(),
        rfc3339(trunc_month(created_now)),
        "month 桶 = 当月 1 日 00:00:00Z"
    );

    // 未知 granularity（前端只发 hour/day/month，但服务端 `_ => "hour"` 回落）
    // —— 钉死回落行为，防止有人改成 4xx 或 panic。
    let rows = trend_rows(&app, &token, "week").await;
    let now_row = rows
        .iter()
        .find(|r| r["modelName"] == model_now.as_str())
        .unwrap_or_else(|| panic!("未知 granularity 应回落 hour: {rows:?}"));
    assert_eq!(
        now_row["bucket"].as_str().unwrap(),
        rfc3339(trunc_hour(created_now)),
        "未知 granularity 回落 hour 桶"
    );

    // 排除面横切三种粒度中最常用的小时桶：充值行与空模型行都不可见。
    for granularity in ["hour", "day", "month"] {
        let rows = trend_rows(&app, &token, granularity).await;
        assert!(
            rows.iter().all(|r| r["modelName"] != model_topup.as_str()),
            "log_type=1（充值）不得进趋势聚合: {granularity} {rows:?}"
        );
        assert!(
            rows.iter().all(|r| r["modelName"] != ""),
            "model_name <> '' 过滤：空模型行不得进趋势: {granularity} {rows:?}"
        );
    }

    // 空窗：窗口落在未来 → items 空数组。服务端不补零（补零是前端
    // pivot_trend 的职责），这里钉「空窗 = 空 items」的服务端行为。
    let start = rfc3339(Utc::now() + chrono::Duration::hours(48));
    let end = rfc3339(Utc::now() + chrono::Duration::hours(72));
    let body = {
        let resp = call(
            &app,
            "GET",
            &format!("/api/log/trend?granularity=hour&start={start}&end={end}"),
            &token,
            None,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK, "空窗也是 200: {resp:?}");
        response_to_json(resp).await
    };
    assert_eq!(
        body["items"].as_array().unwrap().len(),
        0,
        "窗口无数据 → items 空数组（服务端不补零桶）: {body}"
    );
}

/// GET /api/log/trend 的 items 数组（camelCase 行形状由调用方断言）。
async fn trend_rows(app: &axum::Router, token: &str, granularity: &str) -> Vec<Value> {
    let resp = call(
        app,
        "GET",
        &format!("/api/log/trend?granularity={granularity}"),
        token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "trend {granularity}");
    let body = response_to_json(resp).await;
    body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("trend 必须走 items 信封: {body}"))
        .clone()
}

/// monitor 契约：`{"items":[{channelKey,days,total,okCount,availability,avgLatencyMs}]}`
/// （camelCase）；availability = okCount/total（0.0..=1.0 小数，不是百分数）；
/// avgLatencyMs 只对成功样本求均值；只回有记录的渠道（无记录渠道不出现在
/// items，而非 total=0 行）；单渠道端点对无记录渠道回 total=0 + null 空态；
/// days clamp 1..=90。
#[tokio::test]
async fn monitor_availability_aggregation_and_empty_states() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let _guard = SERIAL.lock().await;
    let app = build_test_app(&pool).await;
    let (_user, admin) = insert_user(&pool, 100).await;
    let token = login(&app, &admin).await;
    let (ch_key, _ch_name) = insert_channel(&pool).await;

    // 4 次探活：3 成功（延迟 100/200/300，均值 200）+ 1 失败（延迟 999，
    // 失败样本既不进 okCount 也不进延迟均值）。
    let probes = [
        (true, Some(200), 100, ""),
        (true, Some(200), 200, ""),
        (true, Some(200), 300, ""),
        (false, Some(503), 999, "http"),
    ];
    for (ok, status_code, latency, error_kind) in probes {
        record_probe(
            &pool,
            &ProbeOutcome {
                channel_key: ch_key,
                channel_name: "ovw-monitor-ch".into(),
                model: "gpt-4o-mini".into(),
                ok,
                status_code,
                latency_ms: latency,
                error_kind: error_kind.into(),
                message: "overview wire contract".into(),
            },
        )
        .await
        .expect("record probe");
    }

    let body = {
        let resp = call(&app, "GET", "/api/monitor?days=7", &token, None).await;
        assert_eq!(resp.status(), StatusCode::OK, "monitor list: {resp:?}");
        response_to_json(resp).await
    };
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("monitor 必须走 items 信封（前端 Items<T> 剥壳）: {body}"));
    let row = items
        .iter()
        .find(|r| r["channelKey"] == ch_key.to_string())
        .unwrap_or_else(|| {
            panic!("有探活记录的渠道必须出现在 items（camelCase channelKey）: {body}")
        });
    assert_eq!(row["days"], 7, "回显请求的 days");
    assert_eq!(row["total"], 4, "窗口内全部探活行数（含失败）");
    assert_eq!(row["okCount"], 3, "okCount 只计成功行");
    let availability = row["availability"]
        .as_f64()
        .unwrap_or_else(|| panic!("availability 必须是数值: {row}"));
    assert!(
        (availability - 0.75).abs() < 1e-9,
        "availability = okCount/total = 3/4 = 0.75（0..=1 小数，不是百分数 75）: {row}"
    );
    let avg = row["avgLatencyMs"]
        .as_f64()
        .unwrap_or_else(|| panic!("avgLatencyMs 必须是数值: {row}"));
    assert!(
        (avg - 200.0).abs() < 1e-9,
        "avgLatencyMs 只对成功样本求均值：(100+200+300)/3=200，失败样本 999 不计入: {row}"
    );
    // GROUP BY 聚合面：只回有记录的渠道，绝无 total=0 的占位行。
    assert!(
        items.iter().all(|r| r["total"].as_i64().unwrap_or(0) >= 1),
        "monitor list 只列有记录的渠道（无记录渠道缺席，而非 0 行）: {items:?}"
    );

    // 单渠道端点的「无记录空态」：全新 UUID 无任何探活 → total=0、
    // okCount=0、availability 与 avgLatencyMs 均 null（前端空态灰色的依据）。
    let empty_key = uuid::Uuid::new_v4();
    let body = {
        let resp = call(
            &app,
            "GET",
            &format!("/api/monitor/{empty_key}?days=7"),
            &token,
            None,
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "无记录渠道也是 200: {resp:?}"
        );
        response_to_json(resp).await
    };
    assert!(
        body["history"].as_array().unwrap().is_empty(),
        "无记录 → history 空数组: {body}"
    );
    let avail = &body["availability"];
    assert_eq!(avail["total"], 0);
    assert_eq!(avail["okCount"], 0);
    assert!(
        avail["availability"].is_null(),
        "窗口内无记录 → availability null（不是 0.0）: {avail}"
    );
    assert!(
        avail["avgLatencyMs"].is_null(),
        "无成功样本 → avgLatencyMs null: {avail}"
    );

    // days clamp：0 收到 1（合法域 1..=90），不报错。
    let body = {
        let resp = call(
            &app,
            "GET",
            &format!("/api/monitor/{empty_key}?days=0"),
            &token,
            None,
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "days=0 走 clamp 不报错: {resp:?}"
        );
        response_to_json(resp).await
    };
    assert_eq!(body["availability"]["days"], 1, "days clamp 下界 1: {body}");
}
