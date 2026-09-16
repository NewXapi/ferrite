//! 真实用量区洞察卡:上升/下跌最快(名次变动) + 厂商份额(模型名前缀推断)。
//!
//! 纯函数与 UI 同文件:纯函数全部 `pub`,供 `tests/leaderboard_logic.rs` 回归;
//! UI 只消费真实聚合行,不造数据。展示尽量复用 [`crate::api`] 的既有 pub 项
//! (`share_text` / `UsageTopRow`;`fmt_usd` / `window_start` / `top_usage_api`
//! 由父模块 mod.rs 直接复用),本文件只补三类缺口:
//! - 上一等长窗:起点前端推([`previous_window_start_from`]) + 拉取需显式
//!   `end`([`top_usage_between`],api.rs 的 top_usage_api 把 end 固定缺省 now,
//!   表达不了 `[prev_start, cur_start)` 区间,而 api.rs 冻结不可改);
//! - 名次变动的纯整形;
//! - 厂商前缀推断与配色。

use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::api::{UsageTopRow, share_text};
use client::{ApiClient, ApiResult};

use super::fmt_raw;

// ---- 上一等长窗 ----

/// 窗口长度(天):与 [`crate::api::window_start`] 同口径(今天=1 / 本周=7 /
/// 本月=30 / 未知兜底=365)。两边任一漂移都会让「上一窗」错位,故在此显式注释。
fn window_days(timeframe: &str) -> i64 {
    match timeframe {
        "今天" => 1,
        "本周" => 7,
        "本月" => 30,
        _ => 365,
    }
}

/// 上一等长窗起点(纯函数,给定 `now` 便于测试):当前窗起点是 `now - 窗长`,
/// 上一窗再往前一个窗长,即 `now - 2×窗长`。格式与 [`crate::api::window_start`]
/// 一致(RFC3339 UTC,秒级)。
pub fn previous_window_start_from(now: DateTime<Utc>, timeframe: &str) -> String {
    use chrono::Duration;
    (now - Duration::days(window_days(timeframe) * 2))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// 当前时刻的上一等长窗起点(薄包装,组件直呼)。
pub fn previous_window_start(timeframe: &str) -> String {
    previous_window_start_from(Utc::now(), timeframe)
}

/// `/api/log/top` 响应的 `{"items":[...]}` 剥壳(与 api.rs 的 `Items<T>` 同形;
/// 那边是私有结构,冻结区不可复用,此处本地声明)。
#[derive(Default, serde::Deserialize)]
struct TopItems<T> {
    #[serde(default)]
    items: Vec<T>,
}

/// 真实调用: GET /api/log/top?by=&start=&end=&limit= — 指定 `[start, end)` 窗口
/// 的消耗聚合(与 [`crate::api::top_usage_api`] 同端点同 DTO,差异仅在显式传
/// `end`:名次变动的「上一窗」是 `[prev_start, cur_start)`,end 不能落 now)。
/// 错误情况:401/403(未登录或非管理员)、网络失败,均走 [`ApiResult`]。
pub async fn top_usage_between(
    by: &str,
    start: &str,
    end: &str,
    limit: u32,
) -> ApiResult<Vec<UsageTopRow>> {
    let client = ApiClient::shared().clone();
    let r: TopItems<UsageTopRow> = client
        .get(&format!(
            "/api/log/top?by={by}&start={start}&end={end}&limit={limit}"
        ))
        .await?;
    Ok(r.items)
}

// ---- 名次变动(Top Movers / Droppers) ----

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

/// 模型/厂商名 → data-testid slug:ASCII 字母数字保留(转小写),其余折叠为
/// 单个 `-`(如 `deepseek-ai/DeepSeek-V3` → `deepseek-ai-deepseek-v3`),
/// 供 UI 验证按 name 语义定位。
pub fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

// ---- 上升/下跌最快双卡 UI ----

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
#[component]
fn MoveList(
    title: &'static str,
    subtitle: &'static str,
    testid: &'static str,
    row_prefix: &'static str,
    moves: Vec<RankMove>,
) -> Element {
    rsx! {
        div { class: "space-y-3 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            "data-testid": "{testid}",
            div {
                h3 { class: "text-sm font-semibold text-zinc-100", "{title}" }
                p { class: "text-[11px] text-zinc-500", "{subtitle}" }
            }
            if moves.is_empty() {
                p { class: "py-6 text-center text-xs text-zinc-500", "当前窗口无显著变动" }
            } else {
                div { class: "space-y-2",
                    for m in moves {
                        {
                            let (delta_text, delta_class) = match m.delta {
                                // 持平(Moved(0))不会进本卡;防御按上升色渲染
                                RankDelta::Moved(d) if d >= 0 => (format!("↑{d}"), "text-emerald-400"),
                                RankDelta::Moved(d) => (format!("↓{}", -d), "text-rose-400"),
                                RankDelta::New => ("↑new".to_string(), "text-emerald-400"),
                            };
                            rsx! {
                                div {
                                    key: "{m.name}",
                                    class: "flex items-center justify-between gap-3 text-xs",
                                    "data-testid": "{row_prefix}-{slug(&m.name)}",
                                    div { class: "flex min-w-0 items-center gap-2",
                                        span { class: "shrink-0 font-mono text-[10px] text-zinc-500", "#{m.cur_rank}" }
                                        span { class: "truncate text-zinc-200", "{m.name}" }
                                    }
                                    span { class: "shrink-0 font-mono text-[11px] font-medium tabular-nums {delta_class}",
                                        "{delta_text}"
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
/// + ±delta(↑/↓ 文本符号,配合 tabular-nums 对齐);各取变动前 6。
#[component]
pub fn MoversCards(state: MoversState) -> Element {
    match state {
        MoversState::Loading => rsx! {
            div { class: "grid grid-cols-1 gap-4 lg:grid-cols-2",
                for t in ["leaderboard-movers-loading", "leaderboard-droppers-loading"] {
                    div {
                        key: "{t}",
                        class: "h-40 animate-pulse rounded-xl border border-zinc-800 bg-zinc-900/60",
                        "data-testid": "{t}",
                    }
                }
            }
        },
        MoversState::Failed(e) => rsx! {
            div { class: "rounded-xl border border-zinc-800 bg-zinc-900 px-4 py-3 text-xs text-zinc-400",
                "data-testid": "leaderboard-movers-error",
                "名次变动加载失败:{e}"
            }
        },
        MoversState::Ready { cur, prev } => {
            let moves = rank_moves(&names_by_tokens(&cur), &names_by_tokens(&prev));
            let ups = top_movers(&moves, 6);
            let downs = top_droppers(&moves, 6);
            rsx! {
                div { class: "grid grid-cols-1 gap-4 lg:grid-cols-2",
                    MoveList {
                        title: "上升最快",
                        subtitle: "tokens 名次较上一等长窗上升(取前 6)",
                        testid: "leaderboard-movers",
                        row_prefix: "leaderboard-mover",
                        moves: ups,
                    }
                    MoveList {
                        title: "下跌最快",
                        subtitle: "tokens 名次较上一等长窗下跌(取前 6)",
                        testid: "leaderboard-droppers",
                        row_prefix: "leaderboard-dropper",
                        moves: downs,
                    }
                }
            }
        }
    }
}

// ---- 厂商份额(模型名前缀推断) ----

/// 模型名 → 厂商(按模型名前缀推断,大小写不敏感;未知前缀归 Other)。
/// 前缀清单为 PR 任务书约定的映射表;新增厂商在此追加分支即可。
pub fn vendor_of(model_name: &str) -> &'static str {
    let m = model_name.to_ascii_lowercase();
    if m.starts_with("gpt") || m.starts_with("o1") || m.starts_with("openai") {
        "OpenAI"
    } else if m.starts_with("claude") {
        "Anthropic"
    } else if m.starts_with("gemini") {
        "Google"
    } else if m.starts_with("deepseek") {
        "DeepSeek"
    } else if m.starts_with("kimi") || m.starts_with("moonshot") {
        "Moonshot"
    } else if m.starts_with("qwen") {
        "Qwen"
    } else if m.starts_with("grok") {
        "xAI"
    } else if m.starts_with("glm") || m.starts_with("zhipu") {
        "Zhipu"
    } else if m.starts_with("llama") {
        "Meta"
    } else if m.starts_with("mistral") {
        "Mistral"
    } else {
        "Other"
    }
}

/// 厂商品牌色(暗色 zinc 底上可辨;内联 hex 不走 Tailwind 扫描;无官方暗色
/// 规范的厂商取近似品牌色,相邻撞色靠图例色点区分)。
static VENDOR_COLORS: [(&str, &str); 10] = [
    ("OpenAI", "#10a37f"),
    ("Anthropic", "#d97757"),
    ("Google", "#4285f4"),
    ("DeepSeek", "#4d6bfe"),
    ("Moonshot", "#f472b6"),
    ("Qwen", "#a855f7"),
    ("xAI", "#d4d4d8"),
    ("Zhipu", "#38bdf8"),
    ("Meta", "#0891b2"),
    ("Mistral", "#ff8200"),
];

/// 未收录厂商的循环 fallback 色(当前映射已全覆盖,防未来扩表遗漏)。
static VENDOR_FALLBACK_COLORS: [&str; 5] = ["#3b82f6", "#facc15", "#34d399", "#f472b6", "#a78bfa"];

/// 厂商段色:品牌表命中取品牌色,未收录按 `fallback_index` 循环 fallback。
/// `fallback_index` 传行在榜中的下标,保证条形与图例两处同参同色(闭环图例)。
pub fn vendor_color(vendor: &str, fallback_index: usize) -> &'static str {
    VENDOR_COLORS
        .iter()
        .find(|(n, _)| *n == vendor)
        .map(|(_, c)| *c)
        .unwrap_or(&VENDOR_FALLBACK_COLORS[fallback_index % VENDOR_FALLBACK_COLORS.len()])
}

/// 厂商份额聚合行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorShare {
    pub vendor: &'static str,
    /// 该厂商窗口内 tokens 合计(份额与条宽的口径)。
    pub tokens: i64,
}

/// 把 by=model 聚合行按厂商合并 tokens,合计降序(同额按名稳定,保证
/// 条形与图例顺序确定)。tokens 为 0 的行不产生产商条目(0 宽段无意义)。
pub fn vendor_shares(rows: &[UsageTopRow]) -> Vec<VendorShare> {
    use std::collections::HashMap;
    let mut acc: HashMap<&'static str, i64> = HashMap::new();
    for r in rows {
        if r.tokens <= 0 {
            continue;
        }
        *acc.entry(vendor_of(&r.name)).or_default() += r.tokens;
    }
    let mut out: Vec<VendorShare> = acc
        .into_iter()
        .map(|(vendor, tokens)| VendorShare { vendor, tokens })
        .collect();
    out.sort_by(|a, b| b.tokens.cmp(&a.tokens).then_with(|| a.vendor.cmp(b.vendor)));
    out
}

/// 厂商份额卡:单根 100% 横向堆叠条(手绘 SVG rect 按宽度百分比排)+ 双列
/// 厂商行(色点与条形段同色,闭环图例)。数据复用 by=model 当前窗聚合;
/// 卡底小字注明「厂商按模型名前缀推断」。加载中渲染骨架;空窗/全零不渲染
/// (下方用量榜已有诚实空态,不重复占位)。
#[component]
pub fn VendorShareCard(rows: Vec<UsageTopRow>, loading: bool) -> Element {
    if loading {
        return rsx! {
            div {
                class: "h-[148px] animate-pulse rounded-xl border border-zinc-800 bg-zinc-900/60",
                "data-testid": "leaderboard-vendor-share-loading",
            }
        };
    }
    let shares = vendor_shares(&rows);
    let total: i64 = shares.iter().map(|s| s.tokens).sum();
    if total <= 0 {
        return rsx! {};
    }
    // 段几何先整形再进 rsx:堆叠条 x 轴偏移依赖前段累加,避免在渲染闭包里改状态
    let mut acc = 0.0f64;
    let segments: Vec<(&'static str, f64, f64, &'static str, String)> = shares
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let pct = s.tokens as f64 / total as f64 * 100.0;
            let x = acc;
            acc += pct;
            (
                s.vendor,
                x,
                pct,
                vendor_color(s.vendor, i),
                format!("{}: {pct:.1}%", s.vendor),
            )
        })
        .collect();
    rsx! {
        div { class: "space-y-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            "data-testid": "leaderboard-vendor-share",
            div {
                h3 { class: "text-sm font-semibold text-zinc-100", "厂商份额" }
                p { class: "text-[11px] text-zinc-500", "窗口内各厂商 Token 消耗占比" }
            }
            div { class: "h-2.5 w-full overflow-hidden rounded-full bg-zinc-800/80",
                svg {
                    class: "h-full w-full",
                    view_box: "0 0 100 10",
                    preserve_aspect_ratio: "none",
                    for (vendor, x, pct, color, tip) in segments {
                        rect {
                            key: "{vendor}",
                            x: "{x:.2}",
                            y: "0",
                            width: "{pct:.2}",
                            height: "10",
                            fill: "{color}",
                            title { "{tip}" }
                        }
                    }
                }
            }
            div { class: "grid grid-cols-1 gap-x-6 gap-y-2 pt-1 sm:grid-cols-2",
                for (i, s) in shares.iter().enumerate() {
                    {
                        let color = vendor_color(s.vendor, i);
                        let share = share_text(s.tokens, total);
                        rsx! {
                            div {
                                key: "{s.vendor}",
                                class: "flex items-center justify-between gap-3 text-xs",
                                "data-testid": "leaderboard-vendor-{slug(s.vendor)}",
                                div { class: "flex min-w-0 items-center gap-2",
                                    span { class: "h-2.5 w-2.5 shrink-0 rounded-[2px]", style: "background: {color}" }
                                    span { class: "truncate text-zinc-200", "{s.vendor}" }
                                }
                                div { class: "flex shrink-0 items-center gap-2 font-mono tabular-nums",
                                    span { class: "text-zinc-500", "{fmt_raw(s.tokens)}" }
                                    if let Some(p) = share {
                                        span { class: "text-zinc-400", "{p}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "text-[10px] text-zinc-600", "厂商按模型名前缀推断" }
        }
    }
}
