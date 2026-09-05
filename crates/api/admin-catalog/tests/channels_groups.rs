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
    catalog::channels::ensure_table(&pool)
        .await
        .expect("channels ddl");
    catalog::groups::ensure_table(&pool)
        .await
        .expect("groups ddl");
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
        svc.create(
            &name,
            "openai",
            "https://api.x.com",
            vec![],
            models_json(),
            "default",
            0,
            0,
            None,
            ""
        )
        .await,
        Err(auth::AuthError::BadRequest(_))
    ));
    // 校验: 非 http base_url 拒
    assert!(matches!(
        svc.create(
            &name,
            "openai",
            "ftp://x",
            vec!["sk-1".into()],
            models_json(),
            "default",
            0,
            0,
            None,
            ""
        )
        .await,
        Err(auth::AuthError::BadRequest(_))
    ));

    // 创建
    let ch = svc
        .create(
            &name,
            "openai",
            "https://api.x.com",
            vec!["sk-a".into(), "sk-b".into()],
            models_json(),
            "default",
            10,
            5,
            Some("gpt-4o".into()),
            "",
        )
        .await
        .expect("create");
    assert_eq!(ch.key_count, 2);
    assert_eq!(ch.keys.as_ref().unwrap().len(), 2); // 单查含完整 keys
    assert_eq!(ch.status, 1);

    // 重名 → Conflict
    assert!(matches!(
        svc.create(
            &name,
            "openai",
            "https://api.x.com",
            vec!["sk-1".into()],
            models_json(),
            "default",
            0,
            0,
            None,
            ""
        )
        .await,
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
        .update(
            key,
            None,
            None,
            Some("https://api.y.com"),
            None,
            None,
            None,
            Some(20),
            None,
            None,
            Some("r"),
            Some(2),
        )
        .await
        .unwrap();
    assert_eq!(updated.base_url, "https://api.y.com");
    assert_eq!(updated.priority, 20);
    assert_eq!(updated.status, 2);

    // set_status 启用
    let enabled = svc.set_status(key, 1).await.unwrap();
    assert_eq!(enabled.status, 1);
    assert!(matches!(
        svc.set_status(key, 9).await,
        Err(auth::AuthError::BadRequest(_))
    ));

    // 删除 + 再删 NotFound
    svc.delete(key).await.unwrap();
    assert!(matches!(
        svc.delete(key).await,
        Err(auth::AuthError::NotFound(_))
    ));
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
        groups
            .create("default", 1.0, serde_json::json!([]), "")
            .await,
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
    let updated = groups
        .update(key, Some(2.0), None, Some("vip2"), None)
        .await
        .unwrap();
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
    sqlx::query("DELETE FROM auth_users WHERE username = $1")
        .bind(&uname)
        .execute(&pool)
        .await
        .unwrap();
    groups.delete(key).await.unwrap();
    assert!(matches!(
        groups.delete(key).await,
        Err(auth::AuthError::UserNotFound)
    ));

    // default 不可删
    let default_key = list
        .iter()
        .find(|g| g.name == "default")
        .unwrap()
        .key
        .clone();
    assert!(matches!(
        groups
            .delete(uuid::Uuid::parse_str(&default_key).unwrap())
            .await,
        Err(auth::AuthError::BadRequest(_))
    ));
}

// ---------- 渠道探活（mock 上游验证 monitor 落库与可用率统计） ----------

#[tokio::test]
#[ignore]
async fn probe_records_history_and_availability() {
    // 直接走 observe::monitor 层验证：record → history → availability → all
    // （HTTP 探活本身由 probe_chat_completions 对真实上游执行，单测 mock 无意义；
    //   网络路径的格式错误分支（非法 URL）由 test_channel 对 error_kind 的归类覆盖。）
    let (_ch, _g) = make_svcs().await;
    let pool = sqlx::PgPool::connect(&db_url()).await.unwrap();
    catalog::channels::ensure_table(&pool).await.unwrap();
    observe::monitor::ensure_table(&pool).await.unwrap();
    let monitor = observe::monitor::MonitorDeps::new(pool.clone());
    let key = uuid::Uuid::new_v4();

    // 3 成功 1 失败
    for (ok, latency) in [(true, 100), (true, 200), (true, 300), (false, 5000)] {
        monitor
            .record(&observe::monitor::ProbeOutcome {
                channel_key: key,
                channel_name: "ch-test".into(),
                model: "gpt-x".into(),
                ok,
                status_code: if ok { Some(200) } else { Some(500) },
                latency_ms: latency,
                error_kind: if ok { String::new() } else { "http".into() },
                message: String::new(),
            })
            .await
            .unwrap();
    }

    // 历史（新→旧）
    let h = monitor.history(key, 10).await.unwrap();
    assert_eq!(h.len(), 4);
    assert!(h[0].id > h[3].id);

    // 可用率 3/4 = 0.75，成功均值 200ms
    let a = monitor.availability(key, 7).await.unwrap();
    assert_eq!(a.total, 4);
    assert_eq!(a.ok_count, 3);
    let av = a.availability.unwrap();
    assert!((av - 0.75).abs() < 1e-9);
    let lat = a.avg_latency_ms.unwrap();
    assert!((lat - 200.0).abs() < 1e-9);

    // 全渠道一览包含该渠道
    let all = monitor.availability_all(7).await.unwrap();
    assert!(all.iter().any(|x| x.channel_key == key.to_string()));

    // 清理
    sqlx::query("DELETE FROM monitor_history WHERE channel_key = $1")
        .bind(key)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore]
async fn test_channel_rejects_bad_config() {
    // 无 keys / 无 model 的渠道探活 → BadRequest（不走网络）
    let (svc, _g) = make_svcs().await;
    let pool = sqlx::PgPool::connect(&db_url()).await.unwrap();
    observe::monitor::ensure_table(&pool).await.unwrap();
    let monitor = observe::monitor::MonitorDeps::new(pool.clone());

    // 直接插一个无 keys 渠道（create 校验会拦，这里绕过以测 test_channel 分支）
    let key = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO api_channels (key, name, channel_type, base_url, keys, models, group_name) \
         VALUES ($1, $2, 'openai', 'https://upstream.test', '[]', '[]', 'default')",
    )
    .bind(key)
    .bind(format!("ch-empty-{}", uuid::Uuid::new_v4().simple()))
    .execute(&pool)
    .await
    .unwrap();

    let err = svc.test_channel(&monitor, key, None).await.unwrap_err();
    assert!(matches!(err, auth::AuthError::BadRequest(_)));

    sqlx::query("DELETE FROM api_channels WHERE key = $1")
        .bind(key)
        .execute(&pool)
        .await
        .unwrap();
}

// ---------- token key 重取 ----------

#[tokio::test]
#[ignore]
async fn token_regenerate_key_rotates() {
    use catalog::tokens::TokenService;
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .unwrap();
    catalog::tokens::ensure_table(&pool).await.unwrap();
    let svc = TokenService::new(pool);

    let owner = uuid::Uuid::new_v4();
    let created = svc
        .create(owner, "regen-test", None, 0, true, None)
        .await
        .unwrap();
    let key = uuid::Uuid::parse_str(&created.token.key).unwrap();

    // 重取 → 新明文 ≠ 旧明文，且 sk- 前缀
    let new_key = svc.regenerate_key(owner, key, false).await.unwrap();
    assert_ne!(new_key, created.plaintext);
    assert!(new_key.starts_with("sk-"));

    // 陌生人无权重取
    let stranger = uuid::Uuid::new_v4();
    assert!(matches!(
        svc.regenerate_key(stranger, key, false).await,
        Err(auth::AuthError::NotFound(_))
    ));
    // admin 可以
    assert!(svc.regenerate_key(stranger, key, true).await.is_ok());

    svc.delete(owner, key, false).await.unwrap();
}

// ---------- 用户单查/搜索 ----------

#[tokio::test]
#[ignore]
async fn user_get_and_search() {
    use auth::service::AuthService;
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .unwrap();
    auth::ddl::run(&pool).await.unwrap();
    let svc = AuthService::new(pool, b"test-secret-must-be-long-enough-32!".to_vec()).unwrap();

    let username = format!("usr_{}", &uuid::Uuid::new_v4().simple().to_string()[..10]);
    let view = svc
        .register(&username, "hunter2hunter", None)
        .await
        .unwrap();
    let key = uuid::Uuid::parse_str(&view.key).unwrap();

    // 单查
    let got = svc.get_user(key).await.unwrap();
    assert_eq!(got.username, username);

    // 不存在 → UserNotFound
    assert!(matches!(
        svc.get_user(uuid::Uuid::new_v4()).await,
        Err(auth::AuthError::UserNotFound)
    ));

    // 搜索命中
    let hits = svc.search_users(&username).await.unwrap();
    assert!(hits.iter().any(|u| u.key == view.key));
}
