//! E2E: 用户页 wire 契约 —— 管理端用户列表/管理操作（/api/user/users*）、
//! 密钥 CRUD（/api/token）、登录会话（/api/user/self/sessions）打真 Router + 真 PG，
//! 按前端（admin-page-users / admin-page-account 的 keys·sessions 面板）实际发出的
//! 请求钉死响应形状与后端行为。
//!
//! 历史事故（本文件存在的理由）：
//! - `crates/web/admin-page-users/src/panel.rs` 把用户卡文案常量「启用/禁用」直接当
//!   action 值发给 `POST /api/user/users/manage`；后端 `ManageUserAction`
//!   （crates/api/auth/src/routes.rs，serde snake_case）只认
//!   `enable|disable|set_role|adjust_quota|reset_password`，中文值在 axum Json
//!   提取层反序列化失败 → 422，管理员点启停永远失败。此前用户域端点零 wire
//!   覆盖，该 422 全绿落地（page-audit-users §2.4-1/§3.1，P0）。
//!   `user_manage_rejects_invalid_action_values` 从两侧同时钉死：合法 action
//!   逐个落库生效（前置用例）+ 非法值（含「启用/禁用」真实事故载荷）必须被拒——
//!   前端修接线后若再发错值，或后端枚举漂移，这里当场红。
//! - `/api/token` 一次性明文语义（明文仅创建响应出现一次，之后任何端点只回掩码
//!   preview）与 PUT minimal-diff（缺席字段 = 保留现值，不静默清零）此前只有
//!   admin_gateway_flow 的 insert_token 夹具侧面覆盖，无生命周期测试。
//! - `/api/user/self/sessions` 的吊销语义（吊销会话 = 吊销对应 refresh token；
//!   access token 是无状态 JWT，不受会话表拦截）此前无任何集成测试。
//!
//! 模式与 web_wire_contract.rs 一致：PG 不可达即 skip；`build_app_with_egress`
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

/// reset_password 动作使用的新口令（须过 validate_password 8..=128 字节策略）。
const RESET_PASSWORD: &str = "reset_password_456";

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
async fn build_test_app(pool: &sqlx::PgPool) -> axum::Router {
    let _guard = DDL_LOCK.lock().await;
    ensure_jwt_secret();
    api::build_app_with_egress(pool.clone(), Arc::new(MockEgress))
        .await
        .expect("build_app_with_egress")
}

/// 进程级一次性注入 JWT secret（同 web_wire_contract：Once + DDL_LOCK 串行化，
/// 已由外部设置时不覆盖）。
fn ensure_jwt_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var("FERRITE_JWT_SECRET").is_err() {
            // SAFETY: 仅此一次写入，发生在所有 build_test_app 之前（Once +
            // DDL_LOCK 串行化），此后全程只读。
            unsafe {
                std::env::set_var(
                    "FERRITE_JWT_SECRET",
                    "ferrite-e2e-users-wire-contract-secret-2026",
                );
            }
        }
    });
}

/// 读取 app 实际使用的 JWT secret：外部已设置 FERRITE_JWT_SECRET 时
/// ensure_jwt_secret 不覆盖，因此运行时读 env 而非硬编码常量，
/// 保证 claims 解码密钥与签发密钥同源。
fn jwt_secret_vec() -> Vec<u8> {
    ensure_jwt_secret();
    std::env::var("FERRITE_JWT_SECRET")
        .expect("FERRITE_JWT_SECRET set by ensure_jwt_secret")
        .into_bytes()
}

/// 解出 access token 的 sid claim（会话定位；current 标注断言用）。
fn access_sid(access: &str) -> String {
    auth::jwt::parse(&jwt_secret_vec(), access)
        .expect("access token must parse with the app's JWT secret")
        .sid
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

/// login / refresh / register 共用：请求体 JSON POST + ConnectInfo extension
/// （login/refresh 的 UA/IP 提取需要它，oneshot 裸 Router 不自带必须手动塞）。
async fn post_with_connect_info(
    app: &axum::Router,
    uri: &str,
    body: Value,
) -> axum::response::Response {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:4242".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    ServiceExt::oneshot(app.clone(), req).await.unwrap()
}

/// 走真实注册端点建普通用户（role=1、quota=0、group=default），返回 user key。
async fn register_user(app: &axum::Router, username: &str) -> uuid::Uuid {
    let resp = post_with_connect_info(
        app,
        "/api/user/register",
        json!({"username": username, "password": TEST_PASSWORD}),
    )
    .await;
    let status = resp.status();
    let body = response_to_json(resp).await;
    assert_eq!(status, StatusCode::OK, "register {username}: {body}");
    uuid::Uuid::parse_str(body["key"].as_str().expect("register returns UserView.key"))
        .expect("UserView.key is a UUID string")
}

/// 登录拿 (accessToken, refreshToken)。refreshToken 是会话吊销语义的必要凭证
/// （web_wire_contract 的 login 夹具只取 accessToken，这里取全量）。
async fn login_full(app: &axum::Router, username: &str) -> (String, String) {
    let resp = post_with_connect_info(
        app,
        "/api/user/login",
        json!({"username": username, "password": TEST_PASSWORD}),
    )
    .await;
    let status = resp.status();
    let body = response_to_json(resp).await;
    assert_eq!(status, StatusCode::OK, "login {username}: {body}");
    let access = body["accessToken"]
        .as_str()
        .unwrap_or_else(|| panic!("login missing accessToken: {body}"))
        .to_string();
    let refresh = body["refreshToken"]
        .as_str()
        .unwrap_or_else(|| panic!("login missing refreshToken: {body}"))
        .to_string();
    (access, refresh)
}

/// POST /api/user/refresh —— 会话吊销/轮换语义断言用，返回原响应。
async fn refresh_call(app: &axum::Router, refresh_token: &str) -> axum::response::Response {
    post_with_connect_info(
        app,
        "/api/user/refresh",
        json!({"refreshToken": refresh_token}),
    )
    .await
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

/// 读 auth_users 单列（落库效果断言用）。
async fn user_column(pool: &sqlx::PgPool, column: &str, key: uuid::Uuid) -> i64 {
    // 列名由本文件内字面量传入（非用户输入），白名单断言防呆。
    assert!(
        matches!(column, "status" | "role" | "quota"),
        "unexpected column {column}"
    );
    let sql = format!("SELECT {column} FROM auth_users WHERE key = $1");
    sqlx::query_scalar::<_, i64>(&sql)
        .bind(key)
        .fetch_one(pool)
        .await
        .unwrap()
}

// ============================================================================
// 用例：管理端用户列表 GET /api/user/users
// ============================================================================

/// 管理端用户列表契约：`{items, total}` 信封、UserView camelCase 投影、
/// `?search=` 下推后端（ILIKE username/email/display_name）、page/size 切片且
/// total 不随分页变化、page=0/size=0 clamp 到 1 而非报错。前端 UsersPanel 曾
/// 完全不传 search/page/size（纯前端过滤首屏 20 条），此测试钉住后端已备能力。
#[tokio::test]
async fn user_list_envelope_search_pagination() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin_key, admin_name) = insert_admin_user(&pool).await;
    let token = login_full(&app, &admin_name).await.0;

    // 三个带唯一前缀的用户：prefix 由 uuid 派生，与其他测试/历史数据隔离，
    // 因此 search=prefix 的 total 可以精确断言为 3。
    let prefix = format!("wlst{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let usernames: Vec<String> = ["_a", "_b", "_c"]
        .iter()
        .map(|s| format!("{prefix}{s}"))
        .collect();
    let mut user_keys = Vec::new();
    for name in &usernames {
        user_keys.push(register_user(&app, name).await);
    }
    let all_keys: Vec<String> = user_keys.iter().map(|k| k.to_string()).collect();

    // 无 search 的裸列表：信封 + total ≥ 3（共享库中可能还有其他用户）。
    let resp = call(&app, "GET", "/api/user/users", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "user list must be 200");
    let body = response_to_json(resp).await;
    let items = body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("user list must wrap items: {body}"));
    assert!(
        body["total"].is_i64() && body["total"].as_i64().unwrap() >= 3,
        "user list must carry total >= 3: {body}"
    );

    // UserView camelCase 投影（前端 AdminUserDto 按这些字段解码）。
    let row = items
        .iter()
        .find(|r| r["username"] == usernames[0])
        .unwrap_or_else(|| panic!("registered user must be listed: {body}"));
    assert_eq!(
        row["displayName"], usernames[0],
        "注册时 display_name=username"
    );
    assert_eq!(row["email"], "", "无 email 注册 → 空串（UserView 兜底）");
    assert_eq!(row["role"], 1, "注册端点恒建普通用户");
    assert_eq!(row["status"], 1);
    assert_eq!(row["quota"], 0);
    assert_eq!(row["usedQuota"], 0);
    assert_eq!(row["group"], "default");
    assert_eq!(row["authVersion"], 1);
    assert!(row["createdAt"].is_string(), "createdAt RFC3339 字符串");
    assert!(
        row.get("passwordHash").is_none() && row.get("password_hash").is_none(),
        "UserView 不得泄漏 password_hash: {row}"
    );

    // search 精确命中：total == 3 且全部行都属于本用例数据。
    let resp = call(
        &app,
        "GET",
        &format!("/api/user/users?search={prefix}"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_to_json(resp).await;
    assert_eq!(body["total"], 3, "search=prefix 必须精确命中 3 条: {body}");
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert!(
        items
            .iter()
            .all(|r| r["username"].as_str().unwrap().starts_with(&prefix)),
        "search 结果必须全部匹配前缀: {items:?}"
    );

    // 分页切片：page/size 下推后端，total 恒定。
    let resp = call(
        &app,
        "GET",
        &format!("/api/user/users?search={prefix}&page=1&size=2"),
        &token,
        None,
    )
    .await;
    let page1 = response_to_json(resp).await;
    assert_eq!(page1["total"], 3, "total 不随分页变化");
    let page1_keys: Vec<String> = page1["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(page1_keys.len(), 2);

    let resp = call(
        &app,
        "GET",
        &format!("/api/user/users?search={prefix}&page=2&size=2"),
        &token,
        None,
    )
    .await;
    let page2 = response_to_json(resp).await;
    assert_eq!(page2["total"], 3);
    let page2_keys: Vec<String> = page2["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(page2_keys.len(), 1, "3 条数据 size=2 时第 2 页只剩 1 条");
    // 两页并集 = 全量、交集为空（切片语义，不是重复窗口）。
    let mut union = page1_keys.clone();
    union.extend(page2_keys.clone());
    union.sort();
    let mut expected = all_keys.clone();
    expected.sort();
    assert_eq!(union, expected, "两页并集必须等于全部 3 个用户");
    assert!(
        page1_keys.iter().all(|k| !page2_keys.contains(k)),
        "两页不得重叠"
    );

    // 非法分页参数 clamp 而非报错：page=0 → 1（service page.max(1)）。
    let resp = call(
        &app,
        "GET",
        &format!("/api/user/users?search={prefix}&page=0&size=2"),
        &token,
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "page=0 必须 clamp 到 1 而非报错"
    );
    let body = response_to_json(resp).await;
    assert_eq!(
        body["items"].as_array().unwrap().len(),
        2,
        "page=0 等价 page=1"
    );

    // size=0 clamp 到 1（service size.clamp(1,100)）。
    let resp = call(
        &app,
        "GET",
        &format!("/api/user/users?search={prefix}&size=0"),
        &token,
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "size=0 必须 clamp 到 1 而非报错"
    );
    let body = response_to_json(resp).await;
    assert_eq!(
        body["items"].as_array().unwrap().len(),
        1,
        "size=0 clamp 到 LIMIT 1"
    );
    assert_eq!(body["total"], 3, "total 不受 size clamp 影响");

    // 守卫：非 admin 拿不到列表（403）；无 token 401。
    let plain = format!(
        "wlstplain{}",
        &uuid::Uuid::new_v4().simple().to_string()[..6]
    );
    register_user(&app, &plain).await;
    let plain_token = login_full(&app, &plain).await.0;
    let resp = call(&app, "GET", "/api/user/users", &plain_token, None).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "非 admin 必须 403");
    let anon = Request::builder()
        .uri("/api/user/users")
        .body(Body::empty())
        .unwrap();
    let resp = ServiceExt::oneshot(app.clone(), anon).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "无 token 必须 401");
}

// ============================================================================
// 用例：POST /api/user/users/manage —— 合法 action 落库效果
// ============================================================================

/// action=enable/disable 落库契约：status 1↔2 写 auth_users、disable 硬删该用户
/// 全部 refresh token（旧 refresh 立即 401）、被禁用户 access token 打 self 得
/// 403（UserDisabled）、重新 enable 后旧 access token 无需重登即恢复 200。
#[tokio::test]
async fn user_manage_enable_disable_persists_and_revokes() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin_key, admin_name) = insert_admin_user(&pool).await;
    let admin_token = login_full(&app, &admin_name).await.0;

    let victim = format!("mvct{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let victim_key = register_user(&app, &victim).await;
    let (victim_access, victim_refresh) = login_full(&app, &victim).await;

    // disable → 200，响应即更新后的 UserView（status=2）。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "disable"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "disable must succeed");
    let body = response_to_json(resp).await;
    assert_eq!(body["status"], 2, "disable 响应必须回写 status=2: {body}");
    assert_eq!(body["key"], victim_key.to_string());

    // 落库效果：auth_users.status=2，refresh token 全删。
    assert_eq!(user_column(&pool, "status", victim_key).await, 2);
    let refresh_left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM auth_refresh_tokens WHERE user_key = $1")
            .bind(victim_key)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(refresh_left, 0, "disable 必须硬删该用户全部 refresh token");

    // 被禁用户的 access token：self → 403（UserDisabled）。
    let resp = call(&app, "GET", "/api/user/self", &victim_access, None).await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "被禁用户 self 必须 403"
    );

    // 被禁用户的 refresh token：refresh → 401（已随 disable 删除）。
    let resp = refresh_call(&app, &victim_refresh).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "被禁用户 refresh 必须 401"
    );

    // enable → 200，status 回 1；旧 access token 无需重登即恢复（auth_version 未变）。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "enable"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "enable must succeed");
    let body = response_to_json(resp).await;
    assert_eq!(body["status"], 1, "enable 响应必须回写 status=1: {body}");
    assert_eq!(user_column(&pool, "status", victim_key).await, 1);
    let resp = call(&app, "GET", "/api/user/self", &victim_access, None).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "重新启用后旧 access token 必须恢复可用（auth_version 未 bump）"
    );
}

/// action=set_role 与 adjust_quota 落库契约：role 只接受 1|10|100（其余 400）；
/// adjust_quota 是 delta 语义（GREATEST(0, quota+delta)，负向扣减、地板 0），
/// 非整数 delta 400。
#[tokio::test]
async fn user_manage_set_role_and_adjust_quota_delta() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin_key, admin_name) = insert_admin_user(&pool).await;
    let admin_token = login_full(&app, &admin_name).await.0;

    let victim = format!("mrole{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let victim_key = register_user(&app, &victim).await;

    // set_role：合法值 {1,10,100}，value 是字符串化的数字。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "set_role", "value": "10"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "set_role 10 must succeed");
    let body = response_to_json(resp).await;
    assert_eq!(body["role"], 10, "响应回写新 role: {body}");
    assert_eq!(
        user_column(&pool, "role", victim_key).await,
        10,
        "落库 role=10"
    );

    // set_role 非法值：不在 {1,10,100} → 400；非数字 → 400。
    for bad in ["7", "abc"] {
        let resp = call(
            &app,
            "POST",
            "/api/user/users/manage",
            &admin_token,
            Some(json!({"key": victim_key, "action": "set_role", "value": bad})),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "set_role value={bad} 必须 400"
        );
    }
    assert_eq!(
        user_column(&pool, "role", victim_key).await,
        10,
        "被拒的 set_role 不得改动落库值"
    );

    // adjust_quota：delta 语义。注册用户 quota=0 → +123456 → 123456。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "adjust_quota", "value": "123456"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "adjust_quota must succeed");
    let body = response_to_json(resp).await;
    assert_eq!(body["quota"], 123456, "响应回写调整后额度: {body}");
    assert_eq!(user_column(&pool, "quota", victim_key).await, 123456);

    // 负 delta 是扣减不是覆盖：123456 + (-500) = 122956。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "adjust_quota", "value": "-500"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(user_column(&pool, "quota", victim_key).await, 122956);

    // 地板语义：扣到负数 → GREATEST(0, ...) = 0，不会出现负额度。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "adjust_quota", "value": "-999999999"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        user_column(&pool, "quota", victim_key).await,
        0,
        "额度地板必须是 0（GREATEST(0, quota+delta)）"
    );

    // 非整数 delta → 400。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "adjust_quota", "value": "abc"})),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "非整数 delta 必须 400"
    );
}

/// action=reset_password 契约：写新 password_hash 且 auth_version++（旧 access
/// token 因 claims.auth_version 落后立即 401）、硬删全部 refresh token（旧
/// refresh 401）、新口令可登录、旧口令登录 401。
#[tokio::test]
async fn user_manage_reset_password_invalidates_credentials() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin_key, admin_name) = insert_admin_user(&pool).await;
    let admin_token = login_full(&app, &admin_name).await.0;

    let victim = format!("mrst{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let victim_key = register_user(&app, &victim).await;
    let (victim_access, victim_refresh) = login_full(&app, &victim).await;

    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "reset_password", "value": RESET_PASSWORD})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "reset_password must succeed");
    let body = response_to_json(resp).await;
    assert_eq!(
        body["authVersion"], 2,
        "reset_password 必须 auth_version++（注册时为 1）: {body}"
    );

    // 旧 access token：auth_version 落后 → 401（不是 403）。
    let resp = call(&app, "GET", "/api/user/self", &victim_access, None).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "重置密码后旧 access token 必须 401（auth_version 校验）"
    );

    // 旧 refresh token：已随重置删除 → 401。
    let resp = refresh_call(&app, &victim_refresh).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "旧 refresh 必须 401"
    );

    // 旧口令登录 401、新口令登录 200。
    let resp = post_with_connect_info(
        &app,
        "/api/user/login",
        json!({"username": victim, "password": TEST_PASSWORD}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "旧口令必须 401");
    let (new_access, _) = login_full_with_password(&app, &victim, RESET_PASSWORD).await;

    // 新口令签发的会话可用。
    let resp = call(&app, "GET", "/api/user/self", &new_access, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "新口令登录后的会话必须可用");

    // reset_password 缺 value → 400（service 层显式校验）。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": victim_key, "action": "reset_password"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "缺 value 必须 400");
}

/// login_full 的带口令变体（reset_password 后用新口令登录）。
async fn login_full_with_password(
    app: &axum::Router,
    username: &str,
    password: &str,
) -> (String, String) {
    let resp = post_with_connect_info(
        app,
        "/api/user/login",
        json!({"username": username, "password": password}),
    )
    .await;
    let status = resp.status();
    let body = response_to_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "login {username} with new password: {body}"
    );
    let access = body["accessToken"]
        .as_str()
        .expect("accessToken")
        .to_string();
    let refresh = body["refreshToken"]
        .as_str()
        .expect("refreshToken")
        .to_string();
    (access, refresh)
}

// ============================================================================
// 用例：POST /api/user/users/manage —— 非法 action / 非法 key 拒绝矩阵
//（防回归核心：panel.rs 中文 action 事故的 wire 层闸门）
// ============================================================================

/// 非法 action 值必须在 Json 提取层被拒（axum 0.8 反序列化失败 → 422），
/// 且**不得产生任何落库副作用**。覆盖：
/// - 「启用」「禁用」：panel.rs L483-485 真实事故载荷（文案常量当协议值）；
/// - "delete"：看似合理但枚举未注册的英文值（服务层 catch-all 400 在 wire 层
///   不可达——serde 枚举先拒，钉 422 防止有人误改枚举为宽松匹配）；
/// - 空串。
/// 另钉：合法 action + 非法 UUID key → 400；合法 action + 不存在用户 → 404；
/// 非 admin 调 manage → 403。
#[tokio::test]
async fn user_manage_rejects_invalid_action_values() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin_key, admin_name) = insert_admin_user(&pool).await;
    let admin_token = login_full(&app, &admin_name).await.0;

    let victim = format!("mbad{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let victim_key = register_user(&app, &victim).await;

    // 非法 action 全家族 → 422（合法 JSON、未知枚举值 = JsonDataError 422）。
    for action in ["禁用", "启用", "delete", ""] {
        let resp = call(
            &app,
            "POST",
            "/api/user/users/manage",
            &admin_token,
            Some(json!({"key": victim_key, "action": action})),
        )
        .await;
        let status = resp.status();
        let body = response_to_json(resp).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "非法 action {action:?} 必须被 Json 提取层 422 拒绝（panel.rs 中文 action 事故闸门）: {body}"
        );
    }

    // 合法 action + 非法 UUID key → 400（handler 显式 parse 校验）。
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": "not-a-uuid", "action": "enable"})),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "非法 UUID key 必须 400"
    );

    // 合法 action + 不存在的用户 key → 404（service UserNotFound）。
    let ghost = uuid::Uuid::new_v4();
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &admin_token,
        Some(json!({"key": ghost, "action": "enable"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "不存在的用户必须 404");
    let body = response_to_json(resp).await;
    assert_eq!(body["code"], "USER_NOT_FOUND", "错误码语义: {body}");

    // 非 admin（role=1）调 manage：合法载荷 → 403 FORBIDDEN。
    let plain = format!("mplain{}", &uuid::Uuid::new_v4().simple().to_string()[..6]);
    register_user(&app, &plain).await;
    let plain_token = login_full(&app, &plain).await.0;
    let resp = call(
        &app,
        "POST",
        "/api/user/users/manage",
        &plain_token,
        Some(json!({"key": victim_key, "action": "disable"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "非 admin 必须 403");

    // 副作用守卫：以上所有被拒请求不得改动 victim 的落库状态。
    assert_eq!(
        user_column(&pool, "status", victim_key).await,
        1,
        "被拒请求不得产生落库副作用（status 仍为 1）"
    );
    assert_eq!(
        user_column(&pool, "quota", victim_key).await,
        0,
        "被拒请求不得产生落库副作用（quota 仍为 0）"
    );
}

// ============================================================================
// 用例：/api/token 密钥 CRUD 生命周期
// ============================================================================

/// 密钥 CRUD 契约（keys 面板后端半边）：
/// - POST 创建 → 一次性明文 `sk-`+64hex 只在创建响应出现一次，列表/单查永不再回；
/// - 列表 `{items}` 信封，keyPreview 是掩码 `sk-abcd****wxyz`；
/// - PUT minimal-diff：只带 name → quota/status/group/expiresAt 全保留；
///   显式字段 → 落库；status 只接受 1|2；
/// - DELETE → 列表消失、单查 404、重复 DELETE 404；空 name 创建 400。
#[tokio::test]
async fn token_crud_lifecycle_minimal_diff() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin_key, admin_name) = insert_admin_user(&pool).await;
    let token = login_full(&app, &admin_name).await.0;

    // ---- 创建：name + quota + expiresAt（group/unlimitedQuota 缺省）。----
    let name = format!("tk_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let resp = call(
        &app,
        "POST",
        "/api/token",
        &token,
        Some(json!({"name": name, "quota": 500_000, "expiresAt": "2030-01-01T00:00:00Z"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "token create: {resp:?}");
    let created = response_to_json(resp).await;

    // 一次性明文：sk- + 64 hex，只在创建响应出现。
    let plaintext = created["plaintext"]
        .as_str()
        .unwrap_or_else(|| panic!("create must return one-time plaintext: {created}"))
        .to_string();
    assert!(
        plaintext.starts_with("sk-"),
        "明文必须 sk- 前缀: {plaintext}"
    );
    assert_eq!(plaintext.len(), 67, "明文 = sk- + 64 hex（3+64）");
    assert!(
        plaintext[3..].chars().all(|c| c.is_ascii_hexdigit()),
        "明文体必须是 hex: {plaintext}"
    );

    // 创建响应的 token 视图（TokenView camelCase）。
    let view = &created["token"];
    let token_key = view["key"]
        .as_str()
        .unwrap_or_else(|| panic!("create token view must carry key: {created}"))
        .to_string();
    assert_eq!(view["name"], name);
    assert_eq!(view["quota"], 500_000, "500_000 ≈ $1（内部单位）");
    assert_eq!(view["unlimitedQuota"], false, "缺省 unlimitedQuota=false");
    assert_eq!(view["usedQuota"], 0);
    assert_eq!(view["status"], 1, "新建即启用");
    assert!(
        view["expiresAt"]
            .as_str()
            .unwrap_or("")
            .starts_with("2030-01-01"),
        "expiresAt 落库为请求的 RFC3339: {}",
        view["expiresAt"]
    );
    assert!(view["group"].is_null(), "未传 group → NULL（跟随用户组）");
    // 掩码 preview = sk-<前4>****<后4>，与明文一致且不等于明文。
    let body_hex = &plaintext[3..];
    let expected_preview = format!("sk-{}****{}", &body_hex[..4], &body_hex[60..]);
    assert_eq!(
        view["keyPreview"], expected_preview,
        "preview 必须是首4+****+尾4"
    );
    assert_ne!(view["keyPreview"], plaintext, "preview 不得等于明文");

    // 空名创建 → 400（service trim 校验）。
    let resp = call(
        &app,
        "POST",
        "/api/token",
        &token,
        Some(json!({"name": "   "})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "空白 name 必须 400");

    // ---- 列表：{items} 信封、含该 token、无明文泄漏。----
    let resp = call(&app, "GET", "/api/token", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list = response_to_json(resp).await;
    let items = list["items"]
        .as_array()
        .unwrap_or_else(|| panic!("token list must wrap items: {list}"));
    let row = items
        .iter()
        .find(|r| r["key"] == token_key)
        .unwrap_or_else(|| panic!("created token must be listed: {list}"));
    assert_eq!(row["name"], name);
    assert_eq!(row["keyPreview"], expected_preview);
    assert!(
        row.get("plaintext").is_none() || row["plaintext"].is_null(),
        "列表不得回传明文字段"
    );
    assert!(
        !list.to_string().contains(&plaintext),
        "列表响应任何位置都不得出现明文 key"
    );

    // ---- 单查：200 同形状（明文仍不出现）。----
    let resp = call(
        &app,
        "GET",
        &format!("/api/token/{token_key}"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let one = response_to_json(resp).await;
    assert_eq!(one["key"], token_key);
    assert!(!one.to_string().contains(&plaintext), "单查也不得回传明文");

    // ---- PUT minimal-diff：只带 name，其余字段全部保留。----
    let resp = call(
        &app,
        "PUT",
        &format!("/api/token/{token_key}"),
        &token,
        Some(json!({"name": format!("{name}_renamed")})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "minimal-diff put: {resp:?}");
    let updated = response_to_json(resp).await;
    assert_eq!(updated["name"], format!("{name}_renamed"), "显式 name 落库");
    assert_eq!(updated["quota"], 500_000, "quota 缺席必须保留（不得清零）");
    assert_eq!(updated["status"], 1, "status 缺席必须保留");
    assert_eq!(
        updated["unlimitedQuota"], false,
        "unlimitedQuota 缺席必须保留"
    );
    assert!(updated["group"].is_null(), "group 缺席必须保留 NULL");
    assert_eq!(
        updated["expiresAt"], view["expiresAt"],
        "expiresAt 缺席必须保留（不得清成永不过期）"
    );

    // ---- PUT 显式字段：quota/status/group 落库；name 保持。----
    let resp = call(
        &app,
        "PUT",
        &format!("/api/token/{token_key}"),
        &token,
        Some(json!({"quota": 777, "status": 2, "group": "default"})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let updated = response_to_json(resp).await;
    assert_eq!(updated["quota"], 777);
    assert_eq!(updated["status"], 2);
    assert_eq!(updated["group"], "default");
    assert_eq!(
        updated["name"],
        format!("{name}_renamed"),
        "未提及的 name 保持"
    );

    // status 越界 → 400（后端只认 1|2）。
    let resp = call(
        &app,
        "PUT",
        &format!("/api/token/{token_key}"),
        &token,
        Some(json!({"status": 5})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "status=5 必须 400");

    // ---- 删除 → 404 闭环。----
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/token/{token_key}"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "delete must succeed");
    let body = response_to_json(resp).await;
    assert_eq!(body["success"], true);

    let resp = call(
        &app,
        "GET",
        &format!("/api/token/{token_key}"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "删除后单查必须 404");

    let resp = call(&app, "GET", "/api/token", &token, None).await;
    let list = response_to_json(resp).await;
    assert!(
        !list["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["key"] == token_key),
        "删除后列表不得再包含: {list}"
    );

    let resp = call(
        &app,
        "DELETE",
        &format!("/api/token/{token_key}"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "重复 DELETE 必须 404");
}

// ============================================================================
// 用例：/api/user/self/sessions 登录会话
// ============================================================================

/// 会话列表 + 单会话吊销契约：
/// - GET /self/sessions 回**裸数组**（不是 {items} 信封——sessions 面板按数组解码）；
/// - 每项 SessionView camelCase（sid/userAgent/ip/loginMethod/createdAt/
///   lastActive/expiresAt/current），current 标注来自 access token 的 sid claim；
/// - DELETE /self/sessions/{sid} 吊销该会话并**连坐吊销对应 refresh token**
///   （被吊会话的 refresh → 401），当前会话不受影响（其 refresh 仍可轮换）；
/// - 后端对吊销「当前设备」无特殊拦截（DELETE 当前 sid 也成功）——这是实测
///   行为，前端确认弹窗是唯一防线（audit E-3）；
/// - 非法 sid 字符串 → 400；不存在的 sid → 404。
#[tokio::test]
async fn sessions_list_shape_and_revocation_semantics() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin_key, admin_name) = insert_admin_user(&pool).await;

    // 同一用户登录两次 → 两个会话（每次登录签发独立 sid）。
    let (access1, refresh1) = login_full(&app, &admin_name).await;
    let (access2, refresh2) = login_full(&app, &admin_name).await;
    let sid1 = access_sid(&access1);
    let sid2 = access_sid(&access2);
    assert_ne!(sid1, sid2, "每次登录必须签发新 sid");

    // 列表：裸数组、恰好 2 条、current 按携带的 access token 标注。
    let resp = call(&app, "GET", "/api/user/self/sessions", &access1, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = response_to_json(resp).await;
    let sessions = body
        .as_array()
        .unwrap_or_else(|| panic!("sessions 必须是裸数组（无 items 信封）: {body}"));
    assert_eq!(sessions.len(), 2, "两次登录恰好两个会话: {body}");

    let own = sessions
        .iter()
        .find(|s| s["sid"] == sid1)
        .unwrap_or_else(|| panic!("access1 的会话必须在列表中: {body}"));
    let other = sessions
        .iter()
        .find(|s| s["sid"] == sid2)
        .unwrap_or_else(|| panic!("access2 的会话必须在列表中: {body}"));
    assert_eq!(own["current"], true, "current 标注 access token 所属会话");
    assert_eq!(other["current"], false);
    // SessionView 字段形状（camelCase）。
    for field in [
        "userAgent",
        "ip",
        "loginMethod",
        "createdAt",
        "lastActive",
        "expiresAt",
    ] {
        assert!(
            own[field].is_string(),
            "SessionView.{field} 必须是字符串: {own}"
        );
    }
    assert_eq!(own["loginMethod"], "password", "登录方式固定 password");
    assert_eq!(own["ip"], "127.0.0.1", "ConnectInfo 注入的测试 IP");

    // 吊销另一个会话（sid2）→ 200 {"success": true}。
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/user/self/sessions/{sid2}"),
        &access1,
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "revoke other session: {resp:?}"
    );
    let body = response_to_json(resp).await;
    assert_eq!(body["success"], true);

    // 列表只剩当前会话；被吊会话消失。
    let resp = call(&app, "GET", "/api/user/self/sessions", &access1, None).await;
    let body = response_to_json(resp).await;
    let sessions = body.as_array().unwrap();
    assert_eq!(sessions.len(), 1, "被吊会话必须从列表消失: {body}");
    assert_eq!(sessions[0]["sid"], sid1);

    // 吊销连坐 refresh：被吊会话的 refresh token → 401。
    let resp = refresh_call(&app, &refresh2).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "被吊会话的 refresh token 必须失效"
    );

    // 当前会话的 refresh 未受影响：仍可轮换（200；轮换会签发新 sid，属既有语义）。
    let resp = refresh_call(&app, &refresh1).await;
    assert_eq!(resp.status(), StatusCode::OK, "未吊会话的 refresh 不受牵连");

    // 非法 sid → 400；不存在 sid → 404。
    let resp = call(
        &app,
        "DELETE",
        "/api/user/self/sessions/not-a-uuid",
        &access1,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "非法 sid 必须 400");
    let ghost = uuid::Uuid::new_v4();
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/user/self/sessions/{ghost}"),
        &access1,
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "不存在的 sid 必须 404"
    );
}

/// revoke-others 契约：POST /self/sessions/revoke-others 吊销除 access token
/// 所属会话外的全部会话（各会话 refresh 连坐失效），当前会话保留且列表只剩它。
#[tokio::test]
async fn sessions_revoke_others_keeps_only_current() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin_key, admin_name) = insert_admin_user(&pool).await;

    // access1 的会话由 revoke-others 吊销，无需保留其 access token。
    let (_access1, refresh1) = login_full(&app, &admin_name).await;
    let (access2, refresh2) = login_full(&app, &admin_name).await;
    let sid2 = access_sid(&access2);

    // access2 发起 revoke-others：sid1 被吊，sid2 保留。
    let resp = call(
        &app,
        "POST",
        "/api/user/self/sessions/revoke-others",
        &access2,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "revoke-others: {resp:?}");
    let body = response_to_json(resp).await;
    assert_eq!(body["success"], true);

    // 列表只剩当前会话（sid2，current=true）。
    let resp = call(&app, "GET", "/api/user/self/sessions", &access2, None).await;
    let body = response_to_json(resp).await;
    let sessions = body.as_array().unwrap();
    assert_eq!(sessions.len(), 1, "revoke-others 后只剩当前会话: {body}");
    assert_eq!(sessions[0]["sid"], sid2);
    assert_eq!(sessions[0]["current"], true);

    // 被吊会话的 refresh 连坐失效；当前会话的 refresh 仍可用。
    let resp = refresh_call(&app, &refresh1).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "被吊会话 refresh 必须 401"
    );
    let resp = refresh_call(&app, &refresh2).await;
    assert_eq!(resp.status(), StatusCode::OK, "当前会话 refresh 不受牵连");
}
