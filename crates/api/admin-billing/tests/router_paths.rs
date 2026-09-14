//! 路由路径不冲突回归 —— 防 `apps/api` 启动即 panic。
//!
//! 背景（本文件存在的唯一理由）：`redeem::router` 与 `topup::router` 曾同时
//! 注册 `POST /api/user/topup`。axum 0.8 的 `Router::merge` 对**同 path 同
//! method** 的重叠直接 panic（"Overlapping method route"），而 admin-router
//! 把两者一起 merge——于是 `apps/api` 一启动就崩，且编译期完全看不出来。
//!
//! 断言策略：merge 后对每条路径打一次真实请求，用**状态码不是 404** 证明
//! 路由确实挂上了。只断言"merge 不 panic"是不够的——那样有人把冲突路由
//! 直接删掉一条也能过；这里要求三条路径同时存在。
//!
//! 命中证据取"非 404/405"：具体码取决于提取器顺序——带 `Json<T>` 提取器的
//! handler 会先因 body 缺字段返回 422（提取器在 handler 体之前跑，够不到
//! `bearer_user`），只有 `HeaderMap` 先于 body 的 handler 才返回 401。
//! 404 = 路由不存在（回归），405 = method 挂错（回归）。

use std::sync::Arc;

use axum::body::Body;
use billing::{RedeemAppState, RedeemService, TopupAppState, TopupService};
use http::{Request, StatusCode};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

/// 构造一个**不连接**的 lazy 池：本测试只关心路由表拼装，不碰 DB。
/// `connect_lazy` 直到首次查询才建连接，因此无 PG 环境也能跑。
fn lazy_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .expect("lazy pool never dials at construction")
}

/// JWT secret 只用于构造 AuthService；本测试不签发也不校验令牌。
fn auth_service(pool: sqlx::PgPool) -> Arc<auth::AuthService> {
    Arc::new(
        auth::AuthService::new(pool, b"router_paths_test_secret_32bytes_len!".to_vec())
            .expect("auth service constructs with a 32+ byte secret"),
    )
}

/// 合并全部 billing 子路由（与 admin-router 的聚合动作同形）。
///
/// merge 本身是历史 panic 点：任何同 path 同 method 重叠都会在这里炸。
fn merged_billing_router() -> axum::Router {
    let pool = lazy_pool();
    let auth = auth_service(pool.clone());

    let redeem = billing::router(RedeemAppState {
        svc: Arc::new(RedeemService::new(pool.clone())),
        auth: auth.clone(),
    });
    let wallet = billing::wallet_router(billing::WalletAppState {
        svc: Arc::new(billing::WalletService::new(pool.clone())),
        auth: auth.clone(),
    });
    let currency = billing::currency_router(billing::CurrencyAppState {
        svc: Arc::new(billing::CurrencyService::new(pool.clone())),
        auth: auth.clone(),
    });
    let affiliate = billing::affiliate_router(billing::AffiliateAppState {
        svc: Arc::new(billing::AffiliateService::new(
            pool.clone(),
            billing::WalletService::new(pool.clone()),
        )),
        auth: auth.clone(),
    });
    let topup = billing::topup_router(TopupAppState {
        svc: Arc::new(TopupService::new(pool)),
        auth,
    });

    redeem
        .merge(wallet)
        .merge(currency)
        .merge(affiliate)
        .merge(topup)
}

/// 对合并后的 router 打一次无鉴权 POST，返回状态码。
async fn post_status(router: axum::Router, path: &str) -> StatusCode {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("request builds");
    router
        .oneshot(request)
        .await
        .expect("router responds")
        .status()
}

/// 兑换码核销与充值开单必须**各自独立存在**于不同路径。
///
/// 期望值的由来：两条路径都命中路由 → 走到 `bearer_user` → 无 Authorization
/// 头返回 401。若某条返回 404，说明该路由被删或改名（回归）；若 merge 本身
/// panic，测试直接崩（这是修复前的行为）。
#[tokio::test]
async fn redeem_and_topup_order_paths_coexist() {
    // `/api/user/topup` = 兑换码核销（new-api 惯例，前端已按该形状接线）
    let redeem_status = post_status(merged_billing_router(), "/api/user/topup").await;
    assert_ne!(
        redeem_status,
        StatusCode::NOT_FOUND,
        "兑换码路径必须存在：/api/user/topup 是 redeem 的核销入口"
    );
    assert_ne!(
        redeem_status,
        StatusCode::METHOD_NOT_ALLOWED,
        "method 必须是 POST"
    );
    // 422：`Json<TopupRequest>` 提取器先于 handler 体运行，`{}` 缺 `key`
    // 字段即被拒——这同样证明请求命中了这条路由。
    assert_eq!(
        redeem_status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "命中 redeem handler：Json 提取器因缺 key 字段返回 422"
    );

    // `/api/user/topup/order` = 充值开单（从 /api/user/topup 让出来的新路径）
    let order_status = post_status(merged_billing_router(), "/api/user/topup/order").await;
    assert_ne!(
        order_status,
        StatusCode::NOT_FOUND,
        "充值开单路径必须存在：撞车修复是让路而非删除路由"
    );
    assert_ne!(
        order_status,
        StatusCode::METHOD_NOT_ALLOWED,
        "method 必须是 POST"
    );
    // 同上：`Json<TopUpRequest>` 提取器因 `{}` 缺字段返回 422。
    assert_eq!(
        order_status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "命中 open_topup handler：Json 提取器因缺字段返回 422"
    );
}

/// admin 手工结算路径（带 `{key}` 占位）同样必须存活。
///
/// 它与 `/api/user/topup/order` 共享 `/api/user/topup` 前缀，容易在"解冲突"
/// 时被顺手改坏，所以单独钉一条。
#[tokio::test]
async fn topup_settle_path_survives() {
    let status = post_status(
        merged_billing_router(),
        "/api/user/topup/00000000-0000-0000-0000-000000000000/settle",
    )
    .await;
    assert_ne!(status, StatusCode::NOT_FOUND, "admin 手工结算路径必须存在");
    // settle handler 只取 State/Path/HeaderMap（无 Json body 提取器），
    // 因此请求直达 `require_admin` → 无鉴权头返回 401。
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "命中 settle handler：无鉴权头 → 401"
    );
}
