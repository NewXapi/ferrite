//! billing::topup provider 抽象 + webhook 幂等结算测试。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p billing --test topup_provider`（PG 不可达时真跑断言的用例自动跳过）
//!
//! 覆盖场景（每个测试单独注释测什么、为什么这个预期）：
//! - ManualProvider 纯逻辑（无 PG）：开单回执订单号、验签恒拒
//! - webhook 全链路幂等（PG）：假渠道验签通过 → settle 入金；重复回调 →
//!   200 + 已入账额、余额只加一次
//! - 未知 provider → 404（handler 直调，DB 访问前返回，离线全跑）
//! - 验签失败 → 401（同上；manual 收"合法样子"的回调也 401）
//!
//! HTTP 层选 handler 直调而非 tower oneshot（参考 marker.rs 模式）：本 crate
//! dev-deps 没有 tower，不为此加依赖；路由字符串由 axum 编译期保证，需要行为
//! 证明的是 404/401/200-幂等三分支语义。与 topup_affiliate.rs 同款 skip 模式
//! （CI `cargo test -p billing` 不带 --ignored，#[ignore] = 永远不跑）。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use billing::topup::{ProviderFuture, TopupAppState, TopupProvider, TopupSession, topup_webhook};
use billing::{ManualProvider, ProviderError, TopupService};
use contract::api::billing::TopUpRequest;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

/// 测试假渠道：验签规则 = payload.order 存在即通过（模拟真实 webhook 里
/// 渠道回执商户订单号；真验签是 HMAC 比对，协议细节归实现方，本域只定接口）。
/// 注入方式与生产一致：TopupService::with_provider。
struct FakeProvider;

impl TopupProvider for FakeProvider {
    fn id(&self) -> &'static str {
        "fake"
    }

    fn create<'a>(
        &'a self,
        order_key: &'a str,
        _currency: &'a str,
        _amount: i64,
    ) -> ProviderFuture<'a, Result<TopupSession, ProviderError>> {
        Box::pin(async move {
            Ok(TopupSession {
                reference: format!("fake-ref-{order_key}"),
            })
        })
    }

    fn verify_callback<'a>(
        &'a self,
        payload: &'a serde_json::Value,
    ) -> ProviderFuture<'a, Option<String>> {
        let order = payload
            .get("order")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        Box::pin(async move { order })
    }
}

/// PG 连接 + app state（注入 fake 渠道）；**PG 不可达返回 `None` 让调用方 skip**。
/// skip 而非 #[ignore] 的理由见文件头（与 topup_affiliate.rs 同款约定）。
async fn make_app() -> Option<(TopupAppState, PgPool)> {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .map_err(|e| eprintln!("skipping: postgres unreachable at {}: {e}", db_url()))
        .ok()?;
    db_bootstrap::run_migrations(&pool)
        .await
        .expect("migrations must apply once PG is reachable");
    Some((build_state(pool.clone()), pool))
}

fn build_state(pool: PgPool) -> TopupAppState {
    let auth = Arc::new(
        auth::AuthService::new(pool.clone(), vec![0x42; 32]).expect("32-byte secret is valid"),
    );
    let svc = Arc::new(
        TopupService::new(pool) // 默认表里已有 manual
            .with_provider(Arc::new(FakeProvider)),
    );
    TopupAppState { svc, auth }
}

/// 懒连接 pool：404/401 路径在触碰 DB 之前就返回，connect_lazy 永不建连，
/// 故这两个 handler 测试离线也全跑，不走 skip 模式。
fn lazy_state() -> TopupAppState {
    build_state(
        PgPoolOptions::new()
            .connect_lazy(&db_url())
            .expect("connect_lazy 不建连，仅 URL 非法才 Err"),
    )
}

async fn make_user(pool: &PgPool) -> Uuid {
    let key = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, used_quota, group_id, auth_version)
           VALUES ($1, $2, $3, NULL, 'x', 1, 1, 0, 0, 'default', 1)"#,
    )
    .bind(key)
    .bind(format!("tp_user_{}", key.simple()))
    .bind("tp test")
    .execute(pool)
    .await
    .expect("insert user");
    key
}

/// 清理测试用户的 billing 痕迹（订单 + 余额行），auth_users 行保留（同
/// topup_affiliate.rs 约定，用户名带 uuid 不冲突）。
async fn cleanup(pool: &PgPool, user: Uuid) {
    sqlx::query("DELETE FROM billing_topups WHERE user_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM user_balances WHERE user_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
}

/// 用户 FREE 余额；无行按 0。
async fn free_balance(pool: &PgPool, user: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE((SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'), 0)",
    )
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("free balance")
}

/// ManualProvider 纯逻辑（无 PG）：
/// - id = "manual"（billing_topups.provider / webhook 路径都以此为准）；
/// - create 无外部系统 → reference 就是订单号本身（前端可直接拿它当 settle key）；
/// - verify_callback 恒 None → manual 单**不可能**被 webhook 入金，伪造回调
///   在验签边界就死，只能走 admin settle。这是 manual 作为默认实现的安全底线。
#[tokio::test]
async fn manual_provider_create_returns_reference() {
    assert_eq!(ManualProvider.id(), "manual");
    let session = ManualProvider
        .create("ord-42", "FREE", 1000)
        .await
        .expect("manual 开单无外部依赖，不应失败");
    assert_eq!(session.reference, "ord-42");
    assert_eq!(
        ManualProvider
            .verify_callback(&serde_json::json!({ "order": "ord-42" }))
            .await,
        None,
        "manual 无 webhook：任何回调（哪怕订单号真实）都验不过"
    );
}

/// webhook 全链路幂等（PG）——本批核心保证：**同一订单两次回调，余额只加一次**。
/// 幂等护栏不在 webhook 层，在 settle_topup 的 state CAS（pending→settling 只
/// 成一个）；webhook 层要证明的是重放时回执语义正确：第二次不是 400 报错
/// （渠道会把报错当投递失败无限重试），而是 200 + 首次已入账额。
/// 断言链：首调 200 credited=5000 且余额真 +5000 → 重放 200 credited=5000
/// 且余额纹丝不动 → 排除"回执对了但钱加两次"与"报错了但钱没到账"两类回归。
#[tokio::test]
async fn webhook_settles_order_once() {
    let Some((app, pool)) = make_app().await else {
        return;
    };
    let user = make_user(&pool).await;
    // 开单走服务层现有路径：provider 抽象化后 open_topup 行为不变
    // （pending 单 + currency 校验），这里同时充当该不变式的回归探针。
    let key = app
        .svc
        .open_topup(TopUpRequest {
            user_key: user.to_string(),
            currency: "FREE".into(),
            amount: 5000,
        })
        .await
        .expect("open_topup");
    assert_eq!(free_balance(&pool, user).await, 0);

    let payload = serde_json::json!({ "order": key });
    let Json(first) = topup_webhook(
        State(app.clone()),
        Path("fake".into()),
        Json(payload.clone()),
    )
    .await
    .expect("首次回调：验签通过 + settle 成功");
    assert_eq!(first["credited"].as_i64(), Some(5000));
    assert_eq!(free_balance(&pool, user).await, 5000);

    let Json(second) = topup_webhook(State(app.clone()), Path("fake".into()), Json(payload))
        .await
        .expect("重复回调：CAS 失败但已 paid → 幂等回执 200，不是报错");
    assert_eq!(
        second["credited"].as_i64(),
        Some(5000),
        "回执金额 = 已入账额"
    );
    assert_eq!(
        free_balance(&pool, user).await,
        5000,
        "余额只加一次（幂等证明）"
    );

    cleanup(&pool, user).await;
}

/// 未知 provider → 404：注入表里没有的 id（如 "stripe" 未注册）必须 404，
/// 而不是静默 401——渠道方拼错回调地址时 404 才可在其后台观测到。
/// 不触 DB（404 在查询前返回），用懒连接，离线全跑。
#[tokio::test]
async fn webhook_unknown_provider_404() {
    let app = lazy_state();
    let (status, _) = topup_webhook(
        State(app),
        Path("stripe".into()),
        Json(serde_json::json!({ "order": "whatever" })),
    )
    .await
    .expect_err("未注册 provider 必须拒");
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// 验签失败 → 401：verify_callback 返回 None 的两条真实路径——
/// a) fake 渠道收到缺 `order` 字段的 payload（坏签名/格式错误）；
/// b) manual 收到"样子合法"的回调（订单号字段齐全）——manual 恒 None，
///    HTTP 层同样 401，证明"验签就是鉴权"没有旁路。
/// 均不触 DB，懒连接离线全跑。
#[tokio::test]
async fn webhook_bad_signature_401() {
    let app = lazy_state();
    let (status, _) = topup_webhook(
        State(app.clone()),
        Path("fake".into()),
        Json(serde_json::json!({ "sig": "tampered" })),
    )
    .await
    .expect_err("验签失败必须拒");
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = topup_webhook(
        State(app),
        Path("manual".into()),
        Json(serde_json::json!({ "order": "ord-1" })),
    )
    .await
    .expect_err("manual 无 webhook，恒拒");
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
