//! 名次变动(Top Movers / Droppers):纯判定 + 升降速双卡 UI。
//!
//! - 是什么:真实用量榜下方的「上升最快 / 下跌最快」双卡,把当前窗榜单与上一
//!   等长窗榜单做名次对比。
//! - 负责什么:名次口径(tokens 降序)、变动判定、取前 6,以及双卡渲染。
//! - 交互逻辑:纯展示,无事件、无本地状态;数据状态 `MoversState` 由页面拉取
//!   后传入(拉取中与失败分别走骨架与诚实报错)。
//! - 样式:两卡 `grid-cols-1 lg:grid-cols-2`;行 = `#当前名次` + 模型名 +
//!   tokens 实值 + ±delta(↑/↓ 文本符号,`tabular-nums` 对齐) + 比例长条
//!   (上升 emerald、下跌 rose,条宽 = 行 tokens / 本卡最大 tokens)。
//! - 数据流通:入参为两窗 by=model top20 聚合行;对外无回写。
//! - 纯函数全部 `pub`,供 `tests/leaderboard_logic.rs` 回归。

use dioxus::prelude::*;

use super::shared::{
    DROPPERS_BAR_COLOR, DROPPERS_SUBTITLE, DROPPERS_TITLE, MOVERS_BAR_COLOR, MOVERS_EMPTY,
    MOVERS_ERR_PREFIX, MOVERS_LIMIT, MOVERS_SUBTITLE, MOVERS_TITLE, slug,
};
use crate::api::UsageTopRow;
use crate::shared::fmt_raw;

/// 单个模型当前窗名次相对上一窗的变动。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RankDelta {
    /// 上一窗在榜:变动 = 上一窗名次 − 当前窗名次(正 = 上升,负 = 下降,0 = 持平)。
    Moved(i64),
    /// 上一窗不在榜(进榜,无可比较的旧名次)。
    New,
}

/// 一行名次变动:`name` 当前窗排第 `cur_rank` 名(1 起),变动 `delta`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankMove {
    pub name: String,
    pub cur_rank: usize,
    pub delta: RankDelta,
}

/// 榜单模型名按 tokens 降序(名次口径恒为 tokens,与总览页增长环比同口径)。
/// 后端 /api/log/top 本就 tokens 降序返回;此处防御性重排,避免后端排序变化
/// 时名次悄悄失真。
pub fn names_by_tokens(rows: &[UsageTopRow]) -> Vec<String> {
    let mut sorted: Vec<&UsageTopRow> = rows.iter().collect();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.tokens));
    sorted.into_iter().map(|r| r.name.clone()).collect()
}

/// 计算当前窗榜单相对上一窗榜单的名次变动。
///
/// 输入两列模型名均按名次排序(索引 = 名次 − 1)。只产出「当前窗在榜」的行:
/// 两窗都在 → [`RankDelta::Moved`];仅当前窗在 → [`RankDelta::New`];
/// 仅上一窗在(跌出当前榜)没有当前名次,不产出行——行格式需要 #当前名次,
/// 不伪造占位名次。
pub fn rank_moves(cur: &[String], prev: &[String]) -> Vec<RankMove> {
    cur.iter()
        .enumerate()
        .map(|(i, name)| {
            let delta = match prev.iter().position(|p| p == name) {
                Some(p) => RankDelta::Moved(p as i64 - i as i64),
                None => RankDelta::New,
            };
            RankMove {
                name: name.clone(),
                cur_rank: i + 1,
                delta,
            }
        })
        .collect()
}

/// 上升最快前 `n`:名次上升(`Moved(d>0)`)按 d 降序在前,进榜(`New`)随后
/// (进榜无升幅可排,排在末尾避免伪造「最大涨幅」);持平与下降不进此卡。
pub fn top_movers(moves: &[RankMove], n: usize) -> Vec<RankMove> {
    let mut ups: Vec<RankMove> = moves
        .iter()
        .filter(|m| match m.delta {
            RankDelta::Moved(d) => d > 0,
            RankDelta::New => true,
        })
        .cloned()
        .collect();
    ups.sort_by_key(|m| match m.delta {
        RankDelta::Moved(d) => -d,
        RankDelta::New => i64::MAX,
    });
    ups.truncate(n);
    ups
}

/// 下跌最快前 `n`:名次下降(`Moved(d<0)`)按 d 升序(跌最多在前);
/// 持平/上升/进榜不进此卡。
pub fn top_droppers(moves: &[RankMove], n: usize) -> Vec<RankMove> {
    let mut downs: Vec<RankMove> = moves
        .iter()
        .filter(|m| matches!(m.delta, RankDelta::Moved(d) if d < 0))
        .cloned()
        .collect();
    downs.sort_by_key(|m| match m.delta {
        RankDelta::Moved(d) => d,
        RankDelta::New => 0,
    });
    downs.truncate(n);
    downs
}

/// 双卡数据状态(与主榜单拉取独立:任一窗失败只降级本区,不拖垮三张榜单)。
#[derive(Clone, PartialEq)]
pub enum MoversState {
    /// 拉取中(骨架占位)。
    Loading,
    /// 当前窗与上一等长窗 top20 聚合就绪。
    Ready {
        /// 当前窗 by=model top20(名次口径 tokens)。
        cur: Vec<UsageTopRow>,
        /// 上一等长窗 by=model top20。
        prev: Vec<UsageTopRow>,
    },
    /// 任一窗聚合失败(诚实报错,不假装「无变动」)。
    Failed(String),
}

/// 单卡行列表(上升/下跌共用一套渲染;空态诚实标注「无显著变动」)。
///
/// 行结构对齐 RankBoard 条目解剖:`#当前名次` + 模型名 + 右侧 tokens 实值与
/// ±名次变动 + 相对比例长条(条宽 = 行 tokens / 本卡 movers 最大 tokens,
/// 全零时不渲染长条)。`bar_color` 上升卡 emerald、下跌卡 rose,与 delta 色呼应。
#[component]
fn MoveList(
    title: &'static str,
    subtitle: &'static str,
    testid: &'static str,
    row_prefix: &'static str,
    moves: Vec<RankMove>,
    rows: Vec<UsageTopRow>,
    bar_color: &'static str,
) -> Element {
    // 条宽口径:本卡各 mover 的当前窗 tokens(真实聚合值),相对最大者换算百分比
    let tokens_of = |name: &str| {
        rows.iter()
            .find(|r| r.name == name)
            .map(|r| r.tokens)
            .unwrap_or(0)
    };
    let max_tokens = moves.iter().map(|m| tokens_of(&m.name)).max().unwrap_or(0);

    rsx! {
        div { class: "space-y-3 rounded-xl border border-border bg-card p-5 transition-[border-color] duration-150 hover:border-secondary-hover",
            "data-testid": "{testid}",
            div {
                h3 { class: "{ui::TYPE_CARD_TITLE}", "{title}" }
                p { class: "{ui::TYPE_LABEL}", "{subtitle}" }
            }
            if moves.is_empty() {
                p { class: "py-6 text-center {ui::TYPE_DESC}", "{MOVERS_EMPTY}" }
            } else {
                div { class: "space-y-3",
                    for m in moves {
                        {
                            let (delta_text, delta_class) = match m.delta {
                                // 持平(Moved(0))不会进本卡;防御按上升色渲染
                                RankDelta::Moved(d) if d >= 0 => (format!("↑{d}"), "text-success-foreground"),
                                RankDelta::Moved(d) => (format!("↓{}", -d), "text-rose-400"),
                                RankDelta::New => ("↑new".to_string(), "text-success-foreground"),
                            };
                            let tokens = tokens_of(&m.name);
                            let bar_pct = if max_tokens > 0 {
                                (tokens as f64 / max_tokens as f64 * 100.0).max(2.0)
                            } else {
                                0.0
                            };
                            rsx! {
                                div {
                                    key: "{m.name}",
                                    class: "flex items-center gap-2.5",
                                    "data-testid": "{row_prefix}-{slug(&m.name)}",
                                    span { class: "w-7 shrink-0 text-right font-mono text-[10px] text-muted-foreground",
                                        "#{m.cur_rank}"
                                    }
                                    div { class: "min-w-0 flex-1",
                                        div { class: "flex items-center justify-between gap-3",
                                            span { class: "truncate {ui::TYPE_DESC}", "{m.name}" }
                                            div { class: "flex shrink-0 items-center gap-2",
                                                span { class: "font-mono text-xs font-semibold tabular-nums text-foreground",
                                                    "{fmt_raw(tokens)}"
                                                }
                                                span { class: "shrink-0 font-mono text-[11px] font-medium tabular-nums {delta_class}",
                                                    "{delta_text}"
                                                }
                                            }
                                        }
                                        if max_tokens > 0 {
                                            div { class: "mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-secondary",
                                                div {
                                                    class: "h-full rounded-full transition-all duration-300",
                                                    style: "width: {bar_pct:.1}%; background: {bar_color}",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 上升/下跌最快双卡(`lg:grid-cols-2`,移动端单列)。行 = 模型名 + #当前名次
/// + ±delta(↑/↓ 文本符号,配合 tabular-nums 对齐);各取变动前 `MOVERS_LIMIT`。
#[component]
pub fn MoversCards(state: MoversState) -> Element {
    match state {
        MoversState::Loading => rsx! {
            div { class: "grid grid-cols-1 gap-4 lg:grid-cols-2",
                for t in ["leaderboard-movers-loading", "leaderboard-droppers-loading"] {
                    div {
                        key: "{t}",
                        class: "h-40 animate-pulse rounded-xl border border-border bg-card/60 transition-[border-color] duration-150 hover:border-secondary-hover",
                        "data-testid": "{t}",
                    }
                }
            }
        },
        MoversState::Failed(e) => rsx! {
            div { class: "rounded-xl border border-border bg-card px-4 py-3 {ui::TYPE_DESC} transition-[border-color] duration-150 hover:border-secondary-hover",
                "data-testid": "leaderboard-movers-error",
                "{MOVERS_ERR_PREFIX}{e}"
            }
        },
        MoversState::Ready { cur, prev } => {
            let moves = rank_moves(&names_by_tokens(&cur), &names_by_tokens(&prev));
            let ups = top_movers(&moves, MOVERS_LIMIT);
            let downs = top_droppers(&moves, MOVERS_LIMIT);
            rsx! {
                div { class: "grid grid-cols-1 gap-6 lg:grid-cols-2",
                    MoveList {
                        title: MOVERS_TITLE,
                        subtitle: MOVERS_SUBTITLE,
                        testid: "leaderboard-movers",
                        row_prefix: "leaderboard-mover",
                        moves: ups,
                        rows: cur.clone(),
                        bar_color: MOVERS_BAR_COLOR,
                    }
                    MoveList {
                        title: DROPPERS_TITLE,
                        subtitle: DROPPERS_SUBTITLE,
                        testid: "leaderboard-droppers",
                        row_prefix: "leaderboard-dropper",
                        moves: downs,
                        rows: cur,
                        bar_color: DROPPERS_BAR_COLOR,
                    }
                }
            }
        }
    }
}
