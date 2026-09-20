use super::card::{AdminCard, short_key};
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

/// Renders a four-tab prototype card for an [`AdminUserDto`].
///
/// The tabs follow the user edit modal's fields: 基本信息 (username / email /
/// role), 分组与备注 (group chips and remark, or status when no remark
/// field exists on the DTO), 额度 (CNY quota where `500_000` internal units
/// equal `¥1`, progress, request count), and 系统 (shortened key, created
/// time).
#[component]
pub fn UserCard(
    /// The administrative user DTO displayed by this card.
    user: AdminUserDto,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "分组与备注", "额度", "系统"];

    let role_str = role_label(user.role).to_string();
    let status_str = if user.status == 1 { "启用" } else { "停用" }.to_string();
    let short_k = short_key(&user.key);

    let used_pct = if user.quota > 0 {
        ((user.used_quota as f64 / user.quota as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    // 四个页签内容各自为独立 Element，传给 AdminCard 同格叠加渲染：
    // 容器高度取最高者，切页签时卡片高度不跳动。
    let panel_basic = rsx! {
        div { class: "space-y-2.5",
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "用户名" }
                span { class: "font-medium text-zinc-200 truncate", "{user.username}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "邮箱" }
                span { class: "font-medium text-zinc-200 truncate", "{user.email}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "角色" }
                span { class: "font-medium text-zinc-200", "{role_str}" }
            }
        }
    };
    // AdminUserDto 无 remark 字段，按规格回退展示 groups + status。
    let panel_groups = rsx! {
        div { class: "space-y-2.5",
            div { class: "space-y-1.5",
                p { class: "text-[11px] text-zinc-400", "分组" }
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
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "状态" }
                span { class: "font-medium text-zinc-200", "{status_str}" }
            }
        }
    };
    let panel_quota = rsx! {
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
    };
    let panel_system = rsx! {
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
    };

    rsx! {
        AdminCard {
            title: "{user.username}",
            subtitle: user.display_name.clone(),
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            testid: Some("user-card-new".to_string()),
            panel_0: panel_basic,
            panel_1: panel_groups,
            panel_2: panel_quota,
            panel_3: Some(panel_system),
        }
    }
}
