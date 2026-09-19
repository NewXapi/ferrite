use super::card::{AdminCard, short_key};
use super::price_mode::{PriceMode, PriceModeToggle};
use dioxus::prelude::*;

fn fmt_price(v: f64) -> String {
    format!("¥{v:.4}")
}

/// 分组倍率胶囊的配色：1.0 灰、低于 1.0 绿（打折）、高于 1.0 琥珀（加价）。
///
/// 这是旧卡的信息设计核心（一眼看出一组定价是便宜还是贵），迁移时逐字保留。
fn ratio_tone(ratio: f64) -> &'static str {
    if (ratio - 1.0).abs() < 0.001 {
        "border-zinc-700 bg-zinc-800/80 text-zinc-300"
    } else if ratio < 1.0 {
        "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
    } else {
        "border-amber-500/30 bg-amber-500/10 text-amber-300"
    }
}

/// Renders a four-tab card for a model alias.
///
/// The tabs follow the alias edit modal's fields: 基本信息 (alias / display /
/// 序号), 定价 (input and output CNY prices per 1k tokens, the multiplier, and
/// the per-card 按量/按次 toggle), 分组 (caller-computed usable groups as
/// name + ratio chips with semantic tone, first four with an overflow counter,
/// or "无分组引用" when empty), and 系统 (shortened alias key).
///
/// 卡片本身不发起编辑：数据以值传入，模式切换经 `on_mode_change` 抛回页面
/// 写回 `rows`。序号同时以 header badge 呈现（与旧卡 header 样式一致）。
#[component]
pub fn AliasCard(
    /// Alias used as the card title.
    alias: String,
    /// Optional display name; shown as subtitle only when non-empty and
    /// different from `alias`（与旧卡判定一致，避免标题副标题重复）.
    display: String,
    /// Page-supplied input price in CNY per 1k tokens, not a complete persisted
    /// backend pricing record.
    input_per_1k: f64,
    /// Page-supplied output price in CNY per 1k tokens, not a complete
    /// persisted backend pricing record.
    output_per_1k: f64,
    /// Per-call multiplier displayed on the pricing tab.
    multiplier: f64,
    /// Zero-based card position, displayed as a one-based sequence number.
    index: usize,
    /// Caller-computed list of groups usable by this alias: `(group name,
    /// group ratio)`. Empty when no group references the alias.
    usable_groups: Vec<(String, f64)>,
    /// Backend alias key (UUID); displayed truncated on the system tab.
    alias_key: String,
    /// Per-card pricing mode（按量 / 按次）.
    #[props(default = PriceMode::PerToken)]
    price_mode: PriceMode,
    /// 模式切换回调（抛回页面写回 rows）；未传时不渲染 toggle。
    #[props(default)]
    on_mode_change: Option<EventHandler<PriceMode>>,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "定价", "分组", "系统"];

    let shown_groups = usable_groups.iter().take(4).collect::<Vec<_>>();
    let overflow_groups = usable_groups.len().saturating_sub(4);
    let short_k = short_key(&alias_key);
    // 卡片标题与内容区各持一份克隆,避免 `alias` 被 move 后仍被借用
    let title_alias = alias.clone();

    // 副标题仅在展示名非空且与别名不同时出现（与旧卡一致）。
    let subtitle = if display.is_empty() || display == alias {
        None
    } else {
        Some(display.clone())
    };

    // 四个页签内容各自为独立 Element，传给 AdminCard 同格叠加渲染。
    let panel_basic = rsx! {
        div { class: "space-y-2.5",
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "别名" }
                span { class: "font-medium text-zinc-200", "{title_alias}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "展示名" }
                span { class: "font-medium text-zinc-200", if display.is_empty() { "未填写" } else { "{display}" } }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "序号" }
                span { class: "font-medium text-zinc-200", "#{index + 1}" }
            }
        }
    };
    let panel_pricing = rsx! {
        div { class: "space-y-2",
            div { class: "flex items-center justify-between gap-2",
                p { class: "text-[11px] font-medium text-zinc-400",
                    if price_mode == PriceMode::PerCall { "按次定价" } else { "按量定价" }
                }
                if let Some(cb) = on_mode_change {
                    PriceModeToggle { active: price_mode, on_change: move |m: PriceMode| cb.call(m) }
                }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "输入" }
                span { class: "font-medium text-zinc-200", "{fmt_price(input_per_1k)} / 1k tokens" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "输出" }
                span { class: "font-medium text-zinc-200", "{fmt_price(output_per_1k)} / 1k tokens" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "倍率" }
                span { class: "font-medium text-zinc-200", "×{multiplier}" }
            }
        }
    };
    let panel_groups = rsx! {
        div { class: "space-y-1.5",
            p { class: "text-[11px] text-zinc-400", "可用分组" }
            if shown_groups.is_empty() {
                span { class: "text-[11px] text-zinc-500", "无分组引用" }
            } else {
                div { class: "flex flex-wrap gap-1.5",
                    for (gname, gratio) in shown_groups {
                        {
                            let tone = ratio_tone(*gratio);
                            rsx! {
                                span { class: "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] {tone}",
                                    "{gname}"
                                    span { class: "text-[10px] font-mono opacity-70", "×{gratio:.1}" }
                                }
                            }
                        }
                    }
                    if overflow_groups > 0 {
                        span { class: "rounded-full border border-zinc-700 bg-zinc-800/60 px-2 py-0.5 text-[11px] text-zinc-400",
                            "+{overflow_groups}"
                        }
                    }
                }
            }
        }
    };
    let panel_system = rsx! {
        div { class: "space-y-2 text-xs",
            div { class: "flex justify-between gap-2",
                span { class: "text-zinc-400", "Key" }
                span { class: "font-mono text-zinc-200", "{short_k}" }
            }
        }
    };

    // 序号 badge：与旧卡 header 右侧样式逐字一致。
    let index_badge = rsx! {
        span {
            class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
            "#{index + 1}"
        }
    };

    rsx! {
        AdminCard {
            title: "{title_alias}",
            subtitle: subtitle,
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            testid: Some("alias-card-new".to_string()),
            header_action: index_badge,
            panel_0: panel_basic,
            panel_1: panel_pricing,
            panel_2: panel_groups,
            panel_3: panel_system,
        }
    }
}
