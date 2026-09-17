use super::card::AdminCard;
use dioxus::prelude::*;

fn fmt_price(v: f64) -> String {
    format!("¥{v:.4}")
}

/// Renders a read-only four-tab prototype card for a model alias.
///
/// The tabs are overview, pricing, groups, and system. The overview identifies
/// the alias and its position, pricing presents caller-supplied CNY values per
/// 1k tokens, groups presents only information derivable from the scalar props,
/// and system currently has no data. Group availability still depends on the
/// group's model whitelist and cannot be determined by this card alone. All
/// tabs intentionally omit a bottom action row. The edit callback is only a
/// future Popover integration point; this card does not edit or persist data.
#[component]
pub fn AliasCard(
    /// Alias used as the card title and passed to `on_edit`.
    alias: String,
    /// Optional display name shown as the card subtitle when non-empty.
    display: String,
    /// Page-supplied input price in CNY per 1k tokens, not a complete persisted
    /// backend pricing record.
    input_per_1k: f64,
    /// Page-supplied output price in CNY per 1k tokens, not a complete
    /// persisted backend pricing record.
    output_per_1k: f64,
    /// Scalar multiplier displayed on the pricing and groups tabs.
    multiplier: f64,
    /// Zero-based card position, displayed as a one-based sequence number.
    index: usize,
    /// Callback invoked with `alias` by the edit affordance; callers may later
    /// connect it to an edit Popover.
    on_edit: EventHandler<String>,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["概览", "定价", "分组", "系统"];

    rsx! {
        AdminCard {
            title: "{alias}",
            subtitle: if display.is_empty() { None } else { Some(display.clone()) },
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            show_edit: true,
            on_edit: move |_| on_edit.call(alias.clone()),
            testid: Some("alias-card-new".to_string()),

            {
                match tab() {
                    0 => rsx! {
                        div { class: "space-y-2.5",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "序号" }
                                span { class: "font-medium text-zinc-200", "#{index + 1}" }
                            }
                        }
                    },
                    1 => rsx! {
                        div { class: "space-y-2",
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
                    },
                    2 => rsx! {
                        div { class: "space-y-2",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "倍率" }
                                span { class: "font-medium text-zinc-200", "×{multiplier}" }
                            }
                            span { class: "text-[11px] text-zinc-500", "分组可用性需结合分组白名单判断" }
                        }
                    },
                    3 => rsx! {
                        span { class: "text-[11px] text-zinc-500", "暂无系统数据" }
                    },
                    _ => rsx! {},
                }
            }
        }
    }
}
