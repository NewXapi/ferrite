//! observe logs + dashboard 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p observe --test logs -- --ignored --nocapture`

use std::time::Duration;

use observe::logs::{LOG_TYPE_CONSUME, LOG_TYPE_ERROR, LogService, UsageEvent};
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
    db_bootstrap::run_migrations(&pool)
        .await
        .expect("migrations");
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

#[tokio::test]
#[ignore]
async fn top_usage_and_trend_aggregate_consume_rows() {
    let svc = make_svc().await;
    let user = uuid::Uuid::new_v4();
    let marker_a = format!("ta-{}", uuid::Uuid::new_v4().simple());
    let marker_b = format!("tb-{}", uuid::Uuid::new_v4().simple());

    let mut a = UsageEvent::consume(user, "alice_agg", &marker_a);
    a.prompt_tokens = 100;
    a.completion_tokens = 50;
    a.quota = 300;
    svc.record(&a).await.expect("record a");

    let mut b = UsageEvent::consume(user, "bob_agg", &marker_b);
    b.prompt_tokens = 10;
    b.completion_tokens = 5;
    b.quota = 30;
    svc.record(&b).await.expect("record b");

    let models = svc.top_usage("model", None, None, 10).await.unwrap();
    let a_row = models.iter().find(|r| r.name == marker_a).expect("model a");
    assert_eq!(a_row.tokens, 150);
    assert_eq!(a_row.calls, 1);

    let users = svc.top_usage("user", None, None, 10).await.unwrap();
    assert!(
        users
            .iter()
            .any(|r| r.name == "alice_agg" && r.tokens == 150)
    );

    let trend = svc.trend("hour", None, None).await.unwrap();
    assert!(
        trend
            .iter()
            .any(|r| r.model_name == marker_a && r.tokens == 150)
    );

    sqlx::query("DELETE FROM usage_logs WHERE model_name IN ($1, $2)")
        .bind(&marker_a)
        .bind(&marker_b)
        .execute(&sqlx::PgPool::connect(&db_url()).await.unwrap())
        .await
        .expect("cleanup");
}

/// 整秒时刻：PG timestamptz 只有微秒精度，chrono now() 带纳秒会破坏
/// last_seen/窗口边界的精确相等断言，先截到整秒。
fn whole_second_now() -> chrono::DateTime<chrono::Utc> {
    let now = chrono::Utc::now();
    now - chrono::Duration::nanoseconds(now.timestamp_subsec_nanos() as i64)
}

/// 直插一条钉死 created_at 的日志行。`record()` 不接受时间参数（落库即 now），
/// 窗口聚合测试必须控制 created_at 才能构造"上一窗/当前窗"两窗数据，
/// 故绕过 record() 直接 INSERT（列带默认值，仅需必填集）。
async fn insert_windowed_row(
    pool: &sqlx::PgPool,
    log_type: i16,
    user_key: uuid::Uuid,
    model_name: &str,
    prompt: i32,
    completion: i32,
    created_at: chrono::DateTime<chrono::Utc>,
) {
    sqlx::query(
        r#"INSERT INTO usage_logs
             (log_type, user_key, model_name, prompt_tokens, completion_tokens, created_at)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(log_type)
    .bind(user_key)
    .bind(model_name)
    .bind(prompt)
    .bind(completion)
    .bind(created_at)
    .execute(pool)
    .await
    .expect("insert windowed row");
}

/// 测什么：top_usage 的 previousTokens = 同口径上一等长窗口的同实体 tokens。
/// 为什么：总览榜单要做环比（本期 vs 上期），窗口算法
/// `[start-(end-start), start)` 必须与实现钉死——两窗都写数据、逐值断言，
/// 并覆盖"上窗无数据填 0"与"start 缺省恒 0"两个边界。
#[tokio::test]
#[ignore]
async fn top_usage_previous_tokens_compares_prior_equal_length_window() {
    let svc = make_svc().await;
    let pool = sqlx::PgPool::connect(&db_url()).await.expect("pg");
    let user = uuid::Uuid::new_v4();
    let marker = format!("pt-{}", uuid::Uuid::new_v4().simple()); // 两窗都有
    let marker_old = format!("po-{}", uuid::Uuid::new_v4().simple()); // 只在上窗
    let marker_new = format!("pn-{}", uuid::Uuid::new_v4().simple()); // 只在当前窗
    let base = whole_second_now();
    let cur_start = base - chrono::Duration::hours(2);
    let cur_end = base - chrono::Duration::hours(1);
    // 上一窗 = [cur_start - 1h, cur_start)（与当前窗等长）

    // 当前窗：marker 1 条 ×100 token；marker_new 1 条 ×10 token
    insert_windowed_row(
        &pool,
        LOG_TYPE_CONSUME,
        user,
        &marker,
        60,
        40,
        cur_start + chrono::Duration::minutes(10),
    )
    .await;
    insert_windowed_row(
        &pool,
        LOG_TYPE_CONSUME,
        user,
        &marker_new,
        10,
        0,
        cur_end - chrono::Duration::minutes(1),
    )
    .await;
    // 上一窗：marker 3 条 ×100 = 300；marker_old 2 条 ×50 = 100
    for offset in [50, 40, 30] {
        insert_windowed_row(
            &pool,
            LOG_TYPE_CONSUME,
            user,
            &marker,
            60,
            40,
            cur_start - chrono::Duration::minutes(offset),
        )
        .await;
    }
    for offset in [20, 10] {
        insert_windowed_row(
            &pool,
            LOG_TYPE_CONSUME,
            user,
            &marker_old,
            30,
            20,
            cur_start - chrono::Duration::minutes(offset),
        )
        .await;
    }

    // 当前窗榜单：tokens 为当前窗聚合，previous_tokens 为上一窗聚合
    let rows = svc
        .top_usage("model", Some(cur_start), Some(cur_end), 50)
        .await
        .expect("top_usage with window");
    let row = rows
        .iter()
        .find(|r| r.name == marker)
        .expect("marker 在当前窗");
    assert_eq!(row.tokens, 100, "当前窗 1 条 ×100");
    assert_eq!(row.previous_tokens, 300, "上一窗 3 条 ×100");
    let new_row = rows
        .iter()
        .find(|r| r.name == marker_new)
        .expect("marker_new 在当前窗");
    assert_eq!(new_row.tokens, 10);
    assert_eq!(new_row.previous_tokens, 0, "上窗无该实体填 0");
    assert!(
        !rows.iter().any(|r| r.name == marker_old),
        "只在上窗出现的实体不得进当前窗榜单"
    );

    // start 缺省 → 当前窗口无下界、不可定时 → previous_tokens 恒 0（不抛错）
    let no_start = svc.top_usage("model", None, None, 50).await.unwrap();
    let r = no_start
        .iter()
        .find(|r| r.name == marker)
        .expect("start 缺省时 marker 仍在榜");
    assert_eq!(r.previous_tokens, 0, "start 缺省无上一窗可言，恒 0");

    // 清理（本测试所有行共用一次性 user_key，按 user 删最精准）
    sqlx::query("DELETE FROM usage_logs WHERE user_key = $1")
        .bind(user)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// 测什么：error_stats 按 log_type=5 + model_name 分组聚合，
/// count 降序、lastSeenAt=窗口内最后一次错误时刻、窗口外/错误类型/空模型名剔除、
/// limit 取 count 最高的前 N。
/// 为什么：这是排障榜的数据源，分组口径与排序稳定性直接决定前端断言。
#[tokio::test]
#[ignore]
async fn error_stats_groups_by_model_and_respects_window_and_limit() {
    let svc = make_svc().await;
    let pool = sqlx::PgPool::connect(&db_url()).await.expect("pg");
    let user = uuid::Uuid::new_v4();
    let m_hot = format!("err-hot-{}", uuid::Uuid::new_v4().simple());
    let m_cold = format!("err-cold-{}", uuid::Uuid::new_v4().simple());
    let base = whole_second_now();

    // 窗口 1h 内：m_hot 3 条（-50/-40/-30min），m_cold 1 条（-20min）
    for offset in [50, 40, 30] {
        insert_windowed_row(
            &pool,
            LOG_TYPE_ERROR,
            user,
            &m_hot,
            0,
            0,
            base - chrono::Duration::minutes(offset),
        )
        .await;
    }
    insert_windowed_row(
        &pool,
        LOG_TYPE_ERROR,
        user,
        &m_cold,
        0,
        0,
        base - chrono::Duration::minutes(20),
    )
    .await;
    // 三类干扰行：窗口外的 m_hot 错误（-2h）、窗口内的 m_hot 消费（log_type=2）、
    // 空模型名错误 —— 都不得计入聚合
    insert_windowed_row(
        &pool,
        LOG_TYPE_ERROR,
        user,
        &m_hot,
        0,
        0,
        base - chrono::Duration::hours(2),
    )
    .await;
    insert_windowed_row(
        &pool,
        LOG_TYPE_CONSUME,
        user,
        &m_hot,
        1,
        1,
        base - chrono::Duration::minutes(25),
    )
    .await;
    insert_windowed_row(
        &pool,
        LOG_TYPE_ERROR,
        user,
        "",
        0,
        0,
        base - chrono::Duration::minutes(15),
    )
    .await;

    let rows = svc.error_stats(1, 10).await.expect("error_stats");
    let hot = rows
        .iter()
        .find(|r| r.model_name == m_hot)
        .expect("hot 在列");
    assert_eq!(
        hot.count, 3,
        "只数窗口内 log_type=5 的行（排除窗口外与 consume）"
    );
    assert_eq!(
        hot.last_seen_at,
        base - chrono::Duration::minutes(30),
        "lastSeenAt = 窗口内最后一次错误时刻（精确到整秒）"
    );
    let cold = rows
        .iter()
        .find(|r| r.model_name == m_cold)
        .expect("cold 在列");
    assert_eq!(cold.count, 1);
    assert!(
        rows.iter().all(|r| !r.model_name.is_empty()),
        "空模型名不得单独成组"
    );
    let hot_pos = rows.iter().position(|r| r.model_name == m_hot).unwrap();
    let cold_pos = rows.iter().position(|r| r.model_name == m_cold).unwrap();
    assert!(hot_pos < cold_pos, "count 降序：hot(3) 排在 cold(1) 之前");

    // limit 生效：只要 count 最高的 1 行（clamp 到 1..=50 的下界场景）
    let top1 = svc.error_stats(1, 1).await.expect("error_stats limit=1");
    assert_eq!(top1.len(), 1);
    assert_eq!(top1[0].model_name, m_hot);
    assert_eq!(top1[0].count, 3);

    // 清理
    sqlx::query("DELETE FROM usage_logs WHERE user_key = $1")
        .bind(user)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// 测什么：dashboard 的 quotaRemaining 只汇总 status=1（启用）用户的 quota；
/// asOf 必须存在且为非空字符串（Freshness 激活）。
/// 为什么：禁用/封禁用户的残留余额不可消费，计入会高估平台可用额度；
/// delta 断言（插入前后差值）把全局聚合变成可控增量——本测试新增的
/// status=2 用户贡献必须为 0。其他测试不会新增带 quota 的用户
/// （注册默认 quota=0），且 cargo 各测试二进制串行执行，delta 稳定。
#[tokio::test]
#[ignore]
async fn dashboard_quota_remaining_sums_enabled_users_only() {
    let svc = make_svc().await;
    let pool = sqlx::PgPool::connect(&db_url()).await.expect("pg");
    let before = svc.dashboard().await.unwrap();

    let key_on = uuid::Uuid::new_v4();
    let key_off = uuid::Uuid::new_v4();
    let suffix = uuid::Uuid::new_v4().simple();
    // 启用用户：quota=12345，必须计入 delta
    sqlx::query("INSERT INTO auth_users (key, username, password_hash, status, quota) VALUES ($1, $2, 'test-only-hash', 1, 12345)")
        .bind(key_on)
        .bind(format!("qro-{suffix}"))
        .execute(&pool)
        .await
        .expect("insert enabled user");
    // 禁用用户（status=2）：quota=999999，不得计入 delta
    sqlx::query("INSERT INTO auth_users (key, username, password_hash, status, quota) VALUES ($1, $2, 'test-only-hash', 2, 999999)")
        .bind(key_off)
        .bind(format!("qrx-{suffix}"))
        .execute(&pool)
        .await
        .expect("insert disabled user");

    let after = svc.dashboard().await.unwrap();
    let delta =
        after["quotaRemaining"].as_i64().unwrap() - before["quotaRemaining"].as_i64().unwrap();
    assert_eq!(
        delta, 12345,
        "quotaRemaining 只汇总 status=1 用户：启用 +12345，禁用 +999999 必须缺席"
    );
    // Freshness 激活：asOf 为非空 ISO8601/RFC3339 字符串
    assert!(
        after["asOf"].as_str().is_some_and(|s| !s.is_empty()),
        "dashboard 必须携带非空 asOf"
    );
    // 既有字段不回归：两次读数间 users 至少 +2（本测试插入的两行）
    assert!(
        after["users"].as_i64().unwrap() - before["users"].as_i64().unwrap() >= 2,
        "既有 users 字段语义不变"
    );

    // 清理
    sqlx::query("DELETE FROM auth_users WHERE key = ANY($1)")
        .bind(vec![key_on, key_off])
        .execute(&pool)
        .await
        .expect("cleanup");
}
