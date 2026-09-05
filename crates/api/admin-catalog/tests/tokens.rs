//! catalog::tokens 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p catalog --test tokens -- --ignored --nocapture`
use std::time::Duration;

use auth::error::AuthError;
use catalog::tokens::TokenService;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

async fn make_svc() -> TokenService {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("PG connect");
    catalog::tokens::ensure_table(&pool).await.expect("ddl");
    TokenService::new(pool)
}

fn uid() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

#[tokio::test]
#[ignore]
async fn token_create_list_update_delete() {
    let svc = make_svc().await;
    let owner = uid();

    // 创建 — 明文 sk- 前缀, preview 掩码
    let created = svc
        .create(
            owner,
            "my-token",
            Some("default".into()),
            100_000,
            false,
            None,
        )
        .await
        .expect("create");
    assert!(created.plaintext.starts_with("sk-"));
    assert!(created.plaintext.len() > 60);
    assert!(created.token.key_preview.contains("****"));
    assert_ne!(created.token.key_preview, created.plaintext);

    // 列表 (owner 视角)
    let list = svc.list(owner, false).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "my-token");

    // 跨用户不可见
    let stranger = uid();
    assert!(svc.list(stranger, false).await.unwrap().is_empty());
    assert!(
        svc.get(
            stranger,
            uuid::Uuid::parse_str(&created.token.key).unwrap(),
            false
        )
        .await
        .is_err()
    );

    // admin_all 可见 (跨用户聚合; 表里可能有其他测试残留, 只断言包含自己这条)
    let all = svc.list(owner, true).await.unwrap();
    assert!(all.iter().any(|t| t.key == created.token.key));

    // 搜索
    let hits = svc.search(owner, false, "my-tok").await.unwrap();
    assert_eq!(hits.len(), 1);
    assert!(
        svc.search(owner, false, "no-such")
            .await
            .unwrap()
            .is_empty()
    );

    // 更新: 改名 + 禁用 + 限额
    let key = uuid::Uuid::parse_str(&created.token.key).unwrap();
    let updated = svc
        .update(
            owner,
            key,
            false,
            Some("renamed"),
            None,
            Some(55),
            Some(true),
            None,
            Some(2),
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "renamed");
    assert_eq!(updated.quota, 55);
    assert!(updated.unlimited_quota);
    assert_eq!(updated.status, 2);

    // 空 token 名拒
    assert!(matches!(
        svc.update(owner, key, false, Some("  "), None, None, None, None, None)
            .await,
        Err(auth::AuthError::BadRequest(_))
    ));

    // 删除
    svc.delete(owner, key, false).await.unwrap();
    assert!(svc.list(owner, false).await.unwrap().is_empty());
    // 再删 → NotFound
    assert!(matches!(
        svc.delete(owner, key, false).await,
        Err(AuthError::NotFound(_))
    ));
}

#[tokio::test]
#[ignore]
async fn duplicate_plaintext_hash_rejected() {
    let svc = make_svc().await;
    let owner = uid();
    let created = svc.create(owner, "t1", None, 0, true, None).await.unwrap();

    // 手工插同 hash → unique 冲突
    let res = sqlx::query(
        "INSERT INTO api_tokens (key, user_key, name, key_hash) VALUES ($1, $2, 't2', $3)",
    )
    .bind(uid())
    .bind(owner)
    .bind(sha(&created.plaintext))
    .execute(&svc_pool().await)
    .await;
    assert!(res.is_err(), "duplicate key_hash must violate unique");
}

#[tokio::test]
#[ignore]
async fn token_supplement_get_auto_groups_batch() {
    let svc = make_svc().await;
    let owner = uid();

    let created = svc.create(owner, "get_test", None, 0, true, None).await.unwrap();
    let got = svc
        .get(owner, Uuid::parse_str(&created.token.key).unwrap(), false)
        .await
        .unwrap();

    // 单查不存在 → NotFound
    let missing = svc.get(owner, uid(), false).await.unwrap_err();
    assert!(matches!(missing, AuthError::NotFound(_)));

    // auto_groups
    let groups = svc.auto_groups().await.unwrap();
    assert!(groups.iter().any(|g| g == "default"));

    // batch_create
    let batch = svc
        .batch_create(
            owner,
            vec![
                catalog::tokens::NewToken {
                    name: "b1".into(),
                    group: Some("default".into()),
                    quota: 500,
                    unlimited_quota: false,
                    expires_at: None,
                },
                catalog::tokens::NewToken {
                    name: "b2".into(),
                    group: None,
                    quota: 0,
                    unlimited_quota: true,
                    expires_at: None,
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(batch.len(), 2);
    assert!(batch.iter().all(|r| r.plaintext.starts_with("sk-")));

    // batch_keys
    let uuids: Vec<Uuid> = batch
        .iter()
        .map(|r| Uuid::parse_str(&r.token.key).unwrap())
        .collect();
    let keys = svc.batch_keys(owner, &uuids, false).await.unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|(_, p)| p.starts_with("sk-")));
}

fn sha(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

async fn svc_pool() -> sqlx::PgPool {
    sqlx::PgPool::connect(&db_url()).await.unwrap()
}
