//! catalog channels + groups 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p catalog --test channels_groups -- --ignored --nocapture`

use std::time::Duration;

use catalog::channels::ChannelService;
use catalog::groups::GroupService;
use sqlx::postgres::PgPoolOptions;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

async fn make_svcs() -> (ChannelService, GroupService) {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("PG connect");
    catalog::channels::ensure_table(&pool).await.expect("channels ddl");
    catalog::groups::ensure_table(&pool).await.expect("groups ddl");
    (ChannelService::new(pool.clone()), GroupService::new(pool))
}

fn models_json() -> serde_json::Value {
    serde_json::json!([{"alias": "gpt-4o", "upstream": "openai/gpt-4o"}])
}

#[tokio::test]
#[ignore]
async fn channel_crud_flow() {
    let (svc, _groups) = make_svcs().await;
    let name = format!("ch-{}", uuid::Uuid::new_v4().simple());

    // 校验: 空 keys 拒
    assert!(matches!(
        svc.create(&name, "openai", "https://api.x.com", vec![], models_json(), "default", 0, 0, None, "").await,
        Err(auth::AuthError::BadRequest(_))
    ));
    // 校验: 非 http base_url 拒
    assert!(matches!(
        svc.create(&name, "openai", "ftp://x", vec!["sk-1".into()], models_json(), "default", 0, 0, None, "").await,
        Err(auth::AuthError::BadRequest(_))
    ));

    // 创建
    let ch = svc
        .create(&name, "openai", "https://api.x.com", vec!["sk-a".into(), "sk-b".into()], models_json(), "default", 10, 5, Some("gpt-4o".into()), "")
        .await
        .expect("create");
    assert_eq!(ch.key_count, 2);
    assert_eq!(ch.keys.as_ref().unwrap().len(), 2); // 单查含完整 keys
    assert_eq!(ch.status, 1);

    // 重名 → Conflict
    assert!(matches!(
        svc.create(&name, "openai", "https://api.x.com", vec!["sk-1".into()], models_json(), "default", 0, 0, None, "").await,
        Err(auth::AuthError::Conflict(_))
    ));

    // 列表 (掩码: keys=None) + 搜索
    let (list, total) = svc.list(Some(&name), 1, 20).await.unwrap();
    assert!(total >= 1);
    let found = list.iter().find(|c| c.key == ch.key).unwrap();
    assert!(found.keys.is_none());
    assert_eq!(found.key_count, 2);

    // 更新: 改 base_url + 停用
    let key = uuid::Uuid::parse_str(&ch.key).unwrap();
    let updated = svc
        .update(key, None, None, Some("https://api.y.com"), None, None, None, Some(20), None, None, Some("r"), Some(2))
        .await
        .unwrap();
    assert_eq!(updated.base_url, "https://api.y.com");
    assert_eq!(updated.priority, 20);
    assert_eq!(updated.status, 2);

    // set_status 启用
    let enabled = svc.set_status(key, 1).await.unwrap();
    assert_eq!(enabled.status, 1);
    assert!(matches!(svc.set_status(key, 9).await, Err(auth::AuthError::BadRequest(_))));

    // 删除 + 再删 NotFound
    svc.delete(key).await.unwrap();
    assert!(matches!(svc.delete(key).await, Err(auth::AuthError::UserNotFound)));
}

#[tokio::test]
#[ignore]
async fn group_crud_flow() {
    let (_ch, groups) = make_svcs().await;
    let name = format!("grp-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);

    // default 组已 seed
    let list = groups.list().await.unwrap();
    assert!(list.iter().any(|g| g.name == "default"));

    // 校验: ratio<=0 拒
    assert!(matches!(
        groups.create(&name, 0.0, serde_json::json!([]), "").await,
        Err(auth::AuthError::BadRequest(_))
    ));
    // 校验: default 保留名拒
    assert!(matches!(
        groups.create("default", 1.0, serde_json::json!([]), "").await,
        Err(auth::AuthError::BadRequest(_))
    ));

    // 创建
    let g = groups
        .create(&name, 1.5, serde_json::json!(["gpt-4o", "claude-x"]), "vip")
        .await
        .expect("create");
    assert_eq!(g.ratio, 1.5);

    // 重名 Conflict
    assert!(matches!(
        groups.create(&name, 1.0, serde_json::json!([]), "").await,
        Err(auth::AuthError::Conflict(_))
    ));

    // 更新
    let key = uuid::Uuid::parse_str(&g.key).unwrap();
    let updated = groups.update(key, Some(2.0), None, Some("vip2"), None).await.unwrap();
    assert_eq!(updated.ratio, 2.0);
    assert_eq!(updated.remark, "vip2");

    // 有引用不可删 — 把一个用户挪进该组
    let pool = sqlx::PgPool::connect(&db_url()).await.unwrap();
    let uname = format!("u-{}", uuid::Uuid::new_v4().simple());
    sqlx::query("INSERT INTO auth_users (key, username, email, password_hash, group_id) VALUES ($1, $2, $3, 'x', $4)")
        .bind(uuid::Uuid::new_v4())
        .bind(&uname)
        .bind(format!("{uname}@x.com"))
        .bind(&name)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        groups.delete(key).await,
        Err(auth::AuthError::Conflict(_))
    ));

    // 清引用后可删
    sqlx::query("DELETE FROM auth_users WHERE username = $1").bind(&uname).execute(&pool).await.unwrap();
    groups.delete(key).await.unwrap();
    assert!(matches!(groups.delete(key).await, Err(auth::AuthError::UserNotFound)));

    // default 不可删
    let default_key = list.iter().find(|g| g.name == "default").unwrap().key.clone();
    assert!(matches!(
        groups.delete(uuid::Uuid::parse_str(&default_key).unwrap()).await,
        Err(auth::AuthError::BadRequest(_))
    ));
}
