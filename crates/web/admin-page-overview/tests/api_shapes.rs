//! Dashboard section 集成测试。
//! 总览页真实数据管线: pivot_trend 的桶对齐/空桶补零/模型截断不变量。
//! leaderboard/models 仍为展示型 mock（showcase），此处不测。

use admin_page_overview::api::{UsageTrendRow, pivot_trend};

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
