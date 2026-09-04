//! observe logs + dashboard 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p observe --test logs -- --ignored --nocapture`

use std::time::Duration;

use observe::logs::{LogService, UsageEvent};
use sqlx::postgres::PgPoolOptions;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

async fn make_svc() -> LogService {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("PG connect");
    observe::logs::ensure_table(&pool).await.expect("ddl");
    LogService::new(pool)
}

#[tokio::test]
#[ignore]
async fn record_and_query_flow() {
    let svc = make_svc().await;
    let user = uuid::Uuid::new_v4();
    let marker = format!("m-{}", uuid::Uuid::new_v4().simple());

    // 写 3 条 consume
    for i in 0..3 {
        let mut e = UsageEvent::consume(user, "alice", &marker);
        e.prompt_tokens = 100;
        e.completion_tokens = 50 + i;
        e.quota = 1000;
        e.use_time_ms = 800;
        e.is_stream = i % 2 == 0;
        e.request_id = format!("req-{i}");
        svc.record(&e).await.expect("record");
    }
    // 写 1 条 topup (不同类型)
    let mut topup = UsageEvent::consume(user, "alice", &marker);
    topup.log_type = 1;
    topup.quota = 500_000;
    topup.content = "topup $1".into();
    svc.record(&topup).await.expect("record topup");

    // admin 全量查 (按模型过滤)
    let (items, total) = svc
        .list_logs(None, None, None, Some(&marker), None, None, 1, 20)
        .await
        .unwrap();
    assert_eq!(total, 4);
    assert_eq!(items.len(), 4);
    // id 倒序
    assert!(items[0].id > items[1].id);

    // 按 log_type 过滤
    let (_, topup_total) = svc
        .list_logs(Some(1), None, None, Some(&marker), None, None, 1, 20)
        .await
        .unwrap();
    assert_eq!(topup_total, 1);
    let (_, consume_total) = svc
        .list_logs(Some(2), None, None, Some(&marker), None, None, 1, 20)
        .await
        .unwrap();
    assert_eq!(consume_total, 3);

    // 用户自查只看到自己的
    let stranger = uuid::Uuid::new_v4();
    let (_, stranger_total) = svc
        .list_self_logs(stranger, None, None, Some(&marker), None, None, 1, 20)
        .await
        .unwrap();
    assert_eq!(stranger_total, 0);
    let (_, self_total) = svc
        .list_self_logs(user, None, None, Some(&marker), None, None, 1, 20)
        .await
        .unwrap();
    assert_eq!(self_total, 4);

    // 分页
    let (page1, _) = svc
        .list_logs(None, None, None, Some(&marker), None, None, 1, 2)
        .await
        .unwrap();
    assert_eq!(page1.len(), 2);
    let (page2, _) = svc
        .list_logs(None, None, None, Some(&marker), None, None, 2, 2)
        .await
        .unwrap();
    assert_eq!(page2.len(), 2);
    assert_ne!(page1[0].id, page2[0].id);

    // stat (今日累计 + rpm/tpm)
    let stat = svc.stat().await.unwrap();
    assert!(stat.quota >= 500_000);
    assert!(stat.requests >= 4);

    let self_stat = svc.self_stat(user).await.unwrap();
    assert!(self_stat.quota >= 500_000);

    // dashboard 汇总
    let dash = svc.dashboard().await.unwrap();
    assert!(dash["users"].as_i64().unwrap() >= 1);
    assert!(dash["groups"].as_i64().unwrap() >= 1);
    assert!(dash["requestsToday"].as_i64().unwrap() >= 4);

    // 清理本次测试数据 (stat 是全局聚合, 残留会污染其他断言)
    sqlx::query("DELETE FROM usage_logs WHERE model_name = $1")
        .bind(&marker)
        .execute(&sqlx::PgPool::connect(&db_url()).await.unwrap())
        .await
        .expect("cleanup");
}
