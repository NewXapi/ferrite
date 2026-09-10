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

/// daily 聚合: 写两天消费日志 (+ 一条 topup) → 按天断言 requests/tokens/quota。
///
/// 口径说明: `self_daily_stats` 与 `self_stat` 一致, **不按 log_type 过滤**——
/// topup (log_type=1) 行计入当日聚合, 故下方 topup 行会拉高今天那组的 requests/quota。
#[tokio::test]
#[ignore]
async fn daily_stats_aggregation() {
    let svc = make_svc().await;
    let user = uuid::Uuid::new_v4();
    let marker = format!("m-{}", uuid::Uuid::new_v4().simple());
    let pool = sqlx::PgPool::connect(&db_url()).await.unwrap();

    // 两天数据: 2 天前写 2 条 consume, 今天写 1 条 consume + 1 条 topup。
    // 用 2 天偏移远离 make_interval(days=>30) 的 30 天窗口边界, 避免时间竞态。
    sqlx::query(
        r#"INSERT INTO usage_logs
               (log_type, user_key, username, model_name, prompt_tokens, completion_tokens, quota, created_at)
           VALUES
             (2, $1, 'alice', $2, 100, 50, 1000, now() - interval '2 day'),
             (2, $1, 'alice', $2, 200, 50, 2000, now() - interval '2 day'),
             (2, $1, 'alice', $2, 10, 10, 50, now()),
             (1, $1, 'alice', $2, 0, 0, 500000, now())"#,
    )
    .bind(user)
    .bind(&marker)
    .execute(&pool)
    .await
    .expect("seed daily rows");

    let days: Vec<observe::logs::DailyUsageStat> = svc
        .self_daily_stats(user, 30)
        .await
        .expect("self_daily_stats");
    // 两个有数据的日期, 倒序: [0]=今天, [1]=2 天前
    assert_eq!(days.len(), 2, "应只有两天有数据: {days:?}");
    let today = &days[0];
    let prev = &days[1];
    // 今天: 1 consume + 1 topup (口径同 self_stat, topup 计入)
    assert_eq!(
        today.requests, 2,
        "topup 行计入, 与 self_stat 口径一致: {today:?}"
    );
    assert_eq!(today.tokens, 20);
    assert_eq!(today.quota, 500_050, "consume 50 + topup 500000: {today:?}");
    // 2 天前: 2 条 consume
    assert_eq!(prev.requests, 2);
    assert_eq!(prev.tokens, 400);
    assert_eq!(prev.quota, 3000);
    // 日期字符串与 SQL 侧 to_char 对齐 (避免 session TZ 漂移), 且 YYYY-MM-DD 格式
    let (exp_prev, exp_today): (String, String) = sqlx::query_as(
        "SELECT to_char(now() - interval '2 day', 'YYYY-MM-DD'), to_char(now(), 'YYYY-MM-DD')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(today.date, exp_today, "{today:?}");
    assert_eq!(prev.date, exp_prev, "{prev:?}");
    assert_eq!(today.date.len(), 10);

    // 无数据用户返回空 vec
    let stranger: Vec<observe::logs::DailyUsageStat> = svc
        .self_daily_stats(uuid::Uuid::new_v4(), 30)
        .await
        .expect("stranger");
    assert!(stranger.is_empty());

    // 清理: daily 按 user_key 聚合, 残留行会污染后续断言
    sqlx::query("DELETE FROM usage_logs WHERE model_name = $1")
        .bind(&marker)
        .execute(&pool)
        .await
        .expect("cleanup");
}
