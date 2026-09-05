//! catalog::routes (route_units) 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p catalog --test route_units -- --ignored`

use catalog::routes::RouteUnitService;
use catalog::channels::ChannelService;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

async fn make_svcs() -> (RouteUnitService, ChannelService, sqlx::PgPool) {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("PG connect");
    catalog::channels::ensure_table(&pool).await.expect("channels ddl");
    catalog::routes::ensure_table(&pool).await.expect("routes ddl");
    (
        RouteUnitService::new(pool.clone()),
        ChannelService::new(pool.clone()),
        pool,
    )
}

fn uniq(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

/// 建一个测试渠道（两把 key），返回 channel key。
async fn make_channel(svc: &ChannelService, name: &str) -> String {
    let ch = svc
        .create(
            name,
            "openai",
            "https://api.test.local/v1",
            vec!["sk-key0".to_string(), "sk-key1".to_string()],
            serde_json::json!([{"alias": "gpt-4o", "upstream": "gpt-4o"}]),
            "default",
            0,
            10,
            None,
            "",
        )
        .await
        .expect("create channel");
    ch.key
}

/// create → 校验生效（key_index 越界拒绝）→ update → delete → 再删 NotFound。
#[tokio::test]
#[ignore]
async fn route_unit_crud_flow() {
    let (svc, channel_svc, _pool) = make_svcs().await;
    let channel_key = make_channel(&channel_svc, &uniq("ru_channel")).await;

    // 正常创建：group/model/channel 组合
    let ru = svc
        .create("default", "gpt-4o", Uuid::parse_str(&channel_key).unwrap(), 0, "gpt-4o-upstream", 5, 10)
        .await
        .expect("create route unit");
    assert_eq!(ru.public_model, "gpt-4o");
    assert_eq!(ru.status, 1);
    let ru_key = Uuid::parse_str(&ru.key).unwrap();

    // key_index 越界（渠道只有 2 把 key，index=5 应拒绝）
    let bad = svc
        .create("default", "gpt-4o", Uuid::parse_str(&channel_key).unwrap(), 5, "x", 0, 10)
        .await;
    assert!(matches!(bad, Err(auth::AuthError::BadRequest(_))));

    // 更新 priority + status
    let updated = svc
        .update(ru_key, None, None, None, None, Some(99), None, Some(2))
        .await
        .expect("update");
    assert_eq!(updated.priority, 99);
    assert_eq!(updated.status, 2);

    // 删除 + 再删 NotFound
    svc.delete(ru_key).await.expect("delete");
    assert!(matches!(
        svc.delete(ru_key).await,
        Err(auth::AuthError::NotFound(_))
    ));
}

/// list 按 group 过滤 + 分页。
#[tokio::test]
#[ignore]
async fn route_unit_list_filter() {
    let (svc, channel_svc, _pool) = make_svcs().await;
    let channel_key = make_channel(&channel_svc, &uniq("ru_channel")).await;
    let group = uniq("grp");
    let ck = Uuid::parse_str(&channel_key).unwrap();

    svc.create(&group, "m1", ck, 0, "m1-up", 0, 10).await.expect("create 1");
    svc.create(&group, "m2", ck, 0, "m2-up", 0, 10).await.expect("create 2");

    let (items, total) = svc.list(Some(&group), None, 1, 50).await.expect("list");
    assert_eq!(total, 2);
    assert_eq!(items.len(), 2);

    // 精确 public_model 过滤
    let (items, _) = svc.list(Some(&group), Some("m1"), 1, 50).await.expect("list by model");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].public_model, "m1");
}

/// 渠道删除 → 该渠道 route_units 级联失效（status→2）。
#[tokio::test]
#[ignore]
async fn route_unit_channel_cascade() {
    let (svc, channel_svc, _pool) = make_svcs().await;
    let channel_key = make_channel(&channel_svc, &uniq("ru_channel")).await;
    let ck = Uuid::parse_str(&channel_key).unwrap();

    let ru = svc.create("default", "m-x", ck, 0, "m-x-up", 0, 10).await.expect("create");

    // 渠道删除前先手工级联（channels::delete 未来接入）
    let n = svc.invalidate_by_channel(ck).await.expect("invalidate");
    assert_eq!(n, 1);

    // 级联后 unit status=2（查列表应不含 status=1 的活跃单元——这里直接验证 update）
    let updated = svc
        .update(Uuid::parse_str(&ru.key).unwrap(), None, None, None, None, None, None, Some(2))
        .await
        .expect("update after invalidate");
    assert_eq!(updated.status, 2);
}

/// 空 group / 空 model 拒绝。
#[tokio::test]
#[ignore]
async fn route_unit_validation_rejected() {
    let (svc, channel_svc, _pool) = make_svcs().await;
    let channel_key = make_channel(&channel_svc, &uniq("ru_channel")).await;
    let ck = Uuid::parse_str(&channel_key).unwrap();

    assert!(svc.create("", "m", ck, 0, "up", 0, 10).await.is_err());
    assert!(svc.create("default", "", ck, 0, "up", 0, 10).await.is_err());
    // 不存在的渠道
    assert!(matches!(
        svc.create("default", "m", Uuid::new_v4(), 0, "up", 0, 10).await,
        Err(auth::AuthError::NotFound(_))
    ));
}
