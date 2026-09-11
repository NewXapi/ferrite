//! E2E: 网关消费日志 `log_type` 读写同源回归。
//!
//! 历史事故：`apps/api/src/usage.rs` 手写 `log_type: 1`（1=充值，见
//! `db/migrations/0002_usage_logs.sql` 枚举注释），而读侧 `/api/log/top`
//! 与 `/api/log/trend` 过滤 `log_type = 2`（= 消费）。每条真实消费都被标成
//! 充值，然后被两个聚合查询整体过滤掉——总览页的排行榜与趋势曲线在有
//! 真实流量时依然是空的。
//!
//! 本文件把「写侧构造器 → record 落库 → top_usage/trend 查得到」整条链钉住：
//! 任何一侧的 log_type 漂移都会让下面的断言失败，而不是报表静默变空。

use api::usage::{RecordJob, build_consume_event};
use observe::logs::{LOG_TYPE_CONSUME, LOG_TYPE_TOPUP, LogService};
use uuid::Uuid;

/// e2e 库连接；不可达时 skip（与 admin_gateway_flow 的 pg_pool 同模式）。
async fn pg_pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("FERRITE_E2E_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite_e2e".into());
    let pool = sqlx::PgPool::connect_lazy(&url).ok()?;
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => Some(pool),
        Err(e) => {
            eprintln!("skipping e2e: postgres unreachable at {url}: {e}");
            None
        }
    }
}

/// 一个够真的用量载荷：字段值本身不重要，重要的是它们被原样带进事件。
/// channel_key 用合法 UUID 字符串（usage_logs.channel_key 是 UUID 列）。
fn sample_job(model: &str) -> RecordJob {
    RecordJob {
        user_uuid: Uuid::new_v4(),
        username: "usage_log_type_it".into(),
        token_uuid: Uuid::new_v4(),
        token_name: "tk-usage-log-type".into(),
        model_name: model.into(),
        channel_key: Some(Uuid::new_v4().to_string()),
        channel_name: "ch_usage_log_type".into(),
        prompt_tokens: 11,
        completion_tokens: 7,
        cost: 18,
        use_time_ms: 240,
        is_stream: true,
        token_key: Uuid::new_v4().to_string(),
    }
}

/// 写侧翻译必须产出「消费」事件。这是最低成本的回归闸：不依赖 PG，
/// 构造器若漂回手写字面量（哪怕写成 3/4 等非充值枚举）也在此失败。
#[test]
fn build_consume_event_pins_log_type_to_consume() {
    let event = build_consume_event(&sample_job("m-pin"));
    assert_eq!(
        event.log_type, LOG_TYPE_CONSUME,
        "写侧必须用 observe 的消费常量，1 是充值——写 1 会让消费从 top/trend 消失"
    );
    assert_ne!(event.log_type, LOG_TYPE_TOPUP);
}

/// 翻译不得吞字段：量化字段要原样带到事件上（报表按它们求和）。
#[test]
fn build_consume_event_carries_quantified_fields() {
    let job = sample_job("m-pin");
    let channel_key = job.channel_key.clone().unwrap();
    let event = build_consume_event(&job);
    assert_eq!(event.prompt_tokens, 11);
    assert_eq!(event.completion_tokens, 7);
    assert_eq!(event.quota, 18);
    assert_eq!(event.use_time_ms, 240);
    assert!(event.is_stream);
    assert!(event.token_key.is_some(), "token 维度统计依赖 token_key");
    assert_eq!(event.model_name, "m-pin");
    // channel 维度归因来自 pipeline 响应 extensions（RouteAttribution）：
    // 字符串键落库前解析成 UUID；解析失败按缺失处理（NULL），不得让
    // INSERT 报类型错。
    assert!(
        event.channel_key.is_some(),
        "渠道维度统计依赖 channel_key，归因链断了会退回全 NULL"
    );
    assert_eq!(event.channel_key.unwrap().to_string(), channel_key);
    assert_eq!(event.channel_name, "ch_usage_log_type");

    // 非 UUID 的 channel_key 降级为 None（不炸整条记录）
    let bad = RecordJob {
        channel_key: Some("not-a-uuid".into()),
        ..sample_job("m-pin")
    };
    let bad_event = build_consume_event(&bad);
    assert!(bad_event.channel_key.is_none(), "非法 UUID 应按缺失处理");
    assert_eq!(bad_event.channel_name, "ch_usage_log_type");
}

/// 无归因（dispatch 前短路等）时事件与列默认值一致：channel_key NULL、
/// channel_name 空串——不允许出现"键在而名空"的半归因状态污染聚合。
#[test]
fn build_consume_event_without_attribution_leaves_channel_empty() {
    let job = RecordJob {
        channel_key: None,
        channel_name: String::new(),
        ..sample_job("m-pin")
    };
    let event = build_consume_event(&job);
    assert_eq!(event.channel_key, None);
    assert_eq!(event.channel_name, "");
}

/// 全链路：write 侧构造 → 落库 → 读侧 top_usage 与 trend 都必须看到这条消费。
///
/// 用每次运行唯一的 model 名，避免跨运行数据互染；断言只认这个 model。
#[tokio::test]
async fn consume_event_recorded_is_visible_to_top_and_trend() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    db_bootstrap::run_migrations(&pool)
        .await
        .expect("run migrations");

    // 每次运行唯一的 model 名：聚合断言只认它，天然隔离历史数据。
    let model = format!("m-logtype-{}", Uuid::new_v4().simple());
    let event = build_consume_event(&sample_job(&model));

    let svc = LogService::new(pool.clone());
    svc.record(&event).await.expect("record consume event");

    let top = svc
        .top_usage("model", None, None, 50)
        .await
        .expect("top_usage");
    let row = top.iter().find(|r| r.name == model).expect(
        "recorded consume must appear in top_usage — 若缺失，写侧 log_type 已漂离读侧过滤值",
    );
    assert_eq!(row.calls, 1);
    assert_eq!(row.tokens, 18, "tokens = prompt + completion");

    let trend = svc.trend("hour", None, None).await.expect("trend");
    let trow = trend
        .iter()
        .find(|r| r.model_name == model)
        .expect("recorded consume must appear in trend — 若缺失，写侧 log_type 已漂离读侧过滤值");
    assert_eq!(trow.calls, 1);
    assert_eq!(trow.quota, 18);
}
