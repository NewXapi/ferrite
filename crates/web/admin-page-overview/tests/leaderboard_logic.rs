//! 排行榜 tab 增强(W3)纯函数回归。
//!
//! 覆盖四块:名次变动(进榜/退榜/持平/上窗无数据)、厂商前缀映射(命中/大小写/
//! 未知→Other)、上一等长窗起点计算(给定 timeframe 的当前窗长 → 上窗起点)、
//! 以及直接复用 api.rs pub 展示函数(fmt_usd/growth_of/share_text)的 import
//! 可用性与一次格式化输出。

use admin_page_overview::api::{Growth, UsageTopRow, fmt_usd, growth_of, share_text};
use admin_page_overview::insights::{
    RankDelta, previous_window_start_from, rank_moves, top_droppers, top_movers, vendor_of,
    vendor_shares,
};

fn top_row(name: &str, tokens: i64, quota: i64, calls: i64, previous_tokens: i64) -> UsageTopRow {
    UsageTopRow {
        name: name.to_string(),
        tokens,
        quota,
        calls,
        previous_tokens,
    }
}

// ---- 名次变动 ----

/// 进榜/上升/下降/持平四种基本形态:新模型记 New;同模型按名次差记 ±;
/// 名次没变记 Moved(0)。名次口径 = tokens 降序(与 /api/log/top 排序一致)。
#[test]
fn rank_moves_covers_new_up_down_and_flat() {
    let cur = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let prev = vec!["b".to_string(), "a".to_string()];
    let moves = rank_moves(&cur, &prev);
    assert_eq!(moves.len(), 3, "只产出当前窗在榜的行");
    assert_eq!(moves[0].name, "a");
    assert_eq!(moves[0].cur_rank, 1);
    assert_eq!(moves[0].delta, RankDelta::Moved(1), "a 从第 2 升到第 1");
    assert_eq!(moves[1].name, "b");
    assert_eq!(moves[1].delta, RankDelta::Moved(-1), "b 从第 1 降到第 2");
    assert_eq!(moves[2].name, "c");
    assert_eq!(moves[2].delta, RankDelta::New, "c 上一窗不在榜 → 进榜");
}

/// 两窗名次完全一致 → 全部持平 Moved(0)(不进任何一张卡)。
#[test]
fn rank_moves_flat_when_order_unchanged() {
    let names = vec!["x".to_string(), "y".to_string(), "z".to_string()];
    let moves = rank_moves(&names, &names);
    assert!(moves.iter().all(|m| m.delta == RankDelta::Moved(0)));
}

/// 上一窗无数据(空榜)→ 当前窗全员进榜;仅上一窗在榜(退榜跌出)不产出行。
#[test]
fn rank_moves_empty_prev_and_dropped_out_model() {
    let cur = vec!["a".to_string()];
    assert_eq!(
        rank_moves(&cur, &[])[0].delta,
        RankDelta::New,
        "上窗无数据 → 进榜"
    );
    // b 曾在上一窗、本窗跌出榜:行格式需要 #当前名次,故不产出行(诚实省略)
    let prev = vec!["a".to_string(), "b".to_string()];
    let moves = rank_moves(&cur, &prev);
    assert_eq!(moves.len(), 1, "退榜模型无当前名次,不产出行");
    assert_eq!(moves[0].name, "a");
    assert_eq!(moves[0].delta, RankDelta::Moved(0), "a 仍第 1,持平");
}

/// Top6 选取:上升卡只收 Moved(d>0) 与 New(降序、New 殿后);下跌卡只收
/// Moved(d<0)(跌最多在前);持平稳两侧都不进;n 截断生效。
#[test]
fn top_movers_and_droppers_pick_extremes_and_truncate() {
    let cur = vec!["a", "b", "c", "d", "e"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    let prev = vec!["e", "d", "c", "b", "a"]
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    // 完全反转:a +4, b +2, c 0, d -2, e -4
    let moves = rank_moves(&cur, &prev);
    let ups = top_movers(&moves, 6);
    assert_eq!(
        ups.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"],
        "升幅降序,持平不进"
    );
    let downs = top_droppers(&moves, 6);
    assert_eq!(
        downs.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
        vec!["e", "d"],
        "跌最多在前,持平不进"
    );
    // n 截断:只取前 1
    assert_eq!(top_movers(&moves, 1).len(), 1);
    // New 排在上升末尾,不伪造最大涨幅
    let cur2 = vec!["new1".to_string(), "a".to_string()];
    let prev2 = vec!["a".to_string()];
    let moves2 = rank_moves(&cur2, &prev2); // a +0? a 仍第 2 → Moved(0);new1 → New
    let ups2 = top_movers(&moves2, 6);
    assert_eq!(ups2.len(), 1, "持平不进,只剩 New");
    assert_eq!(ups2[0].name, "new1");
}

// ---- 厂商前缀映射 ----

/// 命中/大小写不敏感/未知→Other:映射表逐一钉住,防止前缀分支被误改。
#[test]
fn vendor_of_maps_prefixes_case_insensitively() {
    assert_eq!(vendor_of("gpt-4o"), "OpenAI");
    assert_eq!(vendor_of("GPT-4o-mini"), "OpenAI", "大小写不敏感");
    assert_eq!(vendor_of("o1-preview"), "OpenAI");
    assert_eq!(vendor_of("openai-community/x"), "OpenAI");
    assert_eq!(vendor_of("Claude-3.5-Sonnet"), "Anthropic");
    assert_eq!(vendor_of("gemini-1.5-pro"), "Google");
    assert_eq!(vendor_of("DeepSeek-V3"), "DeepSeek", "大小写不敏感");
    assert_eq!(vendor_of("kimi-k2"), "Moonshot");
    assert_eq!(vendor_of("moonshot-v1-8k"), "Moonshot");
    assert_eq!(vendor_of("qwen-max"), "Qwen");
    assert_eq!(vendor_of("grok-2"), "xAI");
    assert_eq!(vendor_of("glm-4-plus"), "Zhipu");
    assert_eq!(vendor_of("zhipu-x"), "Zhipu");
    assert_eq!(vendor_of("Llama-3.1-70B"), "Meta");
    assert_eq!(vendor_of("mistral-large"), "Mistral");
    assert_eq!(vendor_of("totally-unknown"), "Other", "未知前缀 → Other");
    assert_eq!(vendor_of(""), "Other", "空名兜底 Other");
}

/// 聚合:同厂商 tokens 合并、按合计降序(同额按名稳定)、零 tokens 行不产生条目。
#[test]
fn vendor_shares_aggregates_merges_and_sorts_desc() {
    let rows = vec![
        top_row("gpt-4o", 900, 0, 0, 0),
        top_row("GPT-4o-mini", 100, 0, 0, 0),
        top_row("claude-3", 500, 0, 0, 0),
        top_row("unknown-x", 10, 0, 0, 0),
        top_row("zero-calls-only", 0, 5, 3, 0), // 0 tokens → 不产生 Other 条目
    ];
    let shares = vendor_shares(&rows);
    assert_eq!(shares.len(), 3, "零 tokens 行不产生产商条目");
    assert_eq!(shares[0].vendor, "OpenAI");
    assert_eq!(shares[0].tokens, 1000, "同厂商两行合并(900+100)");
    assert_eq!(shares[1].vendor, "Anthropic");
    assert_eq!(shares[1].tokens, 500);
    assert_eq!(shares[2].vendor, "Other");
    assert_eq!(shares[2].tokens, 10);
}

/// 全零输入 → 空份额(调用方不渲染卡,不伪造 0.0%)。
#[test]
fn vendor_shares_empty_on_zero_rows() {
    assert!(vendor_shares(&[]).is_empty());
    assert!(vendor_shares(&[top_row("a", 0, 1, 1, 0)]).is_empty());
}

// ---- 上一等长窗起点 ----

/// 给定 timeframe 的当前窗长 → 上窗起点 = now - 2×窗长(四种档位逐一钉住,
/// 与 api::window_start 的 1/7/30/365 天口径对齐)。
#[test]
fn previous_window_start_is_two_window_lengths_before_now() {
    use chrono::{Duration, TimeZone, Utc};
    let now = Utc.with_ymd_and_hms(2026, 9, 16, 12, 0, 0).unwrap();
    for (tf, days) in [("今天", 1i64), ("本周", 7), ("本月", 30), ("今年", 365)] {
        let prev = previous_window_start_from(now, tf);
        let parsed = chrono::DateTime::parse_from_rfc3339(&prev).expect("上窗起点应为合法 RFC3339");
        assert_eq!(
            now - parsed.with_timezone(&Utc),
            Duration::days(days * 2),
            "{tf} 档上窗起点 = now - 2×{days}d"
        );
    }
    // 未知档位落今年兜底(同 api::window_start 的 `_` 分支)
    let prev = previous_window_start_from(now, "不存在的档");
    let parsed = chrono::DateTime::parse_from_rfc3339(&prev).unwrap();
    assert_eq!(now - parsed.with_timezone(&Utc), Duration::days(730));
}

// ---- 复用 api.rs pub 展示函数的回归 ----

/// 复用路径回归:import 可用 + 一次格式化输出(与 tests/api_shapes.rs 的
/// 详细断言互补,这里只钉住排行榜引用的同一套函数仍 pub 且行为不变)。
#[test]
fn reused_api_display_functions_still_work() {
    assert_eq!(
        fmt_usd(500_000),
        "$1.00",
        "500000 内部单位 = $1(¥→$ 口径切换依赖它)"
    );
    assert_eq!(
        growth_of(0, 5),
        Some(Growth::New),
        "上窗无该实体 → ↑new(榜单行内同语义)"
    );
    assert_eq!(
        share_text(1, 3),
        Some("33.3%".to_string()),
        "份额 1 位小数(榜单行内同语义)"
    );
}
