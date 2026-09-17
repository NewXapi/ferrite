use super::card::AdminCard;
use dioxus::prelude::*;

fn fmt_price(v: f64) -> String {
    format!("¥{v:.4}")
}

/// Shortens a key by Unicode scalar value without splitting UTF-8 characters.
fn short_key(key: &str) -> String {
    const EDGE_CHARS: usize = 4;

    let char_count = key.chars().count();
    if char_count <= EDGE_CHARS * 2 {
        return key.to_string();
    }

    let head: String = key.chars().take(EDGE_CHARS).collect();
    let tail: String = key.chars().skip(char_count - EDGE_CHARS).collect();
    format!("{head}…{tail}")
}

/// Renders a read-only four-tab prototype card for a model alias.
///
/// The tabs follow the alias edit modal's fields: 基本信息 (alias / display),
/// 定价 (input and output CNY prices per 1k tokens plus the multiplier),
/// 分组 (caller-computed usable groups as name + ratio chips, first four
/// with an overflow counter, or "无分组引用" when empty), and 系统
/// (shortened alias key). All tabs intentionally omit a bottom action row;
/// this card does not edit or persist data.
#[component]
pub fn AliasCard(
    /// Alias used as the card title.
    alias: String,
    /// Optional display name shown as the card subtitle when non-empty.
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
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "定价", "分组", "系统"];

    let shown_groups = usable_groups.iter().take(4).collect::<Vec<_>>();
    let overflow_groups = usable_groups.len().saturating_sub(4);
    let short_k = short_key(&alias_key);
    // 卡片标题与内容区各持一份克隆,避免 `alias` 被 move 后仍被借用
    let title_alias = alias.clone();

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
            p { class: "text-[11px] font-medium text-zinc-400", "按量 / 按次 双模式" }
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
                        span { class: "inline-flex items-center gap-1 rounded-full border border-zinc-700 bg-zinc-800/80 px-2 py-0.5 text-[11px] text-zinc-300",
                            "{gname}"
                            span { class: "text-[10px] font-mono opacity-70", "×{gratio:.1}" }
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

    rsx! {
        AdminCard {
            title: "{title_alias}",
            subtitle: if display.is_empty() { None } else { Some(display.clone()) },
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            testid: Some("alias-card-new".to_string()),
            panel_0: panel_basic,
            panel_1: panel_pricing,
            panel_2: panel_groups,
            panel_3: panel_system,
        }
    }
}
