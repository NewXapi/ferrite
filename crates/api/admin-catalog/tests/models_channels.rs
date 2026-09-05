//! catalog 集成测试（models + channels 补全）— 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p catalog --test models_channels -- --ignored --nocapture`

use std::time::Duration;

use catalog::models::ModelService;
use catalog::channels::ChannelService;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into()
    })
}

async fn make_svcs() -> (ModelService, ChannelService) {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("connect PG");
    catalog::models::ensure_table(&pool).await.expect("ensure models table");
    catalog::channels::ensure_table(&pool).await.expect("ensure channels table");
    (ModelService::new(pool.clone()), ChannelService::new(pool))
}

fn models_json() -> serde_json::Value {
    serde_json::json!([{"alias": "gpt-4o", "upstream": "openai/gpt-4o"}])
}

fn uniq(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

// ---------- 模型管理 ----------

#[tokio::test]
#[ignore]
async fn model_crud_full_flow() {
    let (model_svc, _) = make_svcs().await;
    let name = uniq("model");

    let m = model_svc
        .create(&name, "owner1", "chat", "https://api.x.com/v1", "sk-abcdef123456789012345678901234567890123456789012345678901234567890",
            serde_json::json!(["vision"]), 100, serde_json::json!({}), 8192, true, false)
        .await
        .expect("create model");
    assert_eq!(m.name, name);
    assert_eq!(m.owner, "owner1");
    let key = Uuid::parse_str(&m.key).unwrap();

    // 重名冲突
    let dup = model_svc
        .create(&name, "owner2", "chat", "https://api.x.com/v1", "sk-xyz", serde_json::json!([]), 0, serde_json::json!({}), 0, false, false)
        .await;
    assert!(matches!(dup, Err(auth::AuthError::Conflict(_))));

    // 单查
    let got = model_svc.get(key).await.expect("get");
    assert_eq!(got.key, m.key);

    // 列表 + 搜索
    let (items, total) = model_svc.list(Some(&name), None, None, None, 1, 10).await.expect("list");
    assert!(total >= 1);
    assert!(items.iter().any(|i| i.key == m.key));

    // 更新
    let updated = model_svc.update(key, Some("renamed"), None, None, None, None, None, None, None, None, None, None, Some(2)).await.expect("update");
    assert_eq!(updated.name, "renamed");
    assert_eq!(updated.status, 2);

    // 删除
    model_svc.delete(key).await.expect("delete");
    assert!(matches!(model_svc.get(key).await, Err(auth::AuthError::UserNotFound)));
}

#[tokio::test]
#[ignore]
async fn model_missing_detection() {
    let (model_svc, channel_svc) = make_svcs().await;

    let ch_name = uniq("ch-missing");
    let ch = channel_svc
        .create(&ch_name, "openai", "https://api.x.com/v1", vec!["sk-123".into()],
            serde_json::json!([{"alias": "llama-3", "upstream": "ollama/llama3"}]), "default", 0, 0, None, "")
        .await
        .expect("create channel");

    let missing = model_svc.missing_models().await.expect("missing_models");
    assert!(missing.iter().any(|m| m == "llama-3"), "expected llama-3 in missing, got {:?}", missing);

    // 清理
    channel_svc.delete(Uuid::parse_str(&ch.key).unwrap()).await.ok();
}

// ---------- 渠道补全 ----------

#[tokio::test]
#[ignore]
async fn channel_tag_batch_operations() {
    let (_, channel_svc) = make_svcs().await;
    let tag = &format!("tag-{}", Uuid::new_v4().simple());

    let mut keys = vec![];
    for i in 0..3 {
        let name = uniq(&format!("ch-tag-{}", i));
        let ch = channel_svc
            .create(&name, "openai", "https://api.x.com/v1", vec![format!("sk-{}-{}", i, "12345678901234567890123456789012345678901234567890")],
                models_json(), "default", 0, 0, None, "")
            .await
            .expect("create channel");
        keys.push(Uuid::parse_str(&ch.key).unwrap());
    }

    for k in &keys {
        channel_svc.update_tag(*k, tag).await.expect("add tag");
    }

    let disabled = channel_svc.batch_disable_by_tag(tag).await.expect("batch disable");
    assert_eq!(disabled, 3);

    let enabled = channel_svc.batch_enable_by_tag(tag).await.expect("batch enable");
    assert_eq!(enabled, 3);

    let deleted = channel_svc.batch_delete(&keys).await.expect("batch delete");
    assert_eq!(deleted, 3);

    for k in &keys {
        assert!(matches!(channel_svc.get(*k).await, Err(auth::AuthError::UserNotFound)));
    }
}

#[tokio::test]
#[ignore]
async fn channel_probe_writes_history() {
    let (_, channel_svc) = make_svcs().await;
    let pool = sqlx::PgPool::connect(&db_url()).await.expect("pool2");
    observe::monitor::ensure_table(&pool).await.expect("ensure monitor_history");
    let monitor = observe::monitor::MonitorDeps::new(pool.clone());

    let name = uniq("ch-empty");
    let ch = channel_svc
        .create(&name, "openai", "https://api.x.com/v1", vec!["sk-12345678901234567890123456789012".into()],
            models_json(), "default", 0, 0, None, "")
        .await
        .expect("create channel");

    let result = channel_svc.test_channel(&monitor, Uuid::parse_str(&ch.key).unwrap(), None).await;
    assert!(result.is_ok(), "test_channel should always Ok and record outcome");
    let probe = result.unwrap();
    assert!(!probe.ok, "invalid upstream should fail");
    assert!(!probe.error_kind.is_empty());

    let h = monitor.history(Uuid::parse_str(&ch.key).unwrap(), 10).await.expect("history");
    assert!(!h.is_empty(), "probe should have written history");

    // 清理
    channel_svc.delete(Uuid::parse_str(&ch.key).unwrap()).await.ok();
    let _ = sqlx::query("DELETE FROM monitor_history WHERE channel_key = $1")
        .bind(ch.key.to_string())
        .execute(&pool)
        .await;
}

#[tokio::test]
#[ignore]
async fn channel_update_balance_stub() {
    let (_, channel_svc) = make_svcs().await;
    let balance = channel_svc.update_balance().await.expect("balance stub");
    assert!(balance["total_balance"].is_number());
}

#[tokio::test]
#[ignore]
async fn channel_fetch_models_stub() {
    let (_, channel_svc) = make_svcs().await;
    let models = channel_svc.fetch_models(serde_json::json!({})).await.expect("fetch_models stub");
    assert!(models.is_array());
}

#[tokio::test]
#[ignore]
async fn channel_models_list() {
    let (model_svc, channel_svc) = make_svcs().await;

    let name = uniq("m-list");
    model_svc.create(&name, "o", "chat", "https://api.x.com/v1", "sk-12345678901234567890123456789012345678901234567890",
        serde_json::json!([]), 0, serde_json::json!({}), 0, false, false).await.expect("create");

    let models = channel_svc.channel_models().await.expect("channel_models");
    assert!(models.iter().any(|m| m == &name));

    let to_delete = model_svc.list(Some(&name), None, None, None, 1, 10).await.expect("list").0;
    for m in to_delete { model_svc.delete(Uuid::parse_str(&m.key).unwrap()).await.ok(); }
}

#[tokio::test]
#[ignore]
async fn model_delete_nonexistent() {
    let (model_svc, _) = make_svcs().await;
    assert!(matches!(
        model_svc.delete(Uuid::new_v4()).await,
        Err(auth::AuthError::UserNotFound)
    ));
}

#[tokio::test]
#[ignore]
async fn channel_delete_nonexistent() {
    let (_, channel_svc) = make_svcs().await;
    assert!(matches!(
        channel_svc.delete(Uuid::new_v4()).await,
        Err(auth::AuthError::UserNotFound)
    ));
}

#[tokio::test]
#[ignore]
async fn model_empty_name_rejected() {
    let (model_svc, _) = make_svcs().await;
    assert!(matches!(
        model_svc.create("", "o", "chat", "https://api.x.com/v1", "sk-12345678901234567890123456789012345678901234567890",
            serde_json::json!([]), 0, serde_json::json!({}), 0, false, false).await,
        Err(auth::AuthError::BadRequest(_))
    ));
}

#[tokio::test]
#[ignore]
async fn channel_empty_name_rejected() {
    let (_, channel_svc) = make_svcs().await;
    assert!(matches!(
        channel_svc.create("", "openai", "https://api.x.com/v1", vec!["sk-123".into()], models_json(), "default", 0, 0, None, "").await,
        Err(auth::AuthError::BadRequest(_))
    ));
}

#[tokio::test]
#[ignore]
async fn model_invalid_base_url_rejected() {
    let (model_svc, _) = make_svcs().await;
    assert!(matches!(
        model_svc.create("n", "o", "chat", "ftp://bad", "sk-12345678901234567890123456789012345678901234567890",
            serde_json::json!([]), 0, serde_json::json!({}), 0, false, false).await,
        Err(auth::AuthError::BadRequest(_))
    ));
}

#[tokio::test]
#[ignore]
async fn channel_invalid_base_url_rejected() {
    let (_, channel_svc) = make_svcs().await;
    assert!(matches!(
        channel_svc.create("n", "openai", "ftp://bad", vec!["sk-123".into()], models_json(), "default", 0, 0, None, "").await,
        Err(auth::AuthError::BadRequest(_))
    ));
}

#[tokio::test]
#[ignore]
async fn model_empty_keys_rejected() {
    let (model_svc, _) = make_svcs().await;
    assert!(matches!(
        model_svc.create("n", "o", "chat", "https://api.x.com/v1", "",
            serde_json::json!([]), 0, serde_json::json!({}), 0, false, false).await,
        Err(auth::AuthError::BadRequest(_))
    ));
}

#[tokio::test]
#[ignore]
async fn channel_empty_keys_rejected() {
    let (_, channel_svc) = make_svcs().await;
    assert!(matches!(
        channel_svc.create("n", "openai", "https://api.x.com/v1", vec![], models_json(), "default", 0, 0, None, "").await,
        Err(auth::AuthError::BadRequest(_))
    ));
}
