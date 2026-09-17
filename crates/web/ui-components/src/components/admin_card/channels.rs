use super::card::AdminCard;
use contract::api::admin::ChannelDto;
use dioxus::prelude::*;

/// Masks a key by Unicode scalar value without splitting UTF-8 characters.
fn mask_key(key: &str) -> String {
    const EDGE_CHARS: usize = 4;

    let char_count = key.chars().count();
    if char_count <= EDGE_CHARS * 2 {
        return key.to_string();
    }

    let head: String = key.chars().take(EDGE_CHARS).collect();
    let tail: String = key.chars().skip(char_count - EDGE_CHARS).collect();
    format!("{head}…{tail}")
}

/// Renders a four-tab prototype card for a [`ChannelDto`].
///
/// The overview tab shows status (`1` = enabled, other values = disabled),
/// address, test model, and remark; the key and model tabs show masked
/// credentials and dispatch models; the system tab keeps priority, weight,
/// groups, creation time, and update time. Empty `test_model`,
/// `remark`, and `updated_at` values explicitly render as "未配置", "未填写",
/// and "未记录". `on_edit` preserves the existing edit entry point, while all
/// four tabs intentionally omit a bottom action row.
#[component]
pub fn ChannelCard(
    /// The channel DTO displayed by this card.
    channel: ChannelDto,
    /// Callback invoked by the existing edit affordance.
    on_edit: EventHandler<()>,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["概览", "密钥", "模型", "系统"];

    let channel_status_str = if channel.status == 1 {
        "启用"
    } else {
        "停用"
    }
    .to_string();
    let test_model = channel
        .test_model
        .as_deref()
        .filter(|model| !model.trim().is_empty())
        .unwrap_or("未配置")
        .to_string();
    let remark = if channel.remark.trim().is_empty() {
        "未填写".to_string()
    } else {
        channel.remark.clone()
    };
    let updated_at = if channel.updated_at.trim().is_empty() {
        "未记录".to_string()
    } else {
        channel.updated_at.clone()
    };

    let dispatch_models: Vec<String> = channel
        .models
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| {
                    m.as_str().map(|s| s.to_string()).or_else(|| {
                        m.get("alias")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let masked_keys: Vec<String> = channel
        .keys
        .as_ref()
        .map(|v| v.iter().map(|k| mask_key(k)).collect())
        .unwrap_or_default();

    rsx! {
        AdminCard {
            title: "{channel.name}",
            subtitle: channel.channel_type.clone(),
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            show_edit: true,
            on_edit: move |_| on_edit.call(()),
            testid: Some("channel-card-new".to_string()),

            {
                match tab() {
                    0 => rsx! {
                        div { class: "space-y-2.5",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "状态" }
                                span { class: "font-medium text-zinc-200", "{channel_status_str}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "地址" }
                                span { class: "font-medium text-zinc-200 truncate", "{channel.base_url}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "测试模型" }
                                span { class: "font-medium text-zinc-200 truncate", "{test_model}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "备注" }
                                span { class: "font-medium text-zinc-200 truncate", "{remark}" }
                            }
                        }
                    },
                    1 => rsx! {
                        div { class: "space-y-2",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "密钥数" }
                                span { class: "font-medium text-zinc-200", "{channel.key_count}" }
                            }
                            for mk in &masked_keys {
                                div { class: "flex justify-between gap-2 text-xs",
                                    span { class: "text-zinc-400", "密钥" }
                                    span { class: "font-mono text-zinc-200", "{mk}" }
                                }
                            }
                        }
                    },
                    2 => rsx! {
                        div { class: "space-y-1.5",
                            if dispatch_models.is_empty() {
                                span { class: "text-[11px] text-zinc-500", "无调度模型" }
                            } else {
                                for m in &dispatch_models {
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
                                span { class: "text-zinc-400", "优先级" }
                                span { class: "text-zinc-200", "{channel.priority}" }
                            }
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "权重" }
                                span { class: "text-zinc-200", "{channel.weight}" }
                            }
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "分组" }
                                span { class: "text-zinc-200", "{channel.groups.join(\", \")}" }
                            }
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "创建" }
                                span { class: "text-zinc-200", "{channel.created_at}" }
                            }
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "更新" }
                                span { class: "text-zinc-200", "{updated_at}" }
                            }
                        }
                    },
                    _ => rsx! {},
                }
            }
        }
    }
}
