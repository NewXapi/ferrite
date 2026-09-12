//! Dashboard section 集成测试。
//! 总览页真实数据管线: pivot_trend 的桶对齐/空桶补零/模型截断不变量。
//! 另断言 models / leaderboard 新接的真实端点响应形状(字段名用后端 camelCase 原名):
//! - GET /api/models → {"items":[ModelView], "total"} (admin-catalog models.rs)
//! - GET /api/log/top → {"items":[UsageTopRow]}       (admin-observe logs.rs)

use admin_page_overview::api::{ModelCardView, UsageTopRow, UsageTrendRow, pivot_trend};

fn row(bucket: &str, model: &str, tokens: i64) -> UsageTrendRow {
    UsageTrendRow {
        bucket: bucket.to_string(),
        model_name: model.to_string(),
        tokens,
        quota: tokens * 2,
        calls: 1,
    }
}

#[test]
fn pivot_fills_continuous_hourly_buckets() {
    // 两行同一小时 → 一个桶;24 小时槽位连续生成
    let rows = vec![
        row("2026-09-09T10:30:00Z", "gpt-x", 1000),
        row("2026-09-09T10:45:00Z", "gpt-x", 500),
    ];
    let (buckets, order) = pivot_trend(rows, "今天");
    assert_eq!(buckets.len(), 24, "今天 = 24 个小时桶");
    assert_eq!(order, vec!["gpt-x"]);
    // 10:30 落在第 10 个小时桶(从窗口起点 00:xx 起算) —— 只断言非零桶只有一个
    let nonzero: Vec<&admin_page_overview::api::TrendBucketFE> =
        buckets.iter().filter(|b| b.total > 0.0).collect();
    assert_eq!(nonzero.len(), 1);
    assert_eq!(nonzero[0].total, 1500.0);
}

#[test]
fn pivot_orders_models_by_total_desc() {
    let rows = vec![
        row("2026-09-09T10:00:00Z", "a-model", 100),
        row("2026-09-09T10:00:00Z", "b-model", 900),
        row("2026-09-09T11:00:00Z", "a-model", 50),
    ];
    let (buckets, order) = pivot_trend(rows, "今天");
    assert_eq!(order.first().unwrap(), "b-model", "总量大的模型排前");
    // 每个桶的 per_model 与 model_order 对齐
    for b in &buckets {
        assert_eq!(b.per_model.len(), order.len());
    }
    assert_eq!(buckets.iter().map(|b| b.total).sum::<f64>(), 1050.0);
}

#[test]
fn pivot_empty_rows_gives_all_zero_buckets() {
    let (buckets, order) = pivot_trend(vec![], "本周");
    assert_eq!(buckets.len(), 7, "本周 = 7 个天桶");
    assert!(order.is_empty());
    assert!(buckets.iter().all(|b| b.total == 0.0));
}

#[test]
fn pivot_truncates_to_top_ten_models() {
    let rows: Vec<UsageTrendRow> = (0..12)
        .map(|i| row("2026-09-09T10:00:00Z", &format!("m{i}"), 100 - i))
        .collect();
    let (buckets, order) = pivot_trend(rows, "今天");
    assert_eq!(order.len(), 10, "窗口内超过 10 个模型时截到前 10");
    assert_eq!(order[0], "m0");
    assert_eq!(buckets.len(), 24);
}

#[test]
fn nice_axis_caps_above_max_with_clean_ticks() {
    use admin_page_overview::api::nice_axis_max;

    // 209.5M → step 60M → 轴顶 240M, 柱顶 (209.5/240 ≈ 87%) 不触顶
    let axis = nice_axis_max(209.5e6);
    assert_eq!(axis, 240.0e6);
    assert!(axis > 209.5e6);
    // 刻度 = axis × i/4 全为整数个千万 (无零头)
    for i in 1..=4 {
        let tick = axis * i as f64 / 4.0;
        assert_eq!(tick % 10_000_000.0, 0.0, "tick {tick} 应只保留最高位");
    }
    // 128M → 40M 步长 → 160M
    assert_eq!(nice_axis_max(128e6), 160e6);
    // 恰好落在整位: max=160M → raw 40M → digit 4 → 160M, 柱顶 100%? 不, 见下
    // max=200M → raw=50M → digit=5 → 200M 轴顶, 柱顶=200/200=100% —— 恰好满格。
    // 接受恰满格 (max==axis 仅当 max 本身就是整齐值), 此时刻度仍整齐。
    assert_eq!(nice_axis_max(200e6), 200e6);
    // 空窗兜底: 不出现 0 除法
    assert_eq!(nice_axis_max(0.0), 4.0);
}

/// GET /api/models 返回的 ModelView 是 camelCase;页面本地视图只映射其中一部分字段。
/// 字段名对照后端 admin-catalog/src/models.rs 的 `ModelView`(serde rename_all = camelCase)。
#[test]
fn model_card_view_maps_backend_camel_case_fields() {
    // 后端单条 ModelView 的真实 JSON(多余字段应被忽略,缺省字段走 Default)
    let raw = r#"{
        "key": "3f2b7c1e-0000-0000-0000-000000000001",
        "name": "gpt-x",
        "owner": "ops",
        "modelType": "openai",
        "baseUrl": "https://placeholder.invalid/v1",
        "maskedKey": "sk-1****abcd",
        "capabilities": {},
        "speed": 88,
        "rating": null,
        "usageCount": 4096,
        "maxTokens": 131072,
        "isVision": true,
        "isTool": false,
        "status": 1,
        "createdAt": "2026-09-01T00:00:00Z",
        "updatedAt": "2026-09-02T00:00:00Z"
    }"#;
    let v: ModelCardView = serde_json::from_str(raw).expect("完整 ModelView 应可反序列化");
    assert_eq!(v.name, "gpt-x");
    assert_eq!(v.owner, "ops");
    assert_eq!(v.model_type, "openai");
    assert_eq!(v.status, 1);
    assert_eq!(v.usage_count, 4096);
    assert_eq!(v.max_tokens, 131072);
    assert!(v.is_vision);
    assert!(!v.is_tool);
}

/// 页面视图对后端字段缺省要有韧性:分页/最小化响应里缺 bool/数值时落到 Default,
/// 不至于整页 500(与 admin-page-admin hydrate 的 ModelDto 同样的宽容策略)。
#[test]
fn model_card_view_defaults_on_missing_fields() {
    let v: ModelCardView = serde_json::from_str(r#"{"name": "m1"}"#).expect("仅 name 也应可解析");
    assert_eq!(v.name, "m1");
    assert_eq!(v.status, 0, "缺 status 落 Default → 视为停用(诚实降级)");
    assert_eq!(v.usage_count, 0);
    assert!(!v.is_vision && !v.is_tool);
}

/// GET /api/log/top 行:后端 UsageTopRow 是 {name, tokens, quota, calls}(camelCase 无差异)。
/// 排行榜三个榜单(Token/调用数/费用)都从这一行的不同字段取数。
#[test]
fn usage_top_row_maps_backend_fields() {
    let raw = r#"{"name":"gpt-x","tokens":1500,"quota":750000,"calls":3}"#;
    let v: UsageTopRow = serde_json::from_str(raw).expect("UsageTopRow 应可反序列化");
    assert_eq!(v.name, "gpt-x");
    assert_eq!(v.tokens, 1500);
    assert_eq!(v.quota, 750_000);
    assert_eq!(v.calls, 3);
    assert_eq!(
        UsageTopRow::default(),
        UsageTopRow {
            name: String::new(),
            tokens: 0,
            quota: 0,
            calls: 0
        },
        "Default 用于空窗兜底"
    );
}

/// /api/log/top 的包装形状 {"items": [...]}:与 api.rs 的 Items<T> 剥壳逻辑一致。
#[test]
fn usage_top_items_wrapper_peels_items_field() {
    let raw = r#"{"items":[
        {"name":"a","tokens":100,"quota":10,"calls":1},
        {"name":"b","tokens":900,"quota":90,"calls":9}
    ]}"#;
    #[derive(serde::Deserialize)]
    struct Items<T> {
        #[serde(default)]
        items: Vec<T>,
    }
    let v: Items<UsageTopRow> = serde_json::from_str(raw).expect("items 包装应可剥离");
    assert_eq!(v.items.len(), 2);
    // 后端按 tokens DESC 返回;排行榜本地重排 calls/quota 榜依赖行内字段完整性
    assert_eq!(v.items[0].tokens, 100);
    assert_eq!(v.items[1].tokens, 900);
}
