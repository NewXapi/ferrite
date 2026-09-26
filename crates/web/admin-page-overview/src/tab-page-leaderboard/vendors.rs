//! 厂商份额:模型名前缀推断 + 份额聚合 + 100% 堆叠条卡。
//!
//! - 是什么:真实用量榜上方的厂商份额卡,把 by=model 聚合行按厂商合并 tokens。
//! - 负责什么:前缀 → 厂商映射、品牌配色、份额聚合(纯函数,`pub` 供单测),
//!   以及单根堆叠条 + 双列厂商行的渲染。
//! - 交互逻辑:纯展示,无事件;段上挂 `<title>` 提供原生 tooltip。
//! - 样式:单根 100% 横向堆叠条(手绘 SVG rect 按宽度百分比排,`viewBox 0 0 100 10`
//!   + `preserveAspectRatio=none`);双列厂商行色点与条形段同色(闭环图例)。
//! - 数据流通:入参为 by=model 当前窗聚合行;加载中渲染骨架,空窗/全零不渲染
//!   (下方用量榜已有诚实空态,不重复占位)。

use dioxus::prelude::*;

use super::shared::{VENDOR_FOOTNOTE, VENDOR_SUBTITLE, VENDOR_TITLE, slug};
use crate::api::{UsageTopRow, share_text};
use crate::shared::fmt_raw;

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
        .unwrap_or(VENDOR_FALLBACK_COLORS[fallback_index % VENDOR_FALLBACK_COLORS.len()])
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
                class: "h-[148px] animate-pulse rounded-xl border border-border bg-card/60 transition-[border-color] duration-150 hover:border-secondary-hover",
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
        div { class: "space-y-4 rounded-xl border border-border bg-card p-5 transition-[border-color] duration-150 hover:border-secondary-hover",
            "data-testid": "leaderboard-vendor-share",
            div {
                h3 { class: "{ui::TYPE_CARD_TITLE}", "{VENDOR_TITLE}" }
                p { class: "{ui::TYPE_LABEL}", "{VENDOR_SUBTITLE}" }
            }
            div { class: "h-2.5 w-full overflow-hidden rounded-full bg-secondary/80",
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
            // 厂商行距 gap-y-3.5(8090 预览反馈⑤);卡内段落间距由容器 space-y-4 承担,
            // 与上方演示区 / 下方真实用量榜的外层间距由页面根 gap-6(md:gap-8)保证
            div { class: "grid grid-cols-1 gap-x-6 gap-y-3.5 pt-1 sm:grid-cols-2",
                for (i, s) in shares.iter().enumerate() {
                    {
                        let color = vendor_color(s.vendor, i);
                        let share = share_text(s.tokens, total);
                        rsx! {
                            div {
                                key: "{s.vendor}",
                                class: "flex items-center justify-between gap-3 {ui::TYPE_DESC}",
                                "data-testid": "leaderboard-vendor-{slug(s.vendor)}",
                                div { class: "flex min-w-0 items-center gap-2",
                                    span { class: "h-2.5 w-2.5 shrink-0 rounded-[2px]", style: "background: {color}" }
                                    span { class: "truncate text-foreground", "{s.vendor}" }
                                }
                                div { class: "flex shrink-0 items-center gap-2 font-mono tabular-nums",
                                    span { class: "{ui::C_MUTED}", "{fmt_raw(s.tokens)}" }
                                    if let Some(p) = share {
                                        span { class: "{ui::C_MUTED}", "{p}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "{ui::TYPE_LABEL}", "{VENDOR_FOOTNOTE}" }
        }
    }
}
