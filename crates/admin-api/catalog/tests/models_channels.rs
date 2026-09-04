//! catalog models + channels 补全端点集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p catalog --test models_channels -- --ignored --nocapture`

use std::time::Duration;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::models::{ModelService, ModelView};
use crate::channels::{ChannelService, ChannelView, CreateChannelRequest};

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into()
    })
}

async fn make_svcs() -> (ModelService, ChannelService) {
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("connect PG");
    crate::models::ensure_table(&pool).await.expect("ensure models table");
    crate::channels::ensure_table(&pool).await.expect("ensure channels table");
    (ModelService::new(pool.clone()), ChannelService::new(pool))
}

fn models_json() -> Value {
    serde_json::json!([{"alias": "gpt-4o", "upstream": "openai/gpt-4o"}])
}

// ---------- 模型 CRUD 完整流程 ----------

#[tokio::test]
#[ignore]
async fn model_crud_flow() {
    let (model_svc, channel_svc) = make_svcs().await;

    // 1. 创建模型
    let model = model_svc.create(
        "test-model-1",
        "test-owner",
        "chat",
        "https://api.example.com/v1",
        "sk-test12345678901234567890123456789012",
        serde_json::json!(["vision", "tool_call"]),
        100,
        serde_json::json!({"accuracy": 0.95}),
        0,
        8192,
        true,
        true,
        serde_json::json!(["latest", "flagship"]),
        1,
    ).await.expect("create model");
    assert_eq!(model.name, "test-model-1");
    assert_eq!(model.owner, "test-owner");
    assert_eq!(model.api_key_preview, "sk-****");
    assert!(model.is_vision);
    assert!(model.is_tool);

    // 2. 重名冲突
    let err = model_svc.create(
        "test-model-1", // same name
        "another-owner",
        "chat",
        "https://api2.example.com/v1",
        "sk-test99999999999999999999999999999999",
        serde_json::json!(["stream"]),
        50,
        serde_json::json!({}),
        0,
        4096,
        false,
        false,
        serde_json::json!([]),
        1,
    ).await.expect_err("duplicate name should fail");
    assert!(matches!(err, crate::auth::error::AuthError::Conflict(_)));

    // 3. 列表 & 搜索
    let (items, total) = model_svc.list(None, 1, 20, None, None, None).await.expect("list");
    assert!(total >= 1);
    assert!(items.iter().any(|m| m.name == "test-model-1"));
    // 搜索按 name
    let (items, _) = model_svc.list(Some("test-model"), 1, 20, None, None, None).await.expect("search");
    assert!(items.iter().any(|m| m.name == "test-model-1"));
    // 过滤 owner
    let (items, _) = model_svc.list(None, 1, 20, Some("test-owner"), None, None).await.expect("filter owner");
    assert!(items.iter().all(|m| m.owner == "test-owner"));
    // 过滤 status
    let (items, _) = model_svc.list(None, 1, 20, None, Some(1), None).await.expect("filter status");
    assert!(items.iter().all(|m| m.status == 1));
    // 过滤 model_type
    let (items, _) = model_svc.list(None, 1, 20, None, None, Some("chat")).await.expect("filter type");
    assert!(items.iter().all(|m| m.model_type == "chat"));

    // 4. 单查返回明文 api_key
    let got = model_svc.get(Uuid::parse_str(&model.key).unwrap()).await.expect("get");
    assert_eq!(got.api_key_preview, "sk-test12345678901234567890123456789012");

    // 5. 更新
    let updated = model_svc.update(
        Uuid::parse_str(&model.key).unwrap(),
        Some("test-model-1-updated"),
        None,
        None,
        Some("https://api-updated.example.com/v1"),
        Some("sk-updatedkey123456789012345678901234567890"),
        Some(serde_json::json!(["vision", "tool_call", "stream"])),
        Some(150),
        Some(serde_json::json!({"accuracy": 0.98})),
        Some(16384),
        Some(false),
        Some(false),
        Some(serde_json::json!(["updated"])),
        Some(2),
    ).await.expect("update");
    assert_eq!(updated.name, "test-model-1-updated");
    assert_eq!(updated.base_url, "https://api-updated.example.com/v1");
    assert_eq!(updated.api_key_preview, "sk-updatedkey123456789012345678901234567890");
    assert_eq!(updated.speed, 150);
    assert!(!updated.is_vision);
    assert!(!updated.is_tool);
    assert_eq!(updated.status, 2);

    // 6. 删除
    model_svc.delete(Uuid::parse_str(&model.key).unwrap()).await.expect("delete");
    let err = model_svc.get(Uuid::parse_str(&model.key).unwrap()).await.expect_err("deleted");
    assert!(matches!(err, crate::auth::error::AuthError::UserNotFound));
}

// ---------- 缺失模型检测 ----------

#[tokio::test]
#[ignore]
async fn missing_model_detection() {
    let (model_svc, channel_svc) = make_svcs().await;

    // 先创建一个模型
    let model = model_svc.create(
        "existing-model",
        "owner",
        "chat",
        "https://api.example.com/v1",
        "sk-existing12345678901234567890123456789012",
        serde_json::json!([]),
        0,
        serde_json::json!({}),
        0,
        4096,
        false,
        false,
        serde_json::json!([]),
        1,
    ).await.expect("create model");

    // 创建一个引用 existing-model 和 missing-model 的渠道
    let channel = channel_svc.create(
        "test-channel-missing",
        "openai",
        "https://api.example.com/v1",
        vec!["sk-channel12345678901234567890123456789012".into()],
        serde_json::json!([
            {"alias": "existing-model", "upstream": "existing-model"},
            {"alias": "missing-model", "upstream": "missing-model"}
        ]),
        "default",
        0,
        0,
        None,
        "test channel",
    ).await.expect("create channel");

    // 检测缺失模型
    let missing = model_svc.missing().await.expect("missing");
    assert!(missing.contains(&"missing-model".to_string()));
    assert!(!missing.contains(&"existing-model".to_string()));

    // 清理
    channel_svc.delete(Uuid::parse_str(&channel.key).unwrap()).await.ok();
    model_svc.delete(Uuid::parse_str(&model.key).unwrap()).await.ok();
}

// ---------- 渠道标签启停批量 ----------

#[tokio::test]
#[ignore]
async fn channel_tag_batch_operations() {
    let (model_svc, channel_svc) = make_svcs().await;

    // 创建三个渠道，带不同标签
    let ch1 = channel_svc.create(
        "tag-test-1", "openai", "https://api1.example.com/v1",
        vec!["sk-tag12345678901234567890123456789012".into()],
        serde_json::json!([{"alias": "gpt-4o", "upstream": "gpt-4o"}]),
        "default", 0, 0, None, "tag test",
    ).await.expect("create ch1");

    let ch2 = channel_svc.create(
        "tag-test-2", "openai", "https://api2.example.com/v1",
        vec!["sk-tag12345678901234567890123456789013".into()],
        serde_json::json!([{"alias": "gpt-4o", "upstream": "gpt-4o"}]),
        "default", 0, 0, None, "tag test",
    ).await.expect("create ch2");

    let ch3 = channel_svc.create(
        "tag-test-3", "anthropic", "https://api3.example.com/v1",
        vec!["sk-tag12345678901234567890123456789014".into()],
        serde_json::json!([{"alias": "claude-3", "upstream": "claude-3"}]),
        "default", 0, 0, None, "tag test",
    ).await.expect("create ch3");

    // 渠道模型列表 (从 api_models 读) - 需要先在 models 中创建
    let _m = model_svc.create(
        "gpt-4o", "openai", "chat", "https://api.openai.com/v1",
        "sk-gpt4o12345678901234567890123456789012",
        serde_json::json!(["vision", "tool_call"]), 100, serde_json::json!({}), 0, 8192,
        true, true, serde_json::json!(["flagship"]), 1,
    ).await.ok();

    // 测试渠道可用模型列表
    let models = channel_svc.channel_models().await.expect("channel models");
    assert!(models.iter().any(|m| m == "gpt-4o"));

    // 渠道余额查询 (stub)
    let balance = channel_svc.update_balance().await.expect("update balance");
    assert!(balance.is_object());

    // 批量停用
    let disabled = channel_svc.batch_disable_by_tag("default").await.expect("batch disable");
    assert!(disabled >= 1);

    // 批量启用
    let enabled = channel_svc.batch_enable_by_tag("default").await.expect("batch enable");
    assert!(enabled >= 1);

    // 编辑标签
    let updated = channel_svc.update_tag(Uuid::parse_str(&ch1.key).unwrap(), "new-tag").await.expect("update tag");
    assert_eq!(updated, "new-tag");

    // 批量删除
    let deleted = channel_svc.batch_delete(&["tag-test-1", "tag-test-2", "tag-test-3"]).await.expect("batch delete");
    assert_eq!(deleted, 3);

    // 清理模型
    model_svc.delete(Uuid::parse_str(&_m.key).unwrap()).await.ok();
}

// ---------- 辅助：清理残留 ----------
#[tokio::test]
#[ignore]
async fn cleanup_marker() {
    // 此测试仅作为 marker，确保每次测试运行时表存在且可连接
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url())
        .await
        .expect("connect PG");
    sqlx::query("DELETE FROM api_models WHERE name LIKE 'test-%' OR name LIKE 'tag-test-%' OR name = 'gpt-4o' OR name = 'existing-model' OR name = 'missing-model'")
        .execute(&pool).await.ok();
    sqlx::query("DELETE FROM api_channels WHERE name LIKE 'tag-test-%' OR name LIKE 'test-channel-%'")
        .execute(&pool).await.ok();
    pool.close().await;
}