use super::card::AdminCard;
use contract::api::admin::AdminUserDto;
use dioxus::prelude::*;

/// Formats an internal quota amount as a CNY value.
///
/// The shared contract defines `500_000` internal quota units as `¥1`.
fn fmt_quota_cny(quota: i64) -> String {
    const QUOTA_PER_CNY: f64 = 500_000.0;

    format!("¥{:.2}", quota as f64 / QUOTA_PER_CNY)
}

fn role_label(role: u16) -> &'static str {
    match role {
        100 => "超级管理员",
        10 => "管理员",
        _ => "普通用户",
    }
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

/// Renders a four-tab prototype card for an [`AdminUserDto`].
///
/// The card presents the user's overview, groups, CNY quota (where `500_000`
/// internal units equal `¥1`), and system metadata. `on_edit` receives the
/// user key when the existing edit affordance is selected; it is an entry point
/// only, so callers may attach an edit Popover without changing this card.
#[component]
pub fn UserCard(
    /// The administrative user DTO displayed by this card.
    user: AdminUserDto,
    /// Callback invoked with the user key by the existing edit affordance.
    on_edit: EventHandler<String>,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["概览", "分组", "额度", "系统"];

    let user_status_str = if user.status == 1 { "启用" } else { "停用" }.to_string();
    let role_str = role_label(user.role).to_string();
    let short_k = short_key(&user.key);

    let used_pct = if user.quota > 0 {
        ((user.used_quota as f64 / user.quota as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    rsx! {
        AdminCard {
            title: "{user.username}",
            subtitle: user.display_name.clone(),
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            show_edit: true,
            on_edit: move |_| on_edit.call(user.key.clone()),
            testid: Some("user-card-new".to_string()),

            {
                match tab() {
                    0 => rsx! {
                        div { class: "space-y-2.5",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "邮箱" }
                                span { class: "font-medium text-zinc-200 truncate", "{user.email}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "状态" }
                                span { class: "font-medium text-zinc-200", "{user_status_str}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "角色" }
                                span { class: "font-medium text-zinc-200", "{role_str}" }
                            }
                        }
                    },
                    1 => rsx! {
                        div { class: "flex flex-wrap gap-1.5",
                            for g in &user.groups {
                                span {
                                    class: "rounded-full border border-zinc-700 bg-zinc-800/80 px-2 py-0.5 text-[11px] text-zinc-300",
                                    "{g}"
                                }
                            }
                            if user.groups.is_empty() {
                                span { class: "text-[11px] text-zinc-500", "无分组" }
                            }
                        }
                    },
                    2 => rsx! {
                        div { class: "space-y-2",
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "已用" }
                                span { class: "font-medium text-zinc-200", "{fmt_quota_cny(user.used_quota)}" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "总额" }
                                span { class: "font-medium text-zinc-200", "{fmt_quota_cny(user.quota)}" }
                            }
                            div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                                div { class: "h-full rounded-full bg-emerald-500 transition-all", style: "width: {used_pct:.1}%" }
                            }
                            div { class: "flex justify-between gap-2 text-xs",
                                span { class: "text-zinc-400", "请求数" }
                                span { class: "font-medium text-zinc-200", "{user.request_count}" }
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
                                span { class: "text-zinc-200", "{user.created_at}" }
                            }
                        }
                    },
                    _ => rsx! {},
                }
            }
        }
    }
}
