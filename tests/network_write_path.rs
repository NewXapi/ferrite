//! E2E: 拓扑页 drawer 写路径（#196 移植 #156 的写路径测试到现行 wire）。
//!
//! B1（93e42a4）把 drawer 写操作接到真实 API 后，本文件从 HTTP 层钉死
//! drawer 实际发出的每个写请求的后端半边，与 web_wire_contract.rs 互补：
//! 那边用 SQL 直插种子行，这里全部实体经 wire（POST）创建——即 drawer
//! 导入面板的真实路径——再驱动编辑/删除/启停，验证整条写生命周期。
//!
//! 钉死的契约（后端 admin-catalog channels.rs / groups.rs）：
//! - 渠道编辑 PUT 是最小 diff 语义：`keys` 缺席 = COALESCE 保持现有密钥
//!   （drawer 不重输密钥时的形状；误发 `[]` 会被 validate 400 拒绝）；
//!   `testModel` 直绑无 COALESCE，缺席即清 NULL——前端必须恒带现值。
//! - 分组展示名（remark 列）可更新；分组名无更新路径（UpdateGroupRequest
//!   无 name 列，body 里发 name 被忽略）——UI 侧诚实锁读正是对准它，
//!   防止"改名 200 但什么都没变"的假成功回归。
//! - 删除：渠道 DELETE 后单查 404；分组有引用守卫（被渠道绑定时 409），
//!   先删渠道再删分组才成功——drawer 删除确认弹窗的失败分支依赖该语义。
//! - 启停 `POST /api/channel/{key}/status` 值域 1|2，其余 400。
//! - 掩码不回传：单查 keys 是掩码回显（明文永不出现），drawer 能看到的
//!   只有掩码；配合 keys 缺席语义，掩码值没有任何机会覆盖真实密钥。
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
        unreachable!("write-path tests never hit the forward plane; MockEgress is a placeholder")
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
                std::env::set_var("FERRITE_JWT_SECRET", "ferrite-e2e-write-path-secret-2026");
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

// ---------------------------------------------------------------------------
// wire 建实体 helpers：本文件所有实体都走 POST 创建（drawer 导入面板路径），
// 不用 SQL 种子，保证测的是完整写链路。
// ---------------------------------------------------------------------------

/// 经 `POST /api/channel` 创建渠道（drawer 导入面板的全量
/// `ChannelUpsertRequest` 形状），返回 (key, name)。
async fn create_channel_via_wire(
    app: &axum::Router,
    token: &str,
    name: &str,
    groups: &[&str],
    keys: &[&str],
    test_model: Option<&str>,
) -> (String, String) {
    let body = json!({
        "name": name,
        "channelType": "openai",
        "baseUrl": "http://mock-write-path",
        "keys": keys,
        "models": [],
        "groups": groups,
        "priority": 0,
        "weight": 0,
        "testModel": test_model,
        "remark": ""
    });
    let resp = call(app, "POST", "/api/channel", token, Some(body)).await;
    let status = resp.status();
    let created = response_to_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "channel create must succeed: {created}"
    );
    let key = created["key"]
        .as_str()
        .unwrap_or_else(|| panic!("created channel must carry key: {created}"))
        .to_string();
    (key, name.to_string())
}

/// 经 `POST /api/group` 创建分组（drawer 新建分组路径），返回 key。
async fn create_group_via_wire(
    app: &axum::Router,
    token: &str,
    name: &str,
    remark: &str,
) -> String {
    let body = json!({
        "name": name,
        "ratio": 0.9,
        "modelWhitelist": ["gpt-4o"],
        "remark": remark
    });
    let resp = call(app, "POST", "/api/group", token, Some(body)).await;
    let status = resp.status();
    let created = response_to_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "group create must succeed: {created}"
    );
    created["key"]
        .as_str()
        .unwrap_or_else(|| panic!("created group must carry key: {created}"))
        .to_string()
}

/// 从 DB 读回渠道行的写路径关注列（keys 明文数组 + 各 COALESCE/直绑列），
/// 供断言"落库值"而非只看响应回显。
struct ChannelDbRow {
    keys: Vec<String>,
    models: Value,
    priority: i32,
    weight: i32,
    test_model: Option<String>,
    status: i16,
    name: String,
    base_url: String,
}

async fn db_channel_row(pool: &sqlx::PgPool, key: &str) -> ChannelDbRow {
    let uuid = uuid::Uuid::parse_str(key).unwrap();
    let (keys_raw, models, priority, weight, test_model, status, name, base_url): (
        Value,
        Value,
        i32,
        i32,
        Option<String>,
        i16,
        String,
        String,
    ) = sqlx::query_as(
        "SELECT keys, models, priority, weight, test_model, status, name, base_url \
         FROM api_channels WHERE key = $1",
    )
    .bind(uuid)
    .fetch_one(pool)
    .await
    .unwrap();
    ChannelDbRow {
        keys: serde_json::from_value(keys_raw).unwrap(),
        models,
        priority,
        weight,
        test_model,
        status,
        name,
        base_url,
    }
}

// ============================================================================
// 用例
// ============================================================================

/// drawer「导入面板」创建 + 「单查回显」契约：
/// - POST 全量体落库，响应 ChannelView 携带 key（drawer 后续编辑/删除的
///   定位符）与 keyCount；
/// - `GET /api/channel/{key}` 的 keys 是**掩码**回显（head****tail），明文
///   永不出现——drawer 能看到的 keys 只有掩码，这是"掩码不回传"硬约定的
///   服务端半边（前端半边由 crate 内 channel_update_body.rs 钉死）。
#[tokio::test]
async fn channel_import_creates_and_single_get_masks_keys() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let name = format!("wr_ch_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let (ch_key, _) = create_channel_via_wire(
        &app,
        &token,
        &name,
        &["default"],
        &["sk-plain-alpha", "sk-plain-beta"],
        None,
    )
    .await;

    // 创建响应：key 可定位、keyCount=2、keys 掩码、明文不出现。
    let resp = call(&app, "GET", &format!("/api/channel/{ch_key}"), &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "single get must be 200");
    let body = response_to_json(resp).await;
    assert_eq!(body["key"], ch_key, "单查 key 即创建时返回的定位符");
    assert_eq!(body["name"], name);
    assert_eq!(body["channelType"], "openai");
    assert_eq!(body["baseUrl"], "http://mock-write-path");
    assert_eq!(body["keyCount"], 2, "keyCount 是密钥数展示维度");
    let keys = body["keys"]
        .as_array()
        .unwrap_or_else(|| panic!("single get must carry masked keys: {body}"));
    assert_eq!(keys.len(), 2);
    assert!(
        keys.iter().all(|k| k.as_str().unwrap().contains("****")),
        "单查 keys 必须逐条掩码（head****tail）: {keys:?}"
    );
    assert!(
        !body.to_string().contains("sk-plain-alpha"),
        "明文密钥绝不能出现在任何单查响应中"
    );

    // 落库侧仍是明文（掩码只发生在响应投影，存储永远明文供网关转发用）。
    let row = db_channel_row(&pool, &ch_key).await;
    assert_eq!(
        row.keys,
        vec!["sk-plain-alpha".to_string(), "sk-plain-beta".to_string()],
        "DB 必须保存明文密钥（网关转发依赖），掩码只是响应侧投影"
    );
}

/// drawer「渠道编辑弹窗」最小 diff 更新的整条生命周期（#160 语义在
/// wire 创建实体上的回归闸）：
/// - 列表按 name 定位（drawer find_channel_by_name 依赖）→ 列表 keys 为
///   null（密钥只在单查回显）；
/// - PUT 不带 `keys`（用户未重输密钥的形状）→ DB 明文密钥原样保留
///   （COALESCE）；name/baseUrl 更新生效；恒带的 testModel 保持；
///   弹窗不管理的 models/priority/weight 不被静默清零；
/// - 更新后列表能用新名定位到同一 key（改名后 drawer 刷新链路可用）。
#[tokio::test]
async fn channel_minimal_diff_update_renames_url_and_preserves_keys() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let name = format!("wr_ch_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let (ch_key, _) = create_channel_via_wire(
        &app,
        &token,
        &name,
        &["default"],
        &["sk-keep-1", "sk-keep-2"],
        Some("gpt-4o-mini"),
    )
    .await;

    // 前置：列表按 name 定位（drawer 编辑入口），且列表不携带 keys。
    let resp = call(&app, "GET", "/api/channel", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list = response_to_json(resp).await;
    let row = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("wire-created channel must be listed: {list}"));
    assert_eq!(row["key"], ch_key);
    assert_eq!(row["keyCount"], 2);
    assert!(
        row["keys"].is_null(),
        "列表端点不携带 keys（掩码只在单查回显）: {}",
        row["keys"]
    );

    // PUT 最小 diff 体（UpdateChannelBody 恒发形状，keys 缺席）。
    let new_name = format!("{name}_renamed");
    let put = call(
        &app,
        "PUT",
        &format!("/api/channel/{ch_key}"),
        &token,
        Some(json!({
            "name": new_name,
            "channelType": "openai",
            "baseUrl": "http://renamed-write-path",
            "groups": ["default"],
            "remark": "edited",
            "testModel": "gpt-4o-mini"
        })),
    )
    .await;
    assert_eq!(put.status(), StatusCode::OK, "minimal-diff put: {put:?}");

    let row = db_channel_row(&pool, &ch_key).await;
    assert_eq!(row.name, new_name, "name 更新必须落库");
    assert_eq!(
        row.base_url, "http://renamed-write-path",
        "baseUrl 更新必须落库"
    );
    assert_eq!(
        row.keys,
        vec!["sk-keep-1".to_string(), "sk-keep-2".to_string()],
        "keys 缺席必须保持现有密钥（COALESCE），否则编辑保存一次就清空渠道密钥"
    );
    assert_eq!(
        row.test_model.as_deref(),
        Some("gpt-4o-mini"),
        "恒带的 testModel 现值保持不变（缺席才会清列）"
    );
    assert_eq!(row.models, json!([]), "弹窗不管理的 models 不得被静默清零");
    assert_eq!(row.priority, 0, "弹窗不管理的 priority 不得被静默清零");
    assert_eq!(row.weight, 0, "弹窗不管理的 weight 不得被静默清零");

    // 更新后列表能用新名定位同一渠道（drawer find-by-name + 刷新链路）。
    let resp = call(&app, "GET", "/api/channel", &token, None).await;
    let list = response_to_json(resp).await;
    let row = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == new_name)
        .unwrap_or_else(|| panic!("renamed channel must be listed by new name: {list}"));
    assert_eq!(row["key"], ch_key, "改名后 key 不变（编辑定位符稳定）");
}

/// `testModel` 直绑列语义（无 COALESCE）：
/// - PUT 缺席 `testModel` → 列清 NULL。这是前端必须**恒带现值**的原因
///   （UpdateChannelBody.test_model 恒发的对准点）；钉死后端行为，防止
///   svc 把直绑改成 COALESCE 后前端锚点失效而不自知。
/// - PUT 显式 `testModel` → 列更新。
#[tokio::test]
async fn channel_update_omitting_test_model_clears_column() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let name = format!("wr_ch_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let (ch_key, _) = create_channel_via_wire(
        &app,
        &token,
        &name,
        &["default"],
        &["sk-tm-1"],
        Some("gpt-4o-mini"),
    )
    .await;

    // PUT 缺席 testModel（其余列同样缺席）：该列被清 NULL，keys 不受影响。
    let put = call(
        &app,
        "PUT",
        &format!("/api/channel/{ch_key}"),
        &token,
        Some(json!({"name": name, "channelType": "openai", "baseUrl": "http://mock-write-path"})),
    )
    .await;
    assert_eq!(put.status(), StatusCode::OK, "testModel-omit put: {put:?}");
    let row = db_channel_row(&pool, &ch_key).await;
    assert_eq!(
        row.test_model, None,
        "testModel 缺席 → 列清 NULL（直绑无 COALESCE 的后端语义）"
    );
    assert_eq!(
        row.keys,
        vec!["sk-tm-1".to_string()],
        "testModel 清列不得波及 keys（COALESCE 列各自独立）"
    );

    // PUT 显式 testModel → 落库。
    let put = call(
        &app,
        "PUT",
        &format!("/api/channel/{ch_key}"),
        &token,
        Some(json!({"testModel": "gpt-5-mini"})),
    )
    .await;
    assert_eq!(put.status(), StatusCode::OK);
    let row = db_channel_row(&pool, &ch_key).await;
    assert_eq!(
        row.test_model.as_deref(),
        Some("gpt-5-mini"),
        "显式 testModel 落库"
    );
}

/// drawer「分组展示名编辑」契约（分组名锁读的后端半边）：
/// - PUT remark → 落库生效（display 列即 remark）；
/// - 前端透传的 ratio / modelWhitelist 现值原样保留（GroupUpsertRequest
///   恒发形状，后端 COALESCE 各列独立）；
/// - body 里的 `name` 被服务端忽略（UpdateGroupRequest 无 name 列，分组
///   无更新路径）——发不同名 → 200 但名字不变。这正是 UI 侧把分组名
///   锁为只读的原因：否则就是"改名 200 但什么都没变"的假成功（#156 前
///   Playwright 实测抓到过）。
#[tokio::test]
async fn group_display_update_lands_and_name_is_locked() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let group_name = format!("wr_grp_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let grp_key = create_group_via_wire(&app, &token, &group_name, "旧展示名").await;

    // 前端 update_group_display 的恒发形状：name/ratio/modelWhitelist
    // 透传服务端现值，只有 remark 是新值。
    let put = call(
        &app,
        "PUT",
        &format!("/api/group/{grp_key}"),
        &token,
        Some(json!({
            "name": group_name,
            "ratio": 0.9,
            "modelWhitelist": ["gpt-4o"],
            "remark": "新展示名"
        })),
    )
    .await;
    assert_eq!(put.status(), StatusCode::OK, "group display put: {put:?}");
    let updated = response_to_json(put).await;
    assert_eq!(updated["remark"], "新展示名", "展示名（remark）必须更新");
    assert_eq!(updated["name"], group_name, "分组名在更新响应中保持现值");
    assert_eq!(updated["ratio"], 0.9, "透传的 ratio 现值保留");
    assert_eq!(
        updated["modelWhitelist"],
        json!(["gpt-4o"]),
        "透传的白名单现值保留"
    );

    // 列表读回：remark 已落库、其余列不变。
    let resp = call(&app, "GET", "/api/group", &token, None).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list = response_to_json(resp).await;
    let row = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == grp_key)
        .unwrap_or_else(|| panic!("updated group must be listed: {list}"));
    assert_eq!(row["remark"], "新展示名");
    assert_eq!(row["name"], group_name);

    // 分组名锁读的服务端半边：body 发不同名 → 200 但名字不变（无改名路径）。
    let put = call(
        &app,
        "PUT",
        &format!("/api/group/{grp_key}"),
        &token,
        Some(json!({
            "name": "attempted-rename",
            "ratio": 0.9,
            "modelWhitelist": ["gpt-4o"],
            "remark": "新展示名"
        })),
    )
    .await;
    assert_eq!(put.status(), StatusCode::OK);
    let row = db_group_name(&pool, &grp_key).await;
    assert_eq!(
        row, group_name,
        "body 里的 name 必须被忽略（UpdateGroupRequest 无 name 列）：UI 锁读分组名正是对准该语义"
    );
}

/// 读回分组名（锁读断言用）。
async fn db_group_name(pool: &sqlx::PgPool, key: &str) -> String {
    let uuid = uuid::Uuid::parse_str(key).unwrap();
    sqlx::query_scalar("SELECT name FROM api_groups WHERE key = $1")
        .bind(uuid)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// drawer「删除」契约（确认弹窗后的两条 DELETE 路径 + 引用守卫）：
/// - 分组删除：无引用 → 200 `{"success": true}`，列表不再出现；
/// - 分组引用守卫：仍被渠道绑定的分组 DELETE → 409（防止静默孤儿）；
///   解绑（删渠道）后才能删除成功；
/// - 渠道删除：DELETE → 200，单查 404；不存在的 key → 404。
#[tokio::test]
async fn group_and_channel_delete_paths() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    // 分组无引用删除：200 success，列表不再出现。
    let free_name = format!("wr_grp_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let free_key = create_group_via_wire(&app, &token, &free_name, "").await;
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/group/{free_key}"),
        &token,
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "unreferenced group delete: {resp:?}"
    );
    let body = response_to_json(resp).await;
    assert_eq!(body["success"], true, "分组删除响应为 success 信封: {body}");
    let resp = call(&app, "GET", "/api/group", &token, None).await;
    let list = response_to_json(resp).await;
    assert!(
        !list["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["key"] == free_key),
        "删除后的分组不得再出现在列表中"
    );

    // 分组引用守卫：被渠道绑定的分组删除 → 409，渠道与分组都还在。
    let bound_name = format!("wr_grp_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let bound_key = create_group_via_wire(&app, &token, &bound_name, "").await;
    let (ch_key, _) = create_channel_via_wire(
        &app,
        &token,
        &format!("wr_ch_{}", &uuid::Uuid::new_v4().to_string()[..8]),
        &[&bound_name],
        &["sk-del-1"],
        None,
    )
    .await;
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/group/{bound_key}"),
        &token,
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "仍被渠道引用的分组删除必须 409（引用守卫防孤儿），否则删除成功但路由静默失效"
    );
    let resp = call(&app, "GET", "/api/group", &token, None).await;
    let list = response_to_json(resp).await;
    assert!(
        list["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["key"] == bound_key),
        "409 后分组必须还在（删除未发生）"
    );

    // 解绑（删渠道）后，分组可删除——drawer 的引导顺序即先删渠道再删分组。
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/channel/{ch_key}"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "channel delete: {resp:?}");
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/group/{bound_key}"),
        &token,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "解绑后的分组删除应成功");

    // 渠道删除后单查 404；不存在的 key 也是 404（drawer 错误分支依赖）。
    let resp = call(&app, "GET", &format!("/api/channel/{ch_key}"), &token, None).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "删除后的渠道单查必须 404"
    );
    let ghost = uuid::Uuid::new_v4();
    let resp = call(
        &app,
        "DELETE",
        &format!("/api/channel/{ghost}"),
        &token,
        None,
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "删除不存在的渠道必须 404"
    );
}

/// drawer「渠道启停」快捷写契约：`POST /api/channel/{key}/status`
/// - `{"status": 2}` → DB 停用；`{"status": 1}` → DB 启用；
/// - 值域 1|2：越界值 400（drawer 只发 1/2，钉死服务端值域防误用漂移）。
#[tokio::test]
async fn channel_status_toggle_and_domain() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;
    let (_user, username) = insert_admin_user(&pool).await;
    let token = login(&app, &username).await;

    let name = format!("wr_ch_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let (ch_key, _) =
        create_channel_via_wire(&app, &token, &name, &["default"], &["sk-status-1"], None).await;
    assert_eq!(
        db_channel_row(&pool, &ch_key).await.status,
        1,
        "wire 创建的渠道默认启用（status=1）"
    );

    // 停用。
    let resp = call(
        &app,
        "POST",
        &format!("/api/channel/{ch_key}/status"),
        &token,
        Some(json!({"status": 2})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "disable put: {resp:?}");
    let body = response_to_json(resp).await;
    assert_eq!(body["status"], 2, "停用响应回显新状态");
    assert_eq!(
        db_channel_row(&pool, &ch_key).await.status,
        2,
        "停用必须落库"
    );

    // 重新启用。
    let resp = call(
        &app,
        "POST",
        &format!("/api/channel/{ch_key}/status"),
        &token,
        Some(json!({"status": 1})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        db_channel_row(&pool, &ch_key).await.status,
        1,
        "启用必须落库"
    );

    // 值域 1|2：9 → 400，状态不变。
    let resp = call(
        &app,
        "POST",
        &format!("/api/channel/{ch_key}/status"),
        &token,
        Some(json!({"status": 9})),
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "status 值域必须 1|2"
    );
    assert_eq!(
        db_channel_row(&pool, &ch_key).await.status,
        1,
        "非法值不得改状态"
    );
}
