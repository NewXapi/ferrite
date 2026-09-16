//! Dashboard section 集成测试。
//! 总览页真实数据管线: pivot_trend 的桶对齐/空桶补零/模型截断不变量。
//! 另断言 models / leaderboard 新接的真实端点响应形状(字段名用后端 camelCase 原名):
//! - GET /api/models → {"items":[ModelView], "total"} (admin-catalog models.rs)
//! - GET /api/log/top → {"items":[UsageTopRow]}       (admin-observe logs.rs)
//!
//! 以及统计卡新增的纯展示函数: fmt_usd / sparkline 重切与归一化 / 增长率三态 / 份额。
//! W4 追加: /api/log/errors 信封解析、趋势悬浮卡排序+Total+折叠、时间窗副标题文案、
//! lastSeen 本地时间格式化。

use admin_page_overview::api::{
    Growth, ModelCardView, TIP_MAX_ROWS, TIP_MORE_COLOR, UsageTopRow, UsageTrendRow,
    as_of_local_time, fmt_usd, growth_of, hourly_sums, last_seen_local_time, pivot_trend,
    reslice_sum, share_text, sparkline_points, sparkline_svg_paths, trend_column_tip,
    window_caption,
};
use contract::api::usage::UsageErrorStatPage;

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

/// GET /api/log/top 行:后端 UsageTopRow 是 {name, tokens, quota, calls, previousTokens}(camelCase)。
/// 排行榜三个榜单(Token/调用数/费用)都从这一行的不同字段取数;
/// previousTokens 是 W1 新增的上一等长窗口环比字段。
#[test]
fn usage_top_row_maps_backend_fields() {
    let raw = r#"{"name":"gpt-x","tokens":1500,"quota":750000,"calls":3,"previousTokens":1200}"#;
    let v: UsageTopRow = serde_json::from_str(raw).expect("UsageTopRow 应可反序列化");
    assert_eq!(v.name, "gpt-x");
    assert_eq!(v.tokens, 1500);
    assert_eq!(v.quota, 750_000);
    assert_eq!(v.calls, 3);
    assert_eq!(v.previous_tokens, 1200, "新 wire 的 previousTokens 应命中");
    assert_eq!(
        UsageTopRow::default(),
        UsageTopRow {
            name: String::new(),
            tokens: 0,
            quota: 0,
            calls: 0,
            previous_tokens: 0
        },
        "Default 用于空窗兜底"
    );
}

/// 旧后端 wire 不带 previousTokens(字段晚于页面出现):缺字段必须按 0 兜底,
/// 整行解析不失败——增长率据此诚实降级为「上窗无数据」。
#[test]
fn usage_top_row_previous_tokens_defaults_on_missing_field() {
    let raw = r#"{"name":"legacy","tokens":42,"quota":21,"calls":2}"#;
    let v: UsageTopRow = serde_json::from_str(raw).expect("旧 wire 缺 previousTokens 也应可解析");
    assert_eq!(v.previous_tokens, 0, "缺字段 serde default → 0");
    assert_eq!(v.tokens, 42);
}

/// /api/log/top 的包装形状 {"items": [...]}:与 api.rs 的 Items<T> 剥壳逻辑一致。
#[test]
fn usage_top_items_wrapper_peels_items_field() {
    let raw = r#"{"items":[
        {"name":"a","tokens":100,"quota":10,"calls":1},
        {"name":"b","tokens":900,"quota":90,"calls":9,"previousTokens":800}
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
    // 混合 wire:带 previousTokens 的行命中、不带的行兜底 0
    assert_eq!(v.items[0].previous_tokens, 0);
    assert_eq!(v.items[1].previous_tokens, 800);
}

// ---- 统计卡纯展示函数（W2：fmt_usd / sparkline / 增长率 / 份额 / asOf）----

/// fmt_usd 三档:万元内两位小数原值、万美元起切紧凑 K/M/B(500_000 = $1)。
#[test]
fn fmt_usd_formats_zero_decimal_and_tiers() {
    assert_eq!(fmt_usd(0), "$0.00", "零额度显示 $0.00");
    assert_eq!(fmt_usd(500_000), "$1.00", "500_000 内部单位 = $1");
    assert_eq!(fmt_usd(20_900_000), "$41.80", "小数保留两位");
    assert_eq!(fmt_usd(1_048_575_000), "$2097.15", "万元内不切档");
    assert_eq!(fmt_usd(20_900_000_000), "$41.8K", "万美元起切 K");
    assert_eq!(fmt_usd(20_900_000_000_000), "$41.8M", "切 M");
    assert_eq!(fmt_usd(20_900_000_000_000_000), "$41.8B", "切 B");
}

/// 24 桶重切 12 桶 = 相邻两桶合并求和。
#[test]
fn reslice_sum_merges_24_buckets_into_12_pairwise() {
    let series: Vec<f64> = (1..=24).map(|i| i as f64).collect();
    let out = reslice_sum(&series, 12);
    assert_eq!(out.len(), 12, "24 → 12 桶");
    assert_eq!(out[0], 3.0, "桶 0 = 1+2");
    assert_eq!(out[11], 47.0, "桶 11 = 23+24");
    assert_eq!(out.iter().sum::<f64>(), 300.0, "重切不丢总量");
}

/// 空输入返回空(调用方渲染等高占位);单点保持单桶。
#[test]
fn reslice_sum_handles_empty_and_single_point() {
    assert!(reslice_sum(&[], 12).is_empty(), "空输入 → 空,不做占位造假");
    assert_eq!(reslice_sum(&[5.0], 12), vec![5.0], "单点不足一组长存原值");
}

/// min-max 归一化:最低点贴下缘(y = 高-2.5)、最高点贴上缘(y = 2.5),x 等距铺满。
#[test]
fn sparkline_points_normalize_min_max_with_padding() {
    let pts = sparkline_points(&[0.0, 100.0], 160.0, 36.0);
    assert_eq!(pts.len(), 2);
    let (x0, y0) = pts[0];
    let (x1, y1) = pts[1];
    assert!(
        (x0 - 0.0).abs() < 1e-9 && (x1 - 160.0).abs() < 1e-9,
        "x 铺满 0..160"
    );
    assert!((y0 - 33.5).abs() < 1e-9, "最低点贴下缘留 2.5px 描边余量");
    assert!((y1 - 2.5).abs() < 1e-9, "最高点贴上缘留 2.5px 描边余量");
}

/// 全等序列(全零/恒值)压成等高水平线 y = 0.75 × 高——诚实呈现「无起伏」。
#[test]
fn sparkline_points_flattens_equal_series() {
    let pts = sparkline_points(&[0.0; 12], 160.0, 36.0);
    assert_eq!(pts.len(), 12);
    assert!(
        pts.iter().all(|(_, y)| (y - 27.0).abs() < 1e-9),
        "全零序列 → y = 0.75 × 36 的水平线"
    );
    assert!((pts[0].0 - 0.0).abs() < 1e-9 && (pts[11].0 - 160.0).abs() < 1e-9);
}

/// 空输入无点(渲染占位);单点退化为等高单点(不足两点,路径层同样降级占位)。
#[test]
fn sparkline_points_empty_and_single() {
    assert!(sparkline_points(&[], 160.0, 36.0).is_empty());
    let pts = sparkline_points(&[42.0], 160.0, 36.0);
    assert_eq!(pts.len(), 1);
    assert!((pts[0].1 - 27.0).abs() < 1e-9, "单点按全等规则落在等高线");
}

/// line path 覆盖全部点;area path = line path + 底边闭合(渐变面积)。
#[test]
fn sparkline_svg_paths_area_encloses_line() {
    let (line, area) = sparkline_svg_paths(&[0.0, 100.0], 160.0, 36.0).expect("两点应出路径");
    assert!(line.starts_with("M"), "line 以 M 起笔");
    assert!(area.starts_with(&line), "area 复用 line 的点序列");
    assert!(area.ends_with('Z'), "area 闭合");
    assert_eq!(area.matches("36.0").count(), 2, "底边两次触底(右下+左下)");
    assert!(
        sparkline_svg_paths(&[], 160.0, 36.0).is_none(),
        "空序列 → None,调用方渲染等高占位"
    );
    assert!(
        sparkline_svg_paths(&[7.0], 160.0, 36.0).is_none(),
        "单点画不出趋势 → None 诚实降级"
    );
}

/// 增长率三态:prev>0 比百分比(↑绿/↓红)、prev==0 且 cur>0 为 new、cur==0 不显示。
#[test]
fn growth_of_covers_three_states() {
    assert_eq!(growth_of(0, 0), None, "两窗皆零 → 不显示");
    assert_eq!(growth_of(10, 0), None, "本窗归零 → 不显示(诚实,不秀 ↓100%)");
    assert_eq!(growth_of(0, 5), Some(Growth::New), "上窗无该实体 → new");
    assert_eq!(growth_of(10, 15), Some(Growth::Up(50)), "升 50%");
    assert_eq!(growth_of(10, 5), Some(Growth::Down(50)), "降 50%");
    assert_eq!(growth_of(10, 10), Some(Growth::Up(0)), "持平记 ↑0%");
    assert_eq!(growth_of(3, 4), Some(Growth::Up(33)), "百分比四舍五入");
    assert_eq!(growth_of(4, 3), Some(Growth::Down(25)), "百分比四舍五入");
}

/// 箭头编进文本保证 tabular-nums 列对齐;↑/new 绿、↓ 红。
#[test]
fn growth_label_embeds_arrow_for_alignment() {
    assert_eq!(Growth::Up(50).label(), "↑50%");
    assert_eq!(Growth::Down(50).label(), "↓50%");
    assert_eq!(Growth::New.label(), "↑new");
    assert_eq!(Growth::Up(0).label(), "↑0%");
    assert_eq!(
        Growth::Up(1).text_class(),
        Growth::New.text_class(),
        "升与 new 同绿色档"
    );
    assert_ne!(
        Growth::Up(1).text_class(),
        Growth::Down(1).text_class(),
        "升/降双色档"
    );
}

/// 份额:1 位小数百分比;正值不足 0.1% 显示 <0.1%;合计为 0 不显示。
#[test]
fn share_text_covers_normal_zero_total_and_sub_tenth() {
    assert_eq!(share_text(25, 100), Some("25.0%".to_string()));
    assert_eq!(share_text(1, 3), Some("33.3%".to_string()), "1 位小数");
    assert_eq!(
        share_text(0, 100),
        Some("0.0%".to_string()),
        "零值行显 0.0%"
    );
    assert_eq!(share_text(1, 2000), Some("<0.1%".to_string()), "0.05% 特判");
    assert_eq!(
        share_text(1, 1000),
        Some("0.1%".to_string()),
        "恰好 0.1% 不特判"
    );
    assert_eq!(share_text(5, 0), None, "合计 0 → 不显示(不伪造份额)");
}

/// 24 小时桶聚合:同小时多模型合并、窗口外行丢弃、start 漂移行不越界。
#[test]
fn hourly_sums_buckets_rows_into_24_hourly_slots() {
    let start = "2026-09-09T00:00:00Z";
    let rows = vec![
        UsageTrendRow {
            bucket: "2026-09-09T00:30:00Z".into(),
            model_name: "gpt".into(),
            tokens: 100,
            quota: 500,
            calls: 1,
        },
        UsageTrendRow {
            bucket: "2026-09-09T00:45:00Z".into(),
            model_name: "claude".into(),
            tokens: 50,
            quota: 250,
            calls: 2,
        },
        UsageTrendRow {
            bucket: "2026-09-09T13:00:00Z".into(),
            model_name: "gpt".into(),
            tokens: 700,
            quota: 3500,
            calls: 3,
        },
        // 窗口外:窗口前 / 第 24 小时(越界) → 丢弃
        UsageTrendRow {
            bucket: "2026-09-08T23:59:00Z".into(),
            model_name: "gpt".into(),
            tokens: 999,
            quota: 999,
            calls: 9,
        },
        UsageTrendRow {
            bucket: "2026-09-10T00:00:00Z".into(),
            model_name: "gpt".into(),
            tokens: 999,
            quota: 999,
            calls: 9,
        },
        // 桶解析失败 → 丢弃
        UsageTrendRow {
            bucket: "not-a-time".into(),
            model_name: "gpt".into(),
            tokens: 999,
            quota: 999,
            calls: 9,
        },
    ];
    let (tokens, calls) = hourly_sums(&rows, start);
    assert_eq!(tokens.len(), 24, "固定 24 个小时桶");
    assert_eq!(calls.len(), 24);
    assert_eq!(tokens[0], 150.0, "同小时两行合并(100+50)");
    assert_eq!(calls[0], 3.0, "同小时 calls 合并(1+2)");
    assert_eq!(tokens[13], 700.0, "13 时桶命中");
    assert_eq!(calls[13], 3.0);
    assert_eq!(tokens.iter().sum::<f64>(), 850.0, "窗口外与坏桶行全部丢弃");
}

/// start 解析失败 → 空序列(调用方渲染占位,诚实降级)。
#[test]
fn hourly_sums_bad_start_yields_empty() {
    let (tokens, calls) = hourly_sums(&[], "not-a-time");
    assert!(tokens.is_empty() && calls.is_empty());
}

/// asOf 本地时间:合法 RFC3339 → HH:MM:SS(本地时区,断言形状不依赖机器时区);
/// 空串/垃圾串 → None(诚实降级不显示)。
#[test]
fn as_of_local_time_parses_or_hides() {
    let t = as_of_local_time("2026-09-16T17:00:00Z").expect("合法 RFC3339 应可解析");
    assert_eq!(t.len(), 8, "HH:MM:SS 共 8 字符");
    assert!(
        t.as_bytes()[2] == b':' && t.as_bytes()[5] == b':',
        "HH:MM:SS 冒号位"
    );
    assert!(t.chars().all(|c| c.is_ascii_digit() || c == ':'));
    assert_eq!(as_of_local_time(""), None, "空 asOf → 不显示");
    assert_eq!(as_of_local_time("yesterday"), None, "垃圾串 → 不显示");
}

// ---- W4: 错误呈现卡 + 趋势悬浮卡折叠 + 时间窗副标题 ----

/// trend_column_tip 测试色板:3 色,用于验证系列色按原始下标取模(与堆叠段同色)。
const TIP_PAL: [&str; 3] = ["#c0ffee", "#c1ffee", "#c2ffee"];

/// 悬浮卡整列分解:明细按值降序、系列色与堆叠段同色、Total = 全部原始值之和。
#[test]
fn trend_column_tip_sorts_desc_and_totals() {
    let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let tip = trend_column_tip(&[100.0, 900.0, 50.0], &names, &TIP_PAL);
    assert_eq!(tip.rows[0].0, "b", "值大的模型排前");
    assert_eq!(
        tip.rows[0].1, TIP_PAL[1],
        "系列色按原始下标取色,不随排序漂移"
    );
    assert_eq!(tip.rows[1].0, "a");
    assert_eq!(tip.rows[1].1, TIP_PAL[0]);
    assert_eq!(tip.rows[2].0, "c");
    assert_eq!(tip.rows[2].1, TIP_PAL[2]);
    assert_eq!(tip.total, 1050.0, "Total 含全部原始值");
}

/// 全零与空输入:无明细行(0.01 阈值过滤)、Total 为 0——悬浮卡只剩 Total 行,诚实呈现空列。
#[test]
fn trend_column_tip_all_zero_and_empty_give_total_only() {
    let names = vec!["a".to_string(), "b".to_string()];
    let tip = trend_column_tip(&[0.0, 0.0], &names, &TIP_PAL);
    assert!(tip.rows.is_empty(), "全零明细全部被 0.01 阈值过滤");
    assert_eq!(tip.total, 0.0);
    let tip = trend_column_tip(&[], &[], &TIP_PAL);
    assert!(tip.rows.is_empty(), "空输入无明细行");
    assert_eq!(tip.total, 0.0);
}

/// 超 10 行折叠:保留前 10 行明细,第 11 行换「+N more」(中性 zinc 色块、值 = 被折叠合计)。
#[test]
fn trend_column_tip_folds_beyond_ten_rows() {
    let names: Vec<String> = (0..13).map(|i| format!("m{i}")).collect();
    let values: Vec<f64> = (0..13).map(|i| 100.0 - i as f64).collect();
    let tip = trend_column_tip(&values, &names, &TIP_PAL);
    assert_eq!(tip.rows.len(), TIP_MAX_ROWS + 1, "10 行明细 + 1 行折叠");
    assert_eq!(tip.rows[0].0, "m0", "降序头部不动");
    assert_eq!(tip.rows[9].0, "m9");
    let more = &tip.rows[10];
    assert_eq!(more.0, "+3 more", "折叠行标注被折叠行数");
    assert_eq!(
        more.1, TIP_MORE_COLOR,
        "折叠行用中性 zinc 色块(不专属某模型色)"
    );
    assert_eq!(
        more.2,
        90.0 + 89.0 + 88.0,
        "折叠行 value = m10+m11+m12 合计"
    );
    assert_eq!(
        tip.total,
        values.iter().sum::<f64>(),
        "Total 仍含被折叠值,三段对得上账"
    );
}

/// 恰好 10 行是边界:不折叠、不出「+N more」行。
#[test]
fn trend_column_tip_exactly_ten_rows_needs_no_fold() {
    let names: Vec<String> = (0..10).map(|i| format!("m{i}")).collect();
    let values: Vec<f64> = (0..10).map(|i| 100.0 - i as f64).collect();
    let tip = trend_column_tip(&values, &names, &TIP_PAL);
    assert_eq!(tip.rows.len(), 10, "恰好 10 行不折叠");
    assert!(tip.rows.iter().all(|(name, ..)| !name.starts_with('+')));
}

/// GET /api/log/errors 信封:{"items":[{modelName,count,lastSeenAs}],"asOf"} 整体反序列化
/// 为 contract UsageErrorStatPage(字段名 camelCase;asOf 由后端 DateTime<Utc> 序列化,带小数秒)。
#[test]
fn usage_error_stat_page_maps_backend_envelope() {
    let raw = r#"{"items":[
        {"modelName":"gpt-x","count":12,"lastSeenAt":"2026-09-16T16:59:00Z"},
        {"modelName":"claude-y","count":3,"lastSeenAt":"2026-09-16T10:00:00Z"}
    ],"asOf":"2026-09-16T17:00:00.123456789Z"}"#;
    let p: UsageErrorStatPage = serde_json::from_str(raw).expect("errors 信封应可反序列化");
    assert_eq!(p.items.len(), 2);
    assert_eq!(p.items[0].model_name, "gpt-x");
    assert_eq!(p.items[0].count, 12);
    assert_eq!(p.items[0].last_seen_at, "2026-09-16T16:59:00Z");
    assert_eq!(p.items[1].count, 3);
    assert_eq!(
        p.as_of, "2026-09-16T17:00:00.123456789Z",
        "asOf 原样保留(带小数秒也能被 as_of_local_time 解析)"
    );
}

/// 旧 wire 缺 asOf:serde default 兜底为空串,整页解析不失败;
/// 调用方对空串走 as_of_local_time → None → 不显示(诚实降级,不伪造时间)。
#[test]
fn usage_error_stat_page_defaults_on_missing_as_of() {
    let raw = r#"{"items":[{"modelName":"m","count":1,"lastSeenAt":"2026-09-16T10:00:00Z"}]}"#;
    let p: UsageErrorStatPage = serde_json::from_str(raw).expect("缺 asOf 也应可解析");
    assert_eq!(p.as_of, "", "缺 asOf 落空串兜底");
    assert_eq!(p.items.len(), 1);
    assert_eq!(as_of_local_time(&p.as_of), None, "空串 asOf → 不显示");
}

/// 空 items:合法解析、行列表为空 → 卡内渲染「近 24 小时无错误记录」空态(合计 0)。
#[test]
fn usage_error_stat_page_empty_items_is_honest_empty() {
    let p: UsageErrorStatPage =
        serde_json::from_str(r#"{"items":[],"asOf":"2026-09-16T17:00:00Z"}"#)
            .expect("空 items 应可解析");
    assert!(p.items.is_empty());
    let total: i64 = p.items.iter().map(|r| r.count).sum();
    assert_eq!(total, 0, "空 items 合计 0");
}

/// 时间窗副标题:四个 timeframe 全覆盖,与 window_start/pivot_trend 的桶数语义一致;
/// 未知档落今年兜底(与 window_start 的 `_` 分支同口径)。
#[test]
fn window_caption_maps_all_four_timeframes() {
    assert_eq!(window_caption("今天"), "近 24 小时 · 逐小时");
    assert_eq!(window_caption("本周"), "近 7 天 · 逐天");
    assert_eq!(window_caption("本月"), "近 30 天 · 逐天");
    assert_eq!(window_caption("今年"), "近 12 个月 · 逐月");
    assert_eq!(
        window_caption("whatever"),
        "近 12 个月 · 逐月",
        "未知档落今年兜底"
    );
}

/// lastSeen 本地时间:合法 RFC3339(含小数秒/时区偏移变体)→ HH:MM(断言形状不依赖机器时区);
/// 空串/垃圾串 → None(行内以 — 占位)。
#[test]
fn last_seen_local_time_parses_or_hides() {
    let t = last_seen_local_time("2026-09-16T17:00:00Z").expect("合法 RFC3339 应可解析");
    assert_eq!(t.len(), 5, "HH:MM 共 5 字符");
    assert_eq!(&t[2..3], ":", "冒号在 HH:MM 第 3 位");
    assert!(t.chars().all(|c| c.is_ascii_digit() || c == ':'));
    assert!(
        last_seen_local_time("2026-09-16T17:00:00.123456789Z").is_some(),
        "后端 DateTime<Utc> 序列化带小数秒,也应可解析"
    );
    let off = last_seen_local_time("2026-09-16T00:30:00+08:00").expect("带偏移的 RFC3339 应可解析");
    assert_eq!(off.len(), 5);
    assert_eq!(last_seen_local_time(""), None, "空串 → 不显示");
    assert_eq!(last_seen_local_time("yesterday"), None, "垃圾串 → 不显示");
}
