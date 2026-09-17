use super::card::AdminCard;
use dioxus::prelude::*;

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

fn redemption_status_str(status: i16) -> &'static str {
    match status {
        1 => "未使用",
        2 => "已核销",
        3 => "已停用",
        _ => "未知状态",
    }
}

/// Renders a read-only four-tab prototype card for a redemption code.
///
/// The tabs follow the redemption record's fields: 基本信息 (code preview,
/// CNY quota, status), 核销 (redeemer and redemption time), 生成
/// (generation time), and 系统 (shortened record key). `status` follows the
/// billing API exactly: `1` is unused, `2` is redeemed, and `3` is disabled;
/// other values render as "未知状态". `quota_cny` is an already converted CNY
/// amount and is shown with two decimal places. This card intentionally has
/// no edit affordance: future operations must be connected through a
/// Popover or status workflow.
#[component]
pub fn RedemptionCard(
    /// The redemption record key, shortened safely on the system tab.
    redemption_key: String,
    /// The code preview shown beneath the card title.
    code_preview: String,
    /// The redemption value in CNY, already converted by the caller.
    quota_cny: f64,
    /// Billing redemption status: 1=unused, 2=redeemed, 3=disabled.
    status: i16,
    /// User who redeemed the code, when it has been redeemed.
    redeemed_by: Option<String>,
    /// Redemption timestamp, when available.
    redeemed_at: Option<String>,
    /// Timestamp when the code was generated.
    created_at: String,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "核销", "生成", "系统"];

    let status_str = redemption_status_str(status);
    // 核销人/核销时间缺省占位提前算好，避免在 rsx 内联闭包里写字符串字面量。
    let redeemed_by_str = redeemed_by.clone().unwrap_or_else(|| "未核销".to_string());
    let redeemed_at_str = redeemed_at.clone().unwrap_or_else(|| "—".to_string());

    rsx! {
        AdminCard {
            title: "兑换码",
            subtitle: code_preview.clone(),
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            testid: Some("redemption-card-new".to_string()),

            {
                match tab() {
                    0 => rsx! {
                        div { class: "space-y-2.5",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "码预览" }
                                span { class: "font-medium text-zinc-200 truncate", "{code_preview}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "额度" }
                                span { class: "font-medium text-zinc-200", "¥{quota_cny:.2}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "状态" }
                                span { class: "font-medium text-zinc-200", "{status_str}" }
                            }
                        }
                    },
                    1 => rsx! {
                        div { class: "space-y-2 text-xs",
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "核销人" }
                                span { class: "text-zinc-200", "{redeemed_by_str}" }
                            }
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "核销时间" }
                                span { class: "text-zinc-200", "{redeemed_at_str}" }
                            }
                        }
                    },
                    2 => rsx! {
                        div { class: "space-y-2 text-xs",
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "生成时间" }
                                span { class: "text-zinc-200", "{created_at}" }
                            }
                        }
                    },
                    3 => rsx! {
                        div { class: "space-y-2 text-xs",
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "Key" }
                                span { class: "font-mono text-zinc-200", "{short_key(&redemption_key)}" }
                            }
                        }
                    },
                    _ => rsx! {},
                }
            }
        }
    }
}
