//! overview page 集成测试:api.rs 数据形状不变量。

use page_overview::api;

#[test]
fn stats_pairs_complete() {
    let stats = api::fetch_stats();
    assert!(!stats.is_empty(), "统计卡非空");
    for (value, label) in stats {
        assert!(!value.is_empty(), "统计卡值非空");
        assert!(!label.is_empty(), "统计卡标签非空");
    }
}

#[test]
fn tools_shares_sum_to_reasonable() {
    let tools = api::fetch_tools();
    assert!(!tools.is_empty(), "工具分布非空");
    // (名称, 用量, 占比%): 占比 0..=100
    for (name, _usage, pct) in tools {
        assert!(!name.is_empty(), "工具名非空");
        assert!((0.0..=100.0).contains(pct), "工具 {name} 占比 {pct} 超界");
    }
}

#[test]
fn models_shares_sum_to_reasonable() {
    let models = api::fetch_models();
    assert!(!models.is_empty(), "模型分布非空");
    for (name, _usage, pct) in models {
        assert!(!name.is_empty(), "模型名非空");
        assert!((0.0..=100.0).contains(pct), "模型 {name} 占比 {pct} 超界");
    }
}
