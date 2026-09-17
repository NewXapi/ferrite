use super::card::AdminCard;
use contract::api::admin::GroupDto;
use dioxus::prelude::*;

fn parse_whitelist(raw: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = raw.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(s) = raw.as_str() {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}

fn fmt_ratio(r: f64) -> String {
    format!("×{r:.2}")
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

/// Renders a four-tab prototype card for a [`GroupDto`].
///
/// The card presents the group's overview, ratio, model whitelist, and system
/// metadata. `is_default` labels the default group. `on_edit` preserves the
/// existing edit entry point and may later be connected to an edit Popover;
/// this card itself does not perform a write.
#[component]
pub fn GroupCard(
    /// The group DTO displayed by this card.
    group: GroupDto,
    /// Whether this group is the default group.
    is_default: bool,
    /// Callback invoked by the existing edit affordance.
    on_edit: EventHandler<()>,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["概览", "倍率", "白名单", "系统"];

    let group_status_str = if group.status == 1 {
        "启用"
    } else {
        "停用"
    }
    .to_string();
    let example_cost = (100.0 * group.ratio).round() as i64;
    let whitelist = parse_whitelist(&group.model_whitelist);
    let short_k = short_key(&group.key);
    let remark = if group.remark.trim().is_empty() {
        "未填写".to_string()
    } else {
        group.remark.clone()
    };

    rsx! {
        AdminCard {
            title: "{group.name}",
            subtitle: if is_default { Some("默认分组".to_string()) } else { None },
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            show_edit: true,
            on_edit: move |_| on_edit.call(()),
            testid: Some("group-card-new".to_string()),

            {
                match tab() {
                    0 => rsx! {
                        div { class: "space-y-2.5",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "状态" }
                                span { class: "font-medium text-zinc-200", "{group_status_str}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "备注" }
                                span { class: "font-medium text-zinc-200", "{remark}" }
                            }
                        }
                    },
                    1 => rsx! {
                        div { class: "space-y-2",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "倍率" }
                                span { class: "font-medium text-zinc-200", "{fmt_ratio(group.ratio)}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "示例价格" }
                                span { class: "font-medium text-zinc-200", "100 单位 = {example_cost}" }
                            }
                        }
                    },
                    2 => rsx! {
                        div { class: "space-y-1.5",
                            if whitelist.is_empty() {
                                span { class: "text-[11px] text-zinc-500", "无白名单限制（全模型可用）" }
                            } else {
                                for m in &whitelist {
                                    span {
                                        class: "inline-flex rounded border border-zinc-700 bg-zinc-800/80 px-2 py-0.5 text-[11px] text-zinc-300 mr-1.5 mb-1",
                                        "{m}"
                                    }
                                }
                            }
                        }
                    },
                    3 => rsx! {
                        div { class: "space-y-2 text-xs",
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "Key" }
                                span { class: "font-mono text-zinc-200", "{short_k}" }
                            }
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "创建" }
                                span { class: "text-zinc-200", "{group.created_at}" }
                            }
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "更新" }
                                span { class: "text-zinc-200", "{group.updated_at}" }
                            }
                        }
                    },
                    _ => rsx! {},
                }
            }
        }
    }
}
