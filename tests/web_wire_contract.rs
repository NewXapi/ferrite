//! E2E: 管理端 wire 契约 —— 前端（admin-client / admin-page-*）消费的每个
//! `/api` 端点，按前端实际发出的请求打真 Router + 真 PG，钉死响应形状。
//!
//! 历史断裂（本文件存在的理由）：
//! - `GET /api/group`、`GET /api/channel`、`GET /api/models` 返回裸 map
//!   `{"items":[..]}`（渠道另带 `total`），前端列表 helper 曾按裸 `Vec<Dto>`
//!   解码 → 运行时 `invalid type: map, expected a sequence`，分组/渠道页
//!   永远走 error 分支（#182/#154/#160 分别修复）。任何一侧字段/包装漂移，
//!   这里的断言当场炸，而不是前端页面静默变空。
//! - `PUT /api/channel/{key}` 的 minimal-diff 语义（#160）：`keys` 缺席 =
//!   后端保现有密钥（COALESCE）；`testModel` 直绑无 COALESCE，缺席即清列
//!   ——前端必须恒带现值。这里从 HTTP 层钉死后端行为，防止 svc 契约被改。
//! - `/api/gateway/health`（#171/#172）是 admin 白名单新端点，钉鉴权与信封。
//! - 总览页五端点（#204）：`/api/dashboard`、`/api/log/top`、`/api/log/trend`、
//!   `/api/log/errors`、`/api/monitor` 此前是「每个端点都钉死」承诺的漏网之鱼
//!   （`todo/page-audit-overview.md` §2.3-1）——前端 admin-page-overview 按
//!   `{"items":[...]}` 剥壳 + camelCase 反序列化，任一侧漂移只会在浏览器里
//!   静默变空。这里按前端同款请求钉死鉴权与信封/字段形状。
//!
//! 模式与 admin_gateway_flow.rs 一致：PG 不可达即 skip；`build_app_with_egress`
//! 建真 Router；tower oneshot 发前端同款请求。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bytes::Bytes;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use serde_json::{Value, json};
use tower::ServiceExt;

/// e2e 专用测试口令：仅存在于本地/CI 一次性数据库，命名常量避免裸字面量。
const TEST_PASSWORD: &str = "test_password_123";

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

/// 本文件各测试共用一套建表段；全局 mutex 串行化，避免并发 CREATE TABLE
/// 撞 PG catalog 唯一约束（pg_type_typname_nsp_index）。
static DDL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 建真 app（触发 migrations + 全部子路由装配，与生产 build_app 同源）。
///
/// `build_app_with_egress` 硬性要求 `FERRITE_JWT_SECRET`（组装关注点，见
/// apps/api/src/lib.rs），缺失即 bail。测试进程自己兜底注入 e2e 专用密钥：
/// 仅用于本进程签发/校验测试 JWT，不接触任何真实环境（值同
/// `scripts/dev-backend.sh` 的本地默认，便于与 dev 后端对照排查）。
/// 已由外部设置时不覆盖，方便 CI/本地按需换值。
async fn build_test_app(pool: &sqlx::PgPool) -> axum::Router {
    let _guard = DDL_LOCK.lock().await;
    ensure_jwt_secret();
    api::build_app_with_egress(pool.clone(), Arc::new(MockEgress))
        .await
        .expect("build_app_with_egress")
}

/// 进程级一次性注入 JWT secret。
///
/// `set_var` 在多线程下是 unsafe（Rust 2024）：用 `Once` 保证只在任何 app
/// 构建之前写一次，且写入点被 DDL_LOCK 串行化，不与其他线程的 env 读并发。
fn ensure_jwt_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var("FERRITE_JWT_SECRET").is_err() {
            // SAFETY: 仅此一次写入，发生在所有 build_test_app 之前（Once +
            // DDL_LOCK 串行化），此后全程只读。
            unsafe {
                std::env::set_var(
                    "FERRITE_JWT_SECRET",
                    "ferrite-e2e-wire-contract-secret-2026",
                );
            }
        }
    });
}

/// 插入 admin 用户（role=100，过 ADMIN_ROLE_THRESHOLD=10 白名单）。
/// password_hash 必须是真实 argon2 PHC，否则 login 恒失败拿不到 JWT。
async fn insert_admin_user(pool: &sqlx::PgPool) -> (uuid::Uuid, String) {
    let user_key = uuid::Uuid::new_v4();
    let username = format!("admin_{}", &user_key.to_string()[..8]);
    let phc = auth::password::hash(TEST_PASSWORD).expect("argon2 hash");
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, password_hash, role, status)
           VALUES ($1, $2, $3, 100, 1)"#,
    )
    .bind(user_key)
    .bind(&username)
    .bind(&phc)
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
        "admin login must succeed; status={status} body={body}"
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

/// 插入渠道；返回 (key_uuid, name)。keys/models/groups 与
/// admin_gateway_flow::insert_channel 同构（真实列表页数据形状）。
async fn insert_channel(pool: &sqlx::PgPool) -> (uuid::Uuid, String) {
    let key = uuid::Uuid::new_v4();
    let name = format!("ch_{}", &key.to_string()[..8]);
    sqlx::query(
        r#"INSERT INTO api_channels (key, name, channel_type, base_url, keys, models, groups, status, test_model, remark)
           VALUES ($1, $2, 'openai', 'http://mock', '["sk-e2e-plaintext"]',
                   '[{"alias":"gpt-4o","upstream":"gpt-4o-up"}]', '{default}', 1, 'gpt-4o-mini', 'wire-it')"#,
    )
    .bind(key)
    .bind(&name)
    .execute(pool)
    .await
    .unwrap();
    (key, name)
}

/// 无凭据 GET：admin 白名单端点 401 半边断言用（与 gateway_health 用例的
/// 匿名请求同款——oneshot 裸请求不带 Authorization；缺 header 在 bearer_token
/// 处落 AuthError::InvalidToken → 401）。
async fn anon_get(app: &axum::Router, uri: &str) -> axum::response::Response {
    ServiceExt::oneshot(
        app.clone(),
        Request::builder().uri(uri).body(Body::empty()).unwrap(),
    )
    .await
    .unwrap()
}

/// 落一条用量流水。写侧与 usage_log_type.rs 同源：observe::logs 的构造器
/// （consume/error，log_type 常量唯一定义点）+ LogService::record，
/// 禁止手写 log_type 字面量。
async fn record_usage(pool: &sqlx::PgPool, e: &observe::logs::UsageEvent) {
    observe::logs::LogService::new(pool.clone())
        .record(e)
        .await
        .expect("record usage event");
}

/// 测后清理：按 model_name 精确删除本用例写入的流水行（uuid 隔离键，只碰
/// 自己的行，严禁批量清库）。断言中途 panic 时残留靠 uuid 隔离兜底，
/// 不会污染其他用例或后续运行。
async fn cleanup_usage_by_model(pool: &sqlx::PgPool, models: &[&str]) {
    for m in models {
        sqlx::query("DELETE FROM usage_logs WHERE model_name = $1")
            .bind(m)
            .execute(pool)
            .await
            .expect("cleanup usage_logs");
    }
}

// ============================================================================
// 用例
// ============================================================================

/// 前端「渠道页列表」契约：`GET /api/channel` 返回 `{items, total}`、
/// ChannelView camelCase、keys 掩码不回传明文。#160 活体 bug 的回归闸。
#[tokio::test]
async fn channel_list_items_envelope_and_camel_case() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;
    let (ch_key, ch_name) = insert_channel(&pool).await;

    let resp = call(&app, "GET", "/api/channel", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "channel list must be 200");
    let body = response_to_json(resp).await;

    // 信封：{items, total} —— 裸数组解码会在前端炸 invalid type: map。
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("channel list must wrap items: {body}"));
    assert!(
        body["total"].is_i64(),
        "channel list must carry total: {body}"
    );
    let row = items
        .iter()
        .find(|r| r["key"] == ch_key.to_string())
        .unwrap_or_else(|| panic!("inserted channel must be listed: {body}"));
    // ChannelView camelCase（serde rename_all）：前端 DTO 按这些字段解码。
    assert_eq!(row["name"], ch_name);
    assert_eq!(row["channelType"], "openai");
    assert_eq!(row["baseUrl"], "http://mock");
    assert_eq!(row["testModel"], "gpt-4o-mini");
    assert_eq!(row["groups"], json!(["default"]));
    // 列表端点 include_keys=false → keys 为 null（掩码 keys 只在单查回传）；
    // 密钥数量走 keyCount。明文密钥在任何列表响应中都不应出现。
    assert!(
        row["keys"].is_null(),
        "list endpoint must not carry keys (include_keys=false): {}",
        row["keys"]
    );
    assert_eq!(row["keyCount"], 1, "keyCount 是列表页的密钥数展示维度");
    assert!(
        !body.to_string().contains("sk-e2e-plaintext"),
        "plaintext key must never be echoed back"
    );
}

/// 前端「分组页列表」契约：`GET /api/group` 返回 `{items}`、GroupView
/// camelCase（含 modelWhitelist —— #182 白名单 CRUD 的读侧锚点）。
#[tokio::test]
async fn group_list_items_envelope_and_camel_case() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let group_name = format!("grp_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let created = call(
        &app,
        "POST",
        "/api/group",
        &token,
        Some(json!({"name": group_name, "ratio": 0.8, "modelWhitelist": ["gpt-4o"], "remark": "wire"})),
    )
    .await;
    assert_eq!(
        created.status(),
        StatusCode::OK,
        "group create: {created:?}"
    );

    let resp = call(&app, "GET", "/api/group", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("group list must wrap items: {body}"));
    let row = items
        .iter()
        .find(|r| r["name"] == group_name)
        .unwrap_or_else(|| panic!("created group must be listed: {body}"));
    assert_eq!(row["ratio"], 0.8);
    assert_eq!(row["modelWhitelist"], json!(["gpt-4o"]));
    assert_eq!(row["status"], 1);
    assert!(row["key"].is_string(), "GroupView.key 是前端编辑的定位键");
}

/// 前端「模型/别名页」契约：`GET /api/models?size=100` 返回 `{items}`，
/// 每项至少有 `name`（ModelView 唯一投影字段，拓扑中间层同名依赖）。
#[tokio::test]
async fn models_list_items_envelope() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;
    insert_channel(&pool).await; // models 列表从渠道 models JSONB 展开

    let resp = call(&app, "GET", "/api/models?size=100", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("models list must wrap items: {body}"));
    assert!(
        items.iter().all(|m| m["name"].is_string()),
        "ModelView 投影至少含 name: {items:?}"
    );
}

/// 前端「系统页站点配置」契约：`GET /api/option` 返回 `{items}`、
/// OptionView camelCase（key/value/updatedAt）。#176 读侧锚点。
#[tokio::test]
async fn options_list_items_envelope() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    // 先 PUT 一条（update_option_api 的后端半边）：key 必须在注册表内
    // （options.rs registry 校验，未注册 key → 400），选 observe 档中性值。
    let put = call(
        &app,
        "PUT",
        "/api/option",
        &token,
        Some(json!({"key": "observe.retention.usage_days", "value": 60})),
    )
    .await;
    assert_eq!(put.status(), StatusCode::OK, "option put: {put:?}");

    let resp = call(&app, "GET", "/api/option", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("options list must wrap items: {body}"));
    let row = items
        .iter()
        .find(|r| r["key"] == "observe.retention.usage_days")
        .unwrap_or_else(|| panic!("written option must be listed: {body}"));
    assert_eq!(row["value"], 60);
    assert!(
        row["updatedAt"].is_string(),
        "OptionView camelCase updatedAt"
    );
}

/// 前端「网关健康面板」契约：`GET /api/gateway/health` admin 白名单
/// （无 token → 401；admin token → 200 `{items}`）。#171/#172 锚点。
#[tokio::test]
async fn gateway_health_admin_gated_items_envelope() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    // 无 token：401（admin 白名单，非公开端点）。
    let anon = ServiceExt::oneshot(
        app.clone(),
        Request::builder()
            .uri("/api/gateway/health")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    // admin token：200 + items 信封（空库无记录 → 空数组，非 404/非错误）。
    let resp = call(&app, "GET", "/api/gateway/health", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_to_json(resp).await;
    assert!(
        body["items"].is_array(),
        "gateway health must wrap items: {body}"
    );
}

/// 前端「渠道编辑弹窗」的 minimal-diff 契约（#160 UpdateChannelBody 的后端半边）：
/// - `keys` 缺席 → 现有密钥保持（COALESCE）；
/// - `testModel` 缺席 → 列被清 NULL（直绑无 COALESCE —— 前端必须恒带现值，
///   这里钉死后端行为，防 svc 把直绑改成 COALESCE 后前端锚点失效）；
/// - `testModel` 显式值 → 列更新；
/// - `models`/`priority`/`weight` 缺席 → 不被静默清零。
#[tokio::test]
async fn channel_update_minimal_diff_semantics() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;
    let (ch_key, ch_name) = insert_channel(&pool).await;

    // 第一次 PUT：改名 + 不带 keys + 不带 testModel（前端编辑弹窗不重输密钥时的形状）。
    let resp = call(
        &app,
        "PUT",
        &format!("/api/channel/{ch_key}"),
        &token,
        Some(json!({"name": format!("{ch_name}_renamed"), "remark": "edited"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "minimal-diff put: {resp:?}");

    let keys_raw: serde_json::Value =
        sqlx::query_scalar("SELECT keys FROM api_channels WHERE key = $1")
            .bind(ch_key)
            .fetch_one(&pool)
            .await
            .unwrap();
    let keys: Vec<String> = serde_json::from_value(keys_raw).unwrap();
    assert_eq!(
        keys,
        vec!["sk-e2e-plaintext".to_string()],
        "keys 缺席必须保持现有密钥（COALESCE），否则编辑保存一次就清空渠道密钥"
    );
    let (test_model, models, priority): (Option<String>, Value, i32) =
        sqlx::query_as("SELECT test_model, models, priority FROM api_channels WHERE key = $1")
            .bind(ch_key)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        test_model, None,
        "testModel 缺席 → 列清 NULL（直绑无 COALESCE 的后端语义；前端恒带现值正是对准它）"
    );
    assert_eq!(
        models,
        json!([{"alias": "gpt-4o", "upstream": "gpt-4o-up"}]),
        "models 缺席不得被静默清零"
    );
    assert_eq!(priority, 0, "priority 缺席保持原值");
    let renamed: String = sqlx::query_scalar("SELECT name FROM api_channels WHERE key = $1")
        .bind(ch_key)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(renamed, format!("{ch_name}_renamed"));

    // 第二次 PUT：带 keys + testModel（用户重输密钥/改测试模型的形状）。
    let resp = call(
        &app,
        "PUT",
        &format!("/api/channel/{ch_key}"),
        &token,
        Some(json!({"keys": ["sk-rotated"], "testModel": "gpt-4o"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (keys_raw, test_model): (serde_json::Value, Option<String>) =
        sqlx::query_as("SELECT keys, test_model FROM api_channels WHERE key = $1")
            .bind(ch_key)
            .fetch_one(&pool)
            .await
            .unwrap();
    let keys: Vec<String> = serde_json::from_value(keys_raw).unwrap();
    assert_eq!(keys, vec!["sk-rotated".to_string()], "显式 keys 必须落库");
    assert_eq!(test_model.as_deref(), Some("gpt-4o"), "显式 testModel 落库");
}

/// 前端「兑换码页」契约：`GET /api/redemption` 返回 items 包装。
/// #177 读侧锚点（RedemptionItems 解包的后端半边）。
#[tokio::test]
async fn redemption_list_items_envelope() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let resp = call(&app, "GET", "/api/redemption", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "redemption list: {resp:?}");
    let body = response_to_json(resp).await;
    assert!(
        body["items"].is_array(),
        "redemption list must wrap items: {body}"
    );
}

// ============================================================================
// 总览页五端点（#204，todo/page-audit-overview.md §2.3-1）
// ============================================================================

/// 前端「总览页统计卡」契约：`GET /api/dashboard` admin 白名单（无 token →
/// 401），200 响应必须携带既有 9 个统计字段 + W1 新增 `quotaRemaining`(i64)
/// 与 `asOf`（非空 RFC3339 —— 前端 `as_of_local_time` 靠它渲染数据截止时刻，
/// 解析失败即诚实降级，这里钉死后端必须给真值）。camelCase 字段名逐一断言：
/// 漂移成 snake_case 时 `body["channelsEnabled"]` 等取值为 null，当场炸，
/// 而不是统计卡静默归零。共享 e2e 库里各计数随其他用例波动，只钉类型不钉值。
#[tokio::test]
async fn dashboard_summary_fields_envelope() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    // 无 token：401（admin 白名单，非公开端点）。
    let anon = anon_get(&app, "/api/dashboard").await;
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    let resp = call(&app, "GET", "/api/dashboard", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "dashboard: {resp:?}");
    let body = response_to_json(resp).await;

    // 既有 9 字段（users/tokens/channels/channelsEnabled/groups/quotaToday/
    // requestsToday/rpm/tpm）逐一按 camelCase 钉类型。
    for key in [
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
            body[key].is_i64(),
            "dashboard.{key} 必须存在且为数值（camelCase）: {body}"
        );
    }
    // W1 新增：剩余可用额度（SUM(quota) FROM auth_users WHERE status=1）。
    assert!(
        body["quotaRemaining"].is_i64(),
        "dashboard.quotaRemaining 必须存在且为 i64（W1 新增）: {body}"
    );
    let as_of = body["asOf"]
        .as_str()
        .unwrap_or_else(|| panic!("dashboard.asOf 必须是字符串: {body}"));
    assert!(!as_of.is_empty(), "asOf 不得为空串（新鲜度原则 7）");
    chrono::DateTime::parse_from_rfc3339(as_of)
        .expect("asOf 必须是 RFC3339（前端 as_of_local_time 靠它解析）");
}

/// 前端「消耗榜（总览 Top10 / 排行榜 Tokens 榜）」契约：
/// `GET /api/log/top?by=model&start=<窗起点>&limit=10` admin 白名单（无 token
/// → 401）；200 `{"items":[...]}` 信封（前端 `Items<T>` 剥壳），行 camelCase
/// `name`/`tokens`/`tokens`/`quota`/`calls`/`previousTokens`——previousTokens
/// 是 W1 的环比字段，前端虽有 serde(default) 兜底，后端缺席会让「↑new/↑%」
/// 徽标静默消失，这里从 wire 层钉死必须回传。
///
/// 写侧走 `UsageEvent::consume` 构造器（log_type=2 常量同源）。tokens 取秒级
/// 时间戳量级（i32 上限内）：本用例模型在任何历史数据面前稳居 tokens 榜首，
/// LIMIT=10 截断不会把断言目标挤出榜外；时间戳随运行严格递增，跨运行不并列。
/// `by=user` 同构抽查（group 列白名单的另一分支）。
#[tokio::test]
async fn log_top_items_envelope_and_previous_tokens() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (user_key, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let anon = anon_get(&app, "/api/log/top?by=model&limit=10").await;
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    // 窗口起点先于写入：行落在 [start, now) 内 → 上窗无该实体 → previousTokens=0。
    let start = (chrono::Utc::now() - chrono::Duration::hours(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let stamp = chrono::Utc::now().timestamp() as i32;
    let model = format!("mdl-top-{}", uuid::Uuid::new_v4().simple());

    let mut e1 = observe::logs::UsageEvent::consume(user_key, &username, &model);
    e1.prompt_tokens = stamp;
    e1.completion_tokens = 1_000;
    e1.quota = 111;
    record_usage(&pool, &e1).await;
    let mut e2 = observe::logs::UsageEvent::consume(user_key, &username, &model);
    e2.prompt_tokens = stamp;
    e2.completion_tokens = 2_000;
    e2.quota = 222;
    record_usage(&pool, &e2).await;

    let resp = call(
        &app,
        "GET",
        &format!("/api/log/top?by=model&start={start}&limit=10"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "log/top by=model: {resp:?}");
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("log/top 必须是 {{\"items\":[...]}} 信封: {body}"));
    let row = items
        .iter()
        .find(|r| r["name"] == model)
        .unwrap_or_else(|| panic!("写入的消费行必须上榜（tokens 量级恒居 LIMIT 内）: {body}"));
    let expect_tokens = 2 * stamp as i64 + 3_000;
    assert_eq!(
        row["tokens"], expect_tokens,
        "tokens = Σ(prompt+completion)"
    );
    assert_eq!(row["quota"], 333, "quota 求和");
    assert_eq!(row["calls"], 2, "调用次数 = 行数");
    assert_eq!(
        row["previousTokens"], 0,
        "上窗无该模型 → 0（camelCase 字段名钉死）"
    );

    // by=user 同构抽查：group 列切到 username 白名单分支，同一批行换维度
    // 聚合（admin 用户名 uuid 隔离，只有本用例的两条行携带它）。
    let resp = call(
        &app,
        "GET",
        &format!("/api/log/top?by=user&start={start}&limit=10"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "log/top by=user: {resp:?}");
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("by=user 同样必须 items 信封: {body}"));
    let row = items
        .iter()
        .find(|r| r["name"] == username)
        .unwrap_or_else(|| panic!("按用户聚合必须看到写入的行: {body}"));
    assert_eq!(row["tokens"], expect_tokens, "同一批行换维度聚合值不变");
    assert_eq!(row["calls"], 2);
    for key in ["name", "tokens", "quota", "calls", "previousTokens"] {
        assert!(
            !row[key].is_null(),
            "UsageTopRow.{key} 不得缺席（camelCase wire 契约）: {row}"
        );
    }

    // 测后清理：本用例全部行都携带 uuid 隔离的 username，精确删除。
    sqlx::query("DELETE FROM usage_logs WHERE username = $1")
        .bind(&username)
        .execute(&pool)
        .await
        .expect("cleanup log/top rows");
}

/// 前端「总览页趋势直方图」契约：`GET /api/log/trend?granularity=hour&start=
/// <ISO>&end=<ISO>` admin 白名单（无 token → 401）；200 `{"items":[...]}` 信封
/// （前端 `Items<T>` 剥壳——不是裸数组），行 camelCase `bucket`/`modelName`/
/// `tokens`/`quota`/`calls`，bucket 必须是可解析 RFC3339（前端 pivot_trend
/// 靠它定位时间桶）。trend 聚合无 LIMIT，uuid 隔离模型不受榜截断影响；
/// 跨小时边界时同一模型可能拆成两个桶行，断言按该模型全部行求和。
#[tokio::test]
async fn log_trend_items_envelope_bucket_rows() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (user_key, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let anon = anon_get(&app, "/api/log/trend?granularity=hour").await;
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    let model = format!("mdl-trend-{}", uuid::Uuid::new_v4().simple());
    let mut e1 = observe::logs::UsageEvent::consume(user_key, &username, &model);
    e1.prompt_tokens = 300;
    e1.completion_tokens = 60;
    e1.quota = 40;
    record_usage(&pool, &e1).await;
    let mut e2 = observe::logs::UsageEvent::consume(user_key, &username, &model);
    e2.prompt_tokens = 500;
    e2.completion_tokens = 40;
    e2.quota = 60;
    record_usage(&pool, &e2).await;

    // start 先于写入、end 留一小时余量：trend 是 created_at >= start AND
    // created_at < end，前端同款会传 iso_utc_now() 作 end——写入与请求之间
    // 跨整点会抖动丢行，这里加余量防 flake（参数绑定路径照常被钉住）。
    let start = (chrono::Utc::now() - chrono::Duration::hours(2))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let end = (chrono::Utc::now() + chrono::Duration::hours(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let resp = call(
        &app,
        "GET",
        &format!("/api/log/trend?granularity=hour&start={start}&end={end}"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "log/trend: {resp:?}");
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("log/trend 必须是 {{\"items\":[...]}} 信封: {body}"));
    let rows: Vec<&Value> = items.iter().filter(|r| r["modelName"] == model).collect();
    assert!(
        !rows.is_empty(),
        "窗口内的消费行必须出现在趋势桶（log_type=2 口径）: {body}"
    );
    let tokens: i64 = rows.iter().filter_map(|r| r["tokens"].as_i64()).sum();
    let quota: i64 = rows.iter().filter_map(|r| r["quota"].as_i64()).sum();
    let calls: i64 = rows.iter().filter_map(|r| r["calls"].as_i64()).sum();
    assert_eq!(tokens, 900, "tokens = Σ(prompt+completion)，跨桶行求和");
    assert_eq!(quota, 100);
    assert_eq!(calls, 2);
    for r in &rows {
        let bucket = r["bucket"]
            .as_str()
            .unwrap_or_else(|| panic!("bucket 必须是字符串（camelCase）: {r}"));
        chrono::DateTime::parse_from_rfc3339(bucket)
            .expect("bucket 必须是 RFC3339（前端 pivot_trend 靠它定位时间桶）");
    }

    cleanup_usage_by_model(&pool, &[&model]).await;
}

/// 前端「总览页错误榜」契约：`GET /api/log/errors?hours=24&limit=10` admin
/// 白名单（无 token → 401）；200 `{"items":[...],"asOf"}` 双字段信封（contract
/// `UsageErrorStatPage` 本体，前端整体反序列化不再剥壳），行 camelCase
/// `modelName`/`count`/`lastSeenAt`。
///
/// 口径闸：写 3 条 `UsageEvent::error`（log_type=5 常量同源）+ 1 条同模型
/// `UsageEvent::consume`（log_type=2，top/trend 的口径）→ count 必须是 3。
/// 若读侧过滤漂移把消费行也计入（count=4），当场炸——历史事故里 log_type
/// 写/读两侧漂移曾让报表整体静默变空（见 usage_log_type.rs 文件头）。
#[tokio::test]
async fn log_errors_items_asof_envelope_and_type_filter() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (user_key, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let anon = anon_get(&app, "/api/log/errors?hours=24&limit=10").await;
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    let model = format!("mdl-err-{}", uuid::Uuid::new_v4().simple());
    for _ in 0..3 {
        record_usage(
            &pool,
            &observe::logs::UsageEvent::error(user_key, &username, &model),
        )
        .await;
    }
    // 同模型的消费行（log_type=2）：只该进 top/trend，不得计入错误榜。
    let mut ok = observe::logs::UsageEvent::consume(user_key, &username, &model);
    ok.prompt_tokens = 10;
    record_usage(&pool, &ok).await;

    let resp = call(
        &app,
        "GET",
        "/api/log/errors?hours=24&limit=10",
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "log/errors: {resp:?}");
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("log/errors 必须是 {{\"items\":[...]}} 信封: {body}"));
    let as_of = body["asOf"]
        .as_str()
        .unwrap_or_else(|| panic!("log/errors 必须带 asOf 双字段信封: {body}"));
    assert!(!as_of.is_empty(), "asOf 非空（新鲜度原则 7）");
    chrono::DateTime::parse_from_rfc3339(as_of).expect("asOf 必须是 RFC3339");

    let row = items
        .iter()
        .find(|r| r["modelName"] == model)
        .unwrap_or_else(|| panic!("错误行必须入榜（uuid 隔离 + 独占 log_type=5）: {body}"));
    assert_eq!(
        row["count"], 3,
        "3 条 log_type=5 聚合为 3；若为 4 说明消费行（log_type=2）漏进错误口径"
    );
    let last_seen = row["lastSeenAt"]
        .as_str()
        .unwrap_or_else(|| panic!("lastSeenAt 必须是 camelCase 字符串: {row}"));
    chrono::DateTime::parse_from_rfc3339(last_seen)
        .expect("lastSeenAt 必须是 RFC3339（前端 last_seen_local_time 靠它解析）");

    cleanup_usage_by_model(&pool, &[&model]).await;
}

/// 前端「总览页渠道健康面板」契约：`GET /api/monitor?days=7` admin 白名单
/// （无 token → 401）；200 `{"items":[...]}` 信封——空数据与有数据都不得变
/// 404/错误（面板三态里的「无数据灰」靠空数组落地）。行 camelCase
/// `channelKey`/`days`/`total`/`okCount`/`availability`/`avgLatencyMs`
/// （前端 ChannelAvailability 反序列化形状）。写侧走 observe::monitor 的
/// `record_probe` 既有构造器，聚合断言顺带覆盖 audit §2.3-2 记录的
/// `FILTER (WHERE ok)` 零测试缺口。
#[tokio::test]
async fn monitor_all_items_envelope_and_availability_row() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let anon = anon_get(&app, "/api/monitor?days=7").await;
    assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);

    // 空数据半边：聚合端点对空 monitor_history 返回 200 + items 数组。
    let resp = call(&app, "GET", "/api/monitor?days=7", &token, None).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "monitor 空数据半边: {resp:?}"
    );
    let body = response_to_json(resp).await;
    assert!(body["items"].is_array(), "monitor 必须 items 信封: {body}");

    // 有行半边：落一次成功探活 → 该渠道聚合出 total=1/okCount=1/availability=1。
    let ch_key = uuid::Uuid::new_v4();
    let outcome = observe::monitor::ProbeOutcome {
        channel_key: ch_key,
        channel_name: format!("ch-mon-{}", &ch_key.to_string()[..8]),
        model: "gpt-4o-mini".into(),
        ok: true,
        status_code: Some(200),
        latency_ms: 42,
        error_kind: String::new(),
        message: "wire-it".into(),
    };
    observe::monitor::record_probe(&pool, &outcome)
        .await
        .expect("record probe");

    let resp = call(&app, "GET", "/api/monitor?days=7", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "monitor 有行半边: {resp:?}");
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("monitor 必须 items 信封: {body}"));
    let row = items
        .iter()
        .find(|r| r["channelKey"] == ch_key.to_string())
        .unwrap_or_else(|| panic!("写入探活记录的渠道必须出现在可用率一览: {body}"));
    assert_eq!(row["days"], 7, "days 查询参数透传到行");
    assert_eq!(row["total"], 1);
    assert_eq!(row["okCount"], 1, "FILTER (WHERE ok) 聚合");
    assert_eq!(row["availability"], 1.0, "成功探活 → 可用率 1.0");
    assert_eq!(row["avgLatencyMs"], 42.0, "成功样本平均延迟");

    // 测后清理：按 uuid 精确删本用例的探活行（monitor_history 平表无 FK）。
    sqlx::query("DELETE FROM monitor_history WHERE channel_key = $1")
        .bind(ch_key)
        .execute(&pool)
        .await
        .expect("cleanup monitor_history");
}
