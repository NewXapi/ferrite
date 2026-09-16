//! E2E: 管理页写契约补洞（todo/page-audit-manage.md §2.4 测试缺口前三项）。
//!
//! 与 network_write_path.rs / web_wire_contract.rs 同模式：真 Router
//! （`build_app_with_egress`）+ tower oneshot + PG（不可达即 skip），
//! 实体全部经 wire 创建，落库断言用 sqlx 直查。本文件钉三个此前
//! 零测试覆盖的写契约：
//!
//! 1. **兑换码 status 值域**——三方矛盾已久（迁移注释只声明
//!    `1=未兑换 2=已兑换`，前端映射 `2=停用 / 3=已核销`）。本文件按
//!    **后端实测语义**钉死（crates/api/admin-billing/src/redeem.rs）：
//!    - 生成：INSERT 不带 status，列 DEFAULT 1（未兑换）；
//!    - 核销 `POST /api/user/topup`：CAS `status=1 → SET status=2`（已核销），
//!      同事务给 `user_balances(FREE)` 入账；
//!    - 停用 `DELETE /api/redemption/{key}`：CAS `status=1 → SET status=3`（已停用）。
//!
//!  前端 redemptions.rs 的映射（2=停用、3=已核销）与之**对调**，是
//!    待修的生产 bug——修前端时本文件不动，即成回归闸。
//! 2. **系统选项写路径**——`PUT /api/option`（admin-ops/options.rs）：
//!    5 个注册表 key + 类型化 validator；未知 key / 类型不符 / 越界
//!    一律 400 且不落库；合法写入可经 `GET /api/option` 与
//!    `GET /api/option/{key}` 两种读形状读回。
//! 3. **渠道 update 的 groups 空守卫缺失**——create 有守卫（空 groups
//!    400），update 没有：显式 `"groups": []` 会 200 并把落库值清成
//!    空数组 → snapshot 展开零路由单元，渠道静默永不路由。
//!    本文件按**当前真实行为**钉（known-gap）：后端补守卫后需把该
//!    用例的 200/空数组断言翻转为 400/保持。

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
        unreachable!("manage wire tests never hit the forward plane; MockEgress is a placeholder")
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
/// 仅用于本进程签发/校验测试 JWT，不接触任何真实环境。已由外部设置时不覆盖。
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
                std::env::set_var("FERRITE_JWT_SECRET", "ferrite-e2e-manage-wire-secret-2026");
            }
        }
    });
}

/// 插入 admin 用户（role=100，过 ADMIN_ROLE_THRESHOLD=10 白名单）。
/// password_hash 必须是真实 argon2 PHC，否则 login 恒失败拿不到 JWT。
async fn insert_admin_user(pool: &sqlx::PgPool) -> (uuid::Uuid, String) {
    insert_user(pool, 100).await
}

/// 插入普通用户（role=1，低于 admin 阈值）——兑换码核销
/// `POST /api/user/topup` 只要求登录（bearer_user），不要求 admin，
/// 用普通用户钉住该门禁语义。
async fn insert_user(pool: &sqlx::PgPool, role: i32) -> (uuid::Uuid, String) {
    let user_key = uuid::Uuid::new_v4();
    let username = format!("mgr_{}", &user_key.to_string()[..8]);
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

// ---------------------------------------------------------------------------
// 兑换码 helpers：明文只在生成响应出现一次，库内只有 sha256 哈希 + 脱敏
// preview；DB 侧按 code_hash 定位行（与 redeem.rs 存储口径一致），wire 侧
// 按 codePreview 定位（管理页列表 → 停用按钮的真实路径）。
// ---------------------------------------------------------------------------

fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

/// 复刻 redeem.rs `preview`：明文去 `fx-` 前缀后取前 4 + **** + 后 4。
fn code_preview(plaintext: &str) -> String {
    let body = plaintext.strip_prefix("fx-").unwrap_or(plaintext);
    format!(
        "fx-{}****{}",
        &body[..4.min(body.len())],
        &body[body.len().saturating_sub(4)..]
    )
}

/// admin 经 `POST /api/redemption` 批量生成（管理页「生成」按钮路径），
/// 返回一次性明文码列表。
async fn generate_codes_via_wire(
    app: &axum::Router,
    token: &str,
    quota: i64,
    count: u32,
) -> Vec<String> {
    let resp = call(
        app,
        "POST",
        "/api/redemption",
        token,
        Some(json!({"quota": quota, "count": count})),
    )
    .await;
    let status = resp.status();
    let body = response_to_json(resp).await;
    assert_eq!(status, StatusCode::OK, "generate codes: {body}");
    body["codes"]
        .as_array()
        .unwrap_or_else(|| panic!("codes array missing: {body}"))
        .iter()
        .map(|c| c.as_str().expect("code str").to_string())
        .collect()
}

/// 管理页停用路径：GET /api/redemption 列表按 codePreview 定位行，
/// 返回该项完整 JSON（含 key / status / redeemedBy）。
async fn find_redemption_by_preview(app: &axum::Router, token: &str, preview: &str) -> Value {
    let resp = call(app, "GET", "/api/redemption?page=1&size=100", token, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "redemption list");
    let body = response_to_json(resp).await;
    body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("items array missing: {body}"))
        .iter()
        .find(|r| r["codePreview"] == preview)
        .cloned()
        .unwrap_or_else(|| panic!("code with preview {preview} must be listed: {body}"))
}

/// DB 侧按 code_hash 读兑换码行状态列（status / redeemed_by / redeemed_at）。
async fn db_redemption_by_hash(
    pool: &sqlx::PgPool,
    plaintext: &str,
) -> (
    i16,
    Option<uuid::Uuid>,
    Option<chrono::DateTime<chrono::Utc>>,
) {
    sqlx::query_as(
        "SELECT status, redeemed_by, redeemed_at FROM billing_redemptions WHERE code_hash = $1",
    )
    .bind(sha256_hex(plaintext))
    .fetch_one(pool)
    .await
    .unwrap()
}

/// 测试结束清理自建兑换码行（e2e 库跨运行复用，按 key 精确删除）。
async fn cleanup_redemptions(pool: &sqlx::PgPool, keys: &[uuid::Uuid]) {
    if keys.is_empty() {
        return;
    }
    sqlx::query("DELETE FROM billing_redemptions WHERE key = ANY($1)")
        .bind(keys)
        .execute(pool)
        .await
        .unwrap();
}

/// 读用户某货币余额（user_balances.amount，该货币自己的单位）。
async fn db_user_balance(pool: &sqlx::PgPool, user_key: uuid::Uuid, currency: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = $2",
    )
    .bind(user_key)
    .bind(currency)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// 读渠道落库 groups（JSONB 字符串数组）。
async fn db_channel_groups(pool: &sqlx::PgPool, key: &str) -> Vec<String> {
    let (raw,): (Value,) = sqlx::query_as("SELECT groups FROM api_channels WHERE key = $1")
        .bind(uuid::Uuid::parse_str(key).unwrap())
        .fetch_one(pool)
        .await
        .unwrap();
    serde_json::from_value(raw).unwrap()
}

/// 经 `POST /api/channel` 创建渠道（全量 `CreateChannelRequest` 形状），
/// 返回 key。
async fn create_channel_via_wire(
    app: &axum::Router,
    token: &str,
    name: &str,
    groups: &[&str],
) -> String {
    let body = json!({
        "name": name,
        "channelType": "openai",
        "baseUrl": "http://mock-manage-wire",
        "keys": ["sk-manage-1"],
        "models": [],
        "groups": groups,
        "priority": 0,
        "weight": 0,
        "testModel": null,
        "remark": ""
    });
    let resp = call(app, "POST", "/api/channel", token, Some(body)).await;
    let status = resp.status();
    let created = response_to_json(resp).await;
    assert_eq!(status, StatusCode::OK, "channel create: {created}");
    created["key"]
        .as_str()
        .unwrap_or_else(|| panic!("created channel must carry key: {created}"))
        .to_string()
}

/// 本文件各测试用互不相同的随机码额，e2e 库跨运行残留行不会与
/// 列表定位（quota + preview 双匹配）撞车。
fn unique_quota() -> i64 {
    700_000 + (uuid::Uuid::new_v4().as_u128() % 90_000) as i64
}

// ============================================================================
// 用例 1：兑换码 status 全生命周期（生成 1 → 核销 2 → 停用 3）
// ============================================================================

/// 钉死兑换码 status 的**后端实测值域**（此前三方矛盾的仲裁基准）：
/// - 生成后 DB status=1（列 DEFAULT，INSERT 不带 status）；
/// - 用户 `POST /api/user/topup` 核销 → DB status=2，redeemed_by/redeemed_at
///   落值，`user_balances(FREE)` 恰好 +码额；wire 列表同帧可见 status=2；
/// - admin `DELETE /api/redemption/{key}` 停用 → DB status=3；
/// - 停用后的码再核销 → 404（CAS 只认 status=1）。
/// 前端 redemptions.rs 把 2 画成「已停用」、3 画成「已核销」，与该值域
/// 对调——前端修复以本用例为准。
#[tokio::test]
async fn redemption_status_lifecycle_generate_redeem_disable() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin, admin_name) = insert_admin_user(&pool).await;
    let admin_token = login(&app, &admin_name).await;
    let (user_key, user_name) = insert_user(&pool, 1).await;
    let user_token = login(&app, &user_name).await;

    let quota = unique_quota();
    let codes = generate_codes_via_wire(&app, &admin_token, quota, 2).await;
    assert_eq!(codes.len(), 2, "count=2 应生成两张码");
    assert!(
        codes
            .iter()
            .all(|c| c.starts_with("fx-") && c.len() == 3 + 32),
        "明文码形状 fx-+32hex: {codes:?}"
    );
    let mut created_keys: Vec<uuid::Uuid> = Vec::new();

    // 生成后：两行都是 status=1（未兑换），redeemed_* 均空。
    for code in &codes {
        let (st, by, at) = db_redemption_by_hash(&pool, code).await;
        assert_eq!(st, 1, "生成后 status 必须是 1（未兑换，列 DEFAULT）");
        assert!(
            by.is_none() && at.is_none(),
            "未兑换码不得有 redeemed_*: {by:?} {at:?}"
        );
    }
    // wire 列表同帧可见 status=1（管理页统计卡「未使用」的数据源）。
    let item = find_redemption_by_preview(&app, &admin_token, &code_preview(&codes[0])).await;
    assert_eq!(item["status"], 1, "列表投影 status=1");
    assert_eq!(item["quota"], quota);
    created_keys.push(item["key"].as_str().unwrap().parse().unwrap());

    // 核销第一张：200 + 回显码额。
    let resp = call(
        &app,
        "POST",
        "/api/user/topup",
        &user_token,
        Some(json!({"key": codes[0]})),
    )
    .await;
    let status = resp.status();
    let body = response_to_json(resp).await;
    assert_eq!(status, StatusCode::OK, "首次核销: {body}");
    assert_eq!(body["quota"], quota, "核销响应回显码额");
    assert_eq!(body["success"], true);

    // DB：status 1→2（已核销），redeemed_by=用户、redeemed_at 落值；
    // 钱包 FREE 恰好 +码额（核销与入账同事务）。
    let (st, by, at) = db_redemption_by_hash(&pool, &codes[0]).await;
    assert_eq!(
        st, 2,
        "核销后 status 必须是 2（后端实测；前端错画成「已停用」）"
    );
    assert_eq!(by, Some(user_key), "redeemed_by 必须落核销用户 key");
    assert!(at.is_some(), "redeemed_at 必须落值");
    assert_eq!(
        db_user_balance(&pool, user_key, "FREE").await,
        quota,
        "核销后 user_balances(FREE) 应=码额"
    );
    // wire 列表：管理页此刻应看到「已核销」语义的 status=2 行。
    let item = find_redemption_by_preview(&app, &admin_token, &code_preview(&codes[0])).await;
    assert_eq!(item["status"], 2, "列表投影 status=2");
    assert_eq!(item["redeemedBy"], user_key.to_string(), "列表投影带核销人");

    // 停用第二张：管理页路径 = 列表拿 key → DELETE /api/redemption/{key}。
    let item = find_redemption_by_preview(&app, &admin_token, &code_preview(&codes[1])).await;
    created_keys.push(item["key"].as_str().unwrap().parse().unwrap());
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/redemption/{}", item["key"].as_str().unwrap()),
        &admin_token,
        None,
    )
    .await;
    let status = resp.status();
    let body = response_to_json(resp).await;
    assert_eq!(status, StatusCode::OK, "停用未兑换码: {body}");
    assert_eq!(body["success"], true);
    let (st, _, _) = db_redemption_by_hash(&pool, &codes[1]).await;
    assert_eq!(
        st, 3,
        "停用后 status 必须是 3（后端实测；前端错画成「已核销」）"
    );

    // 停用后的码再核销 → 404（CAS WHERE status=1 不命中），余额不变。
    let resp = call(
        &app,
        "POST",
        "/api/user/topup",
        &user_token,
        Some(json!({"key": codes[1]})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "已停用码核销必须 404");
    assert_eq!(
        db_user_balance(&pool, user_key, "FREE").await,
        quota,
        "被拒核销不得入账"
    );

    cleanup_redemptions(&pool, &created_keys).await;
}

// ============================================================================
// 用例 2：兑换码拒绝路径（未知码 / 复用已核销码 / 停用已核销码）
// ============================================================================

/// 单次性与守卫边界（管理页「停用」按钮与用户核销入口的失败分支）：
/// - 核销不存在的明文码 → 404（按 code_hash 查无行）；
/// - 已核销（status=2）的码再核销 → 404 且余额不再增加（CAS 单赢家，
///   billing_lifecycle 已钉 wire 半边，这里补 DB status/余额不变半边）;
/// - 已核销的码 admin 停用 → 404（disable 守卫只认 status=1），
///   DB status 保持 2——停用不得覆盖/重写已核销终态。
#[tokio::test]
async fn redemption_rejects_unknown_replay_and_disable_after_redeem() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin, admin_name) = insert_admin_user(&pool).await;
    let admin_token = login(&app, &admin_name).await;
    let (user_key, user_name) = insert_user(&pool, 1).await;
    let user_token = login(&app, &user_name).await;

    // 未知明文码（形状合法、库里无哈希行）→ 404。
    let ghost = format!("fx-{}", hex::encode([0u8; 16]));
    let resp = call(
        &app,
        "POST",
        "/api/user/topup",
        &user_token,
        Some(json!({"key": ghost})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "未知码核销必须 404");

    // 生成一张并核销到 status=2。
    let quota = unique_quota();
    let codes = generate_codes_via_wire(&app, &admin_token, quota, 1).await;
    let (key,): (uuid::Uuid,) =
        sqlx::query_as("SELECT key FROM billing_redemptions WHERE code_hash = $1")
            .bind(sha256_hex(&codes[0]))
            .fetch_one(&pool)
            .await
            .unwrap();
    let resp = call(
        &app,
        "POST",
        "/api/user/topup",
        &user_token,
        Some(json!({"key": codes[0]})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "首次核销应成功");
    assert_eq!(db_user_balance(&pool, user_key, "FREE").await, quota);

    // 复用（二次核销）→ 404；DB status 保持 2、余额不再增加。
    let resp = call(
        &app,
        "POST",
        "/api/user/topup",
        &user_token,
        Some(json!({"key": codes[0]})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "复用已核销码必须 404");
    let (st, _, _) = db_redemption_by_hash(&pool, &codes[0]).await;
    assert_eq!(st, 2, "被拒的二次核销不得改写 status");
    assert_eq!(
        db_user_balance(&pool, user_key, "FREE").await,
        quota,
        "被拒的二次核销不得二次入账"
    );

    // 已核销码 admin 停用 → 404（disable 只认 status=1），终态不被覆盖。
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/redemption/{key}"),
        &admin_token,
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "已核销码停用必须 404（守卫只认 status=1）"
    );
    let (st, _, _) = db_redemption_by_hash(&pool, &codes[0]).await;
    assert_eq!(st, 2, "停用失败不得覆盖已核销终态");

    cleanup_redemptions(&pool, &[key]).await;
}

// ============================================================================
// 用例 3：系统选项写路径（PUT /api/option）
// ============================================================================

/// `PUT /api/option`（admin-ops/options.rs 注册表 + 类型化 validator）：
/// - 合法写入（数值/布尔各一）→ 200 回显 `{key, value}`，且
///   `GET /api/option`（items 信封）与 `GET /api/option/{key}` 两种读形状
///   都能读回同值——管理页 SystemOptionsPanel 与 update_option_api 依赖；
/// - 未知 key → 400（注册表白名单，拒绝任意 KV 注入）；
/// - 类型不符（布尔位发字符串、数值位发字符串）→ 400；
/// - 越界（retry 0/99 出 1..=10、retention 3 出 7..=365、负数配额）→ 400；
/// - 所有被拒写入不落库：读回仍是最后一次合法值。
/// 测试结束把触碰过的 key 恢复原值（e2e 库跨测试/跨运行共享）。
#[tokio::test]
async fn option_put_roundtrip_unknown_key_and_range_rejected() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin, admin_name) = insert_admin_user(&pool).await;
    let token = login(&app, &admin_name).await;

    // 触碰前先读原值（库值或注册表默认），结束时恢复。
    async fn read_option(app: &axum::Router, token: &str, key: &str) -> Value {
        let resp = call(app, "GET", &format!("/api/option/{key}"), token, None).await;
        assert_eq!(resp.status(), StatusCode::OK, "get option {key}");
        response_to_json(resp).await["value"].clone()
    }
    let retry_orig = read_option(&app, &token, "gateway.retry.max_attempts").await;
    let retention_orig = read_option(&app, &token, "observe.retention.usage_days").await;
    let reg_orig = read_option(&app, &token, "site.registration_enabled").await;

    // -- 合法写入 + 双读形状 roundtrip --
    let resp = call(
        &app,
        "PUT",
        "/api/option",
        &token,
        Some(json!({"key": "gateway.retry.max_attempts", "value": 7})),
    )
    .await;
    let status = resp.status();
    let body = response_to_json(resp).await;
    assert_eq!(status, StatusCode::OK, "合法数值写入: {body}");
    assert_eq!(body["key"], "gateway.retry.max_attempts");
    assert_eq!(body["value"], 7, "写入响应回显新值");
    assert_eq!(
        read_option(&app, &token, "gateway.retry.max_attempts").await,
        json!(7),
        "GET /api/option/{{key}} 必须读回新值"
    );
    // items 信封读形状（管理页列表的数据源）。
    let resp = call(&app, "GET", "/api/option", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list = response_to_json(resp).await;
    let row = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == "gateway.retry.max_attempts")
        .unwrap_or_else(|| panic!("updated option must be listed: {list}"));
    assert_eq!(row["value"], 7, "items 信封必须读回新值");

    let resp = call(
        &app,
        "PUT",
        "/api/option",
        &token,
        Some(json!({"key": "site.registration_enabled", "value": true})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "合法布尔写入");
    assert_eq!(
        read_option(&app, &token, "site.registration_enabled").await,
        json!(true),
        "布尔 roundtrip"
    );

    // -- 拒绝：未知 key --
    let resp = call(
        &app,
        "PUT",
        "/api/option",
        &token,
        Some(json!({"key": "site.unknown_key_for_e2e", "value": 1})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "未知 key 必须 400");

    // -- 拒绝：类型不符 --
    let resp = call(
        &app,
        "PUT",
        "/api/option",
        &token,
        Some(json!({"key": "site.registration_enabled", "value": "yes"})),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "布尔位发字符串必须 400"
    );
    let resp = call(
        &app,
        "PUT",
        "/api/option",
        &token,
        Some(json!({"key": "gateway.retry.max_attempts", "value": "3"})),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "数值位发字符串必须 400"
    );

    // -- 拒绝：越界 --
    for bad in [0, 99] {
        let resp = call(
            &app,
            "PUT",
            "/api/option",
            &token,
            Some(json!({"key": "gateway.retry.max_attempts", "value": bad})),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "retry 值域 1..=10，{bad} 必须 400"
        );
    }
    let resp = call(
        &app,
        "PUT",
        "/api/option",
        &token,
        Some(json!({"key": "observe.retention.usage_days", "value": 3})),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "retention 值域 7..=365"
    );
    let resp = call(
        &app,
        "PUT",
        "/api/option",
        &token,
        Some(json!({"key": "site.quota_new_user", "value": -1})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "配额必须非负");

    // -- 被拒写入一律不落库：读回仍是最后一次合法值 --
    assert_eq!(
        read_option(&app, &token, "gateway.retry.max_attempts").await,
        json!(7),
        "非法写入不得改值"
    );
    assert_eq!(
        read_option(&app, &token, "observe.retention.usage_days").await,
        retention_orig,
        "非法写入不得改值"
    );

    // -- 恢复原值（写回触碰过的 key）--
    for (key, value) in [
        ("gateway.retry.max_attempts", retry_orig),
        ("observe.retention.usage_days", retention_orig),
        ("site.registration_enabled", reg_orig),
    ] {
        let resp = call(
            &app,
            "PUT",
            "/api/option",
            &token,
            Some(json!({"key": key, "value": value})),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK, "restore {key}");
    }
}

// ============================================================================
// 用例 4：渠道 update 的 groups 语义（omitted 保持 / 显式空数组清空 = known-gap）
// ============================================================================

/// 渠道 update 对 groups 的**当前真实行为**（对照 create 守卫）：
/// - create 显式 `"groups": []` → 400（写侧守卫：空 groups = snapshot 展开
///   零路由单元，admin-catalog/channels.rs create 分支）；
/// - update **缺席** groups → COALESCE 保持落库值（最小 diff 语义）；
/// - update **显式** `"groups": []` → 200 且落库清成空数组。
///   **known-gap（todo/page-audit-manage.md §3-5）**：update 无 groups 非空
///   守卫，前端渠道弹窗 chips 全取消即触发——渠道从所有分组消失、网关
///   快照不再装配、路由静默失效且无任何提示。本用例按现状钉死，后端补
///   守卫后，请把「显式空数组」两处断言翻转为 400/落库不变。
#[tokio::test]
async fn channel_update_groups_omitted_preserved_empty_clears_known_gap() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_admin, admin_name) = insert_admin_user(&pool).await;
    let token = login(&app, &admin_name).await;

    let name = format!("mgr_ch_{}", &uuid::Uuid::new_v4().to_string()[..8]);

    // 对照组：create 空数组被守卫拒绝（update 缺的就是这条）。
    let resp = call(
        &app,
        "POST",
        "/api/channel",
        &token,
        Some(json!({
            "name": format!("{name}_empty"),
            "channelType": "openai",
            "baseUrl": "http://mock-manage-wire",
            "keys": ["sk-manage-1"],
            "groups": []
        })),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "create 空 groups 必须被守卫拒绝"
    );

    // 正常创建：groups=["default"] 落库。
    let ch_key = create_channel_via_wire(&app, &token, &name, &["default"]).await;
    assert_eq!(
        db_channel_groups(&pool, &ch_key).await,
        vec!["default".to_string()],
        "创建后 groups 落库"
    );

    // update 缺席 groups → COALESCE 保持（前端最小 diff 体不重发分组时的语义）。
    let resp = call(
        &app,
        "PUT",
        &format!("/api/channel/{ch_key}"),
        &token,
        Some(json!({"remark": "no-groups-field"})),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "缺席 groups 的 put: {resp:?}"
    );
    assert_eq!(
        db_channel_groups(&pool, &ch_key).await,
        vec!["default".to_string()],
        "update 缺席 groups 必须保持落库值（COALESCE）"
    );

    // known-gap：update 显式空数组 → 200 + 落库清空（后端补守卫后翻转为 400）。
    let resp = call(
        &app,
        "PUT",
        &format!("/api/channel/{ch_key}"),
        &token,
        Some(json!({"groups": []})),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "known-gap：update 空 groups 当前被接受（create 有守卫、update 没有）"
    );
    let body = response_to_json(resp).await;
    assert_eq!(body["groups"], json!([]), "known-gap：响应回显空 groups");
    assert_eq!(
        db_channel_groups(&pool, &ch_key).await,
        Vec::<String>::new(),
        "known-gap：落库 groups 被清空 → 渠道静默永不路由"
    );
}
