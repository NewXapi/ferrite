//! 计费权威切换回归 —— pipeline 结算 sink 侧的装配逻辑。
//!
//! 权威裁决背景（billing.rs 模块文档）：pipeline 结算是唯一扣费写点，usage
//! 中间件已退役。本文件钉住 apps 侧三块纯逻辑 + 一条 PG 全链路：
//! 1. [`api::billing::PgPriceTable`]：model 命中 + 组倍率折算（vip ratio=0.8 →
//!    `group_multiplier` = 表值 × 0.8）；未知模型 → None（库层缺价免费落账，
//!    对齐 new-api）；组缺失 → 中性 1.0；
//! 2. [`api::billing::NameDirectory`]：UUID → 展示名命中/缺失回落空串；
//! 3. `build_consume_event` 的 log_type 钉住回归在 `tests/usage_log_type.rs`
//!    （同源搬运，此处不重复）；
//! 4. （PG-skip）[`api::billing::PgSettleSink::submit`] 落一行 usage_logs +
//!    递增 `api_tokens.used_quota` + 扣内存 quota 快照。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use api::billing::{NameDirectory, PgPriceTable, PgSettleSink, build_consume_event};
use arc_swap::ArcSwap;
use chrono::Utc;
use contract::records::{SyncMeta, TokenRecord, UsageEventRecord, UserRecord};
use gateway_gate::snapshot::{GroupEntry, GroupSnapshot, QuotaSnapshot};
use metering::SettleSink;
use metering::pricing::PriceTable;
use uuid::Uuid;

// ============================================================================
// 离线纯逻辑
// ============================================================================

/// 组快照：vip 组倍率 0.8（对应 api_groups.ratio，#159 装进快照）。
fn group_snapshot() -> gateway_gate::snapshot::SharedGroupSnapshot {
    let mut snapshot = GroupSnapshot::default();
    // GroupSnapshot::upsert 会校验倍率合法性，0.8 是合法值原样入库
    snapshot.upsert(
        "vip".to_string(),
        GroupEntry {
            allowed_models: vec![],
            multiplier: 0.8,
            enabled: true,
        },
    );
    Arc::new(ArcSwap::from_pointee(snapshot))
}

/// 价格表：一行 m-claude（input 2.0 / output 4.0 / cache 1.0，$/M tokens）。
fn price_table() -> PgPriceTable {
    PgPriceTable::new(
        Arc::new(ArcSwap::from_pointee(vec![(
            "m-claude".to_string(),
            2.0,
            4.0,
            1.0,
        )])),
        group_snapshot(),
    )
}

/// 组倍率必须折算进 lookup 返回值（组倍率唯一来源 api_groups.ratio，
/// 价格行基值恒 1.0，见迁移 0003 头注释）。
#[test]
fn price_table_multiplies_group_ratio_into_lookup() {
    let pt = price_table();
    let vip = pt.lookup("m-claude", "vip").expect("表内模型必须有价");
    assert_eq!(vip.input, 2.0);
    assert_eq!(vip.output, 4.0);
    assert_eq!(vip.cache, 1.0);
    assert_eq!(
        vip.group_multiplier, 0.8,
        "vip 组 ratio=0.8 必须乘进 group_multiplier"
    );

    // 端到端乘法口径：cost = tokens × 单价 × group_multiplier × group_ratio。
    // forward 库层传的 group_ratio 恒 1.0（组倍率已折进表值），100 万输入
    // token × $2/M × 0.8 = $1.6 = 800_000 内部单位（500_000 单位 = $1）。
    let cost = metering::pricing::price_of(
        metering::scanner::TokenCounts {
            prompt: 1_000_000,
            completion: 0,
            cached: 0,
        },
        &vip,
        1.0,
    );
    assert_eq!(cost, 800_000, "cost 必须等于 单价 × 组倍率 × 500_000");
}

/// 未知模型 → None：库层 settle_event 对 None 走缺价免费（input=0）落账，
/// 对齐 new-api 缺价 = 0 语义。
#[test]
fn price_table_unknown_model_is_none() {
    let pt = price_table();
    assert!(
        pt.lookup("no-such-model", "vip").is_none(),
        "未知模型必须 None，让库层按缺价免费落账"
    );
}

/// 组缺失 → 中性 1.0：模型命中与组无关，缺组不放大也不拒绝计费。
#[test]
fn price_table_missing_group_falls_back_to_neutral() {
    let pt = price_table();
    let plain = pt
        .lookup("m-claude", "no-such-group")
        .expect("模型命中不应依赖组存在");
    assert_eq!(plain.group_multiplier, 1.0, "组缺失回落中性 1.0");
}

/// 构造 SyncMeta（schema_version 用真实常量，其余字段对本测试无语义）。
fn sync_meta(key: &str) -> SyncMeta {
    SyncMeta {
        key: key.to_string(),
        schema_version: contract::SCHEMA_VERSION,
        logical_version: 1,
        origin: "test".into(),
        updated_at: Utc::now(),
    }
}

/// 名单目录：命中取名，缺失回落空串（usage_logs 冗余展示列的默认值）。
#[test]
fn name_directory_hits_and_falls_back_to_empty() {
    let tokens = vec![TokenRecord {
        meta: sync_meta("tok-uuid-1"),
        user_key: "user-uuid-1".into(),
        name: "tk-alpha".into(),
        key_hash: "00".repeat(32),
        key_preview: "sk-****".into(),
        group: None,
        quota: 1000,
        unlimited_quota: false,
        used_quota: 0,
        expires_at: None,
        status: 1,
    }];
    let users = vec![UserRecord {
        meta: sync_meta("user-uuid-1"),
        username: "alice".into(),
        display_name: "Alice".into(),
        email: String::new(),
        quota: 100,
        used_quota: 0,
        request_count: 0,
        group: "default".into(),
        status: 1,
        role: 1,
        created_at: Utc::now(),
    }];
    let dir = NameDirectory::new(&tokens, &users);
    assert_eq!(dir.token_name("tok-uuid-1"), "tk-alpha");
    assert_eq!(dir.username("user-uuid-1"), "alice");
    assert_eq!(dir.token_name("missing-token"), "", "缺失 token 回落空串");
    assert_eq!(dir.username("missing-user"), "", "缺失用户回落空串");
}

// ============================================================================
// PG 全链路（PG 不可达即 skip，与 tests/usage_log_type.rs 同模式）
// ============================================================================

/// e2e 库连接；不可达时 skip。
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

/// usage_logs 行的断言投影（与测试查询的列序一致）。
type UsageLogRow = (
    i32,
    i32,
    i64,
    bool,
    String,
    String,
    String,
    Option<uuid::Uuid>,
);

/// 轮询等待某 model 的结算行落库：submit 是 spawn-and-forget，行由后台
/// 任务写入（10s 预算；PG 就绪时通常 <100ms，超时即失败）。
async fn wait_log_row(pool: &sqlx::PgPool, model: &str) -> UsageLogRow {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let row = sqlx::query_as::<_, UsageLogRow>(
            "SELECT prompt_tokens, completion_tokens, quota, is_stream, username, token_name, \
             channel_name, token_key \
             FROM usage_logs WHERE model_name = $1",
        )
        .bind(model)
        .fetch_optional(pool)
        .await
        .expect("query usage_logs");
        if let Some(row) = row {
            return row;
        }
        assert!(
            Instant::now() < deadline,
            "结算事件 10s 内未落 usage_logs（model={model}）：sink 落地链路断了"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// sink 全链路：submit 一条结算事件 → usage_logs 落一行（含冗余展示字段、
/// is_stream 透传事件值）→ api_tokens.used_quota 递增 → 内存 quota 快照扣减。
/// 第二阶段钉住渠道改名热更：向 sink 持有的 SharedChannelNames 句柄 store
/// 新映射（与 `apply_snapshot_reload` 同一动作），下一条事件落新名。
///
/// submit 是 spawn-and-forget，测试轮询 usage_logs 直到该 model 出现（每次
/// 运行唯一 model 名天然隔离历史数据），超时即失败。
#[tokio::test]
async fn settle_sink_records_usage_log_and_updates_used_quota() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    db_bootstrap::run_migrations(&pool)
        .await
        .expect("run migrations");

    let token_uuid = Uuid::new_v4();
    let user_uuid = Uuid::new_v4();
    let channel_uuid = Uuid::new_v4();
    let token_key = token_uuid.to_string();
    let model = format!("m-ba-{}", Uuid::new_v4().simple());
    let cost = 123_i64;

    // api_tokens 行：used_quota 断言需要真实行（UPDATE 按 UUID key 命中）；
    // key_hash 列 UNIQUE，用一次性随机串避免跨运行冲突。
    sqlx::query("INSERT INTO api_tokens (key, user_key, name, key_hash) VALUES ($1, $2, $3, $4)")
        .bind(token_uuid)
        .bind(user_uuid)
        .bind("tk-billing-authority")
        .bind(format!("ba-{}", Uuid::new_v4().simple()))
        .execute(&pool)
        .await
        .expect("insert api_tokens row");

    // 名单目录（boot 快照等价物）：token/user 名字由记录构建
    let tokens = vec![TokenRecord {
        meta: sync_meta(&token_key),
        user_key: user_uuid.to_string(),
        name: "tk-billing-authority".into(),
        key_hash: "00".repeat(32),
        key_preview: "sk-****".into(),
        group: None,
        quota: 1000,
        unlimited_quota: false,
        used_quota: 0,
        expires_at: None,
        status: 1,
    }];
    let users = vec![UserRecord {
        meta: sync_meta(&user_uuid.to_string()),
        username: "billing_authority_it".into(),
        display_name: "BA IT".into(),
        email: String::new(),
        quota: 100,
        used_quota: 0,
        request_count: 0,
        group: "default".into(),
        status: 1,
        role: 1,
        created_at: Utc::now(),
    }];

    // 内存 quota 快照：预置 1000，结算后应扣掉 cost
    let quota = QuotaSnapshot::default();
    quota.upsert(token_key.clone(), 1000);
    let quota_snapshot: gateway_gate::snapshot::SharedQuota =
        Arc::new(ArcSwap::from_pointee(quota));

    let mut channel_names_map = HashMap::new();
    channel_names_map.insert(channel_uuid.to_string(), "ch-billing-authority".to_string());
    // 与 boot 装配同形：sink 持共享句柄（SharedChannelNames），不是 clone 的定稿
    // HashMap——reload store 新映射后 submit 现读即生效（改名免重启）。
    let channel_names: api::snapshot::SharedChannelNames =
        Arc::new(ArcSwap::from_pointee(channel_names_map));

    let sink = PgSettleSink::new(
        pool.clone(),
        quota_snapshot.clone(),
        channel_names.clone(),
        Arc::new(ArcSwap::from_pointee(NameDirectory::new(&tokens, &users))),
    );

    let make_event = |model: &str| UsageEventRecord {
        meta: sync_meta(&Uuid::new_v4().to_string()),
        token_key: token_key.clone(),
        user_key: user_uuid.to_string(),
        channel_key: channel_uuid.to_string(),
        route_unit_key: "ru-test".into(),
        public_model: model.to_string(),
        upstream_model: format!("{model}-upstream"),
        prompt_tokens: 11,
        completion_tokens: 7,
        cached_tokens: 0,
        is_stream: false,
        first_token_ms: 10,
        duration_ms: 240,
        cost,
        status_code: 200,
        error: None,
    };

    sink.submit(make_event(&model));

    // 轮询等待 spawn 的后台落地完成（10s 预算；PG 就绪时通常 <100ms）
    let row = wait_log_row(&pool, &model).await;

    // 落库载荷与事件逐字段一致（量化字段原样带入、冗余展示名来自名单目录、
    // is_stream 透传事件的流式标记，本事件为非流式 false）
    let (
        prompt,
        completion,
        quota_col,
        is_stream,
        username,
        token_name,
        channel_name,
        token_key_col,
    ) = row;
    assert_eq!(prompt, 11);
    assert_eq!(completion, 7);
    assert_eq!(quota_col, cost, "usage_logs.quota 必须等于事件 cost");
    assert!(!is_stream, "事件 is_stream=false 应原样透传到 usage_logs");
    assert_eq!(username, "billing_authority_it");
    assert_eq!(token_name, "tk-billing-authority");
    assert_eq!(channel_name, "ch-billing-authority");
    assert_eq!(
        token_key_col.map(|u| u.to_string()).as_deref(),
        Some(token_key.as_str()),
        "合法 UUID token_key 必须挂上 token 维度"
    );

    // used_quota 增量维护（缓存态，权威在 usage_logs）
    let used_quota: i64 = sqlx::query_scalar("SELECT used_quota FROM api_tokens WHERE key = $1")
        .bind(token_uuid)
        .fetch_one(&pool)
        .await
        .expect("api_tokens row must exist");
    assert_eq!(used_quota, cost, "used_quota 必须按事件 cost 递增");

    // 内存 quota 快照扣减（与 QuotaGate 预检同桶）
    assert_eq!(
        quota_snapshot.load().remaining(&token_key),
        1000 - cost,
        "quota 快照必须按事件 cost 扣减"
    );

    // 渠道改名热更（新名经 reload 生效）：向 sink 持有的同一名单句柄 store
    // 新映射（与 apply_snapshot_reload 的 store 同一动作），不重建 sink；
    // 下一条结算事件的 channel_name 必须是新名。
    let renamed_model = format!("m-ba-renamed-{}", Uuid::new_v4().simple());
    channel_names.store(Arc::new(HashMap::from([(
        channel_uuid.to_string(),
        "ch-renamed-hot".to_string(),
    )])));
    sink.submit(make_event(&renamed_model));
    let (_, _, renamed_quota, _, _, _, renamed_channel, _) =
        wait_log_row(&pool, &renamed_model).await;
    assert_eq!(
        renamed_channel, "ch-renamed-hot",
        "store 新名单后 sink 必须现读到新渠道名（改名免重启）"
    );
    assert_eq!(renamed_quota, cost, "改名后事件 cost 口径不变");

    // 清理本次插入的 token 行（usage_logs 行留下，与 usage_log_type.rs 同策略：
    // 用唯一 model 名隔离，不做清理）
    sqlx::query("DELETE FROM api_tokens WHERE key = $1")
        .bind(token_uuid)
        .execute(&pool)
        .await
        .expect("cleanup api_tokens row");
}

/// 价格表持共享句柄：reload store 新行后 lookup 即读到新价（热更无需重启）。
#[test]
fn price_table_reflects_reload_without_restart() {
    let rows = Arc::new(ArcSwap::from_pointee(vec![(
        "m-hot".to_string(),
        1.0,
        2.0,
        0.0,
    )]));
    let pt = PgPriceTable::new(rows.clone(), group_snapshot());
    assert_eq!(pt.lookup("m-hot", "default").expect("boot 价").input, 1.0);
    // 模拟 reload：向同一 ArcSwap 实例 store 新价
    rows.store(Arc::new(vec![("m-hot".to_string(), 9.9, 8.8, 0.0)]));
    let after = pt.lookup("m-hot", "default").expect("reload 后仍有价");
    assert_eq!(after.input, 9.9, "改价必须免重启生效（持句柄读取）");
    assert_eq!(after.output, 8.8);
}

/// ≥400 + 失败摘要的观测载荷翻译成 log_type=5 错误行；成功/免费行仍 consume。
#[test]
fn error_jobs_translate_to_log_type_5() {
    let base = || api::billing::RecordJob {
        user_uuid: Uuid::new_v4(),
        username: "u".into(),
        token_uuid: Some(Uuid::new_v4()),
        token_name: "t".into(),
        model_name: "m".into(),
        channel_key: None,
        channel_name: String::new(),
        prompt_tokens: 0,
        completion_tokens: 0,
        cost: 0,
        use_time_ms: 0,
        is_stream: true,
        token_key: Uuid::new_v4().to_string(),
        status_code: 200,
        error: None,
    };
    let ok = build_consume_event(&base());
    assert_eq!(ok.log_type, observe::logs::LOG_TYPE_CONSUME);

    let failed = api::billing::RecordJob {
        status_code: 502,
        error: Some("upstream reset".into()),
        ..base()
    };
    assert!(failed.is_error());
    let err = build_consume_event(&failed);
    assert_eq!(
        err.log_type,
        observe::logs::LOG_TYPE_ERROR,
        "≥400+失败摘要 → 错误行"
    );
    assert!(
        err.content.contains("502"),
        "状态码进 content 列, got {:?}",
        err.content
    );
    assert!(
        err.content.contains("upstream reset"),
        "错误摘要进 content 列, got {:?}",
        err.content
    );
    assert_eq!(err.quota, 0, "错误行零成本");
    assert!(err.is_stream, "流式意图透传");
}
