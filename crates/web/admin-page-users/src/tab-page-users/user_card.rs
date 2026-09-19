//! 单张用户卡:头像 + 徽标行 + 额度进度条 + 计数行 + 操作区。
//!
//! 面板只负责网格布局与回调分发,卡片内部的展示推导(配色 tone、日期截断、
//! 分组标签回落)都在本文件内,面板不感知这些细节。

use contract::api::admin::AdminUserDto;
use dioxus::prelude::*;

use crate::data::{fmt_cny, fmt_created_date, fmt_num, role_label, short_key, used_pct};

use super::badge::Badge;
use super::labels::{LBL_QUOTA, STATUS_DISABLED, STATUS_ENABLED};

#[component]
pub fn UserCard(
    user: AdminUserDto,
    on_edit: EventHandler<String>,
    on_topup: EventHandler<String>,
    on_toggle: EventHandler<(String, String, Option<String>)>,
) -> Element {
    let initial = user
        .display_name
        .chars()
        .next()
        .or_else(|| user.username.chars().next())
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let pct = used_pct(user.quota, user.used_quota);
    // 进度条配色随用量升高转告警
    let bar_tone = if pct >= 90 {
        "bg-red-500"
    } else if pct >= 70 {
        "bg-amber-500"
    } else {
        "bg-emerald-500"
    };

    let (status_text, status_tone) = if user.status == 1 {
        (
            STATUS_ENABLED,
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
        )
    } else {
        (STATUS_DISABLED, "border-zinc-600 bg-zinc-800 text-zinc-400")
    };
    let role_tone = match user.role {
        100 => "border-violet-500/30 bg-violet-500/20 text-violet-300",
        10 => "border-sky-500/30 bg-sky-500/20 text-sky-300",
        _ => "border-zinc-700 bg-zinc-800/80 text-zinc-400",
    };

    // 头像右侧一行:名称为主,id 截断为次、可收缩、不抢占名称空间
    let key_short = short_key(&user.key);
    let created_date = fmt_created_date(&user.created_at);

    // 分组名 → 展示标签:取分组列表里的 remark(与分组管理页同口径),
    // 取不到回落裸名 —— 卡片与弹窗 chips 必须显示同一套文案
    let groups_ctx = use_context::<Signal<Vec<(String, String)>>>();
    let group_badges: Vec<String> = user
        .groups
        .iter()
        .map(|name| {
            groups_ctx()
                .iter()
                .find(|(_, n)| n == name)
                .map(|(l, _)| l.clone())
                .unwrap_or_else(|| name.clone())
        })
        .collect();

    // 三个回调各自持有 key 的副本(EventHandler 是 move 捕获,String 不可 Copy)。
    let edit_key = user.key.clone();
    let topup_key = user.key.clone();
    let toggle_key = user.key.clone();

    rsx! {
        div {
            class: "group flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            role: "listitem",
            "aria-label": "用户 {user.username}",
            "data-testid": "user-card",

            // 头部:头像字母圈 + 名称 + 截断 key,严格单行
            div { class: "flex items-center gap-3",
                div {
                    class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200",
                    "{initial}"
                }
                div { class: "flex min-w-0 flex-1 items-baseline gap-2",
                    h3 { class: "shrink-0 truncate text-sm font-medium text-zinc-100", "{user.username}" }
                    // UUID 截断:悬停 title 看全值,单行内不凸出卡片
                    span {
                        class: "min-w-0 shrink truncate rounded bg-zinc-800/80 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400",
                        title: "ID {user.key}",
                        "{key_short}"
                    }
                }
            }
            p { class: "mt-1 truncate text-[11px] text-zinc-500", "{user.display_name}" }

            // 徽标行:分组一列一个(多分组多徽标),标签取分组管理页口径
            div { class: "mt-3 flex flex-wrap gap-1.5",
                for label in &group_badges {
                    Badge { text: label.clone(), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                }
                Badge { text: role_label(user.role).to_string(), tone: role_tone }
                Badge { text: status_text.to_string(), tone: status_tone }
            }

            // 额度进度条
            div { class: "mt-3 space-y-1.5",
                div { class: "flex justify-between gap-2 text-[11px]",
                    span { class: "text-zinc-400", "{LBL_QUOTA}" }
                    span { class: "whitespace-nowrap font-medium text-zinc-200",
                        "{fmt_cny(user.used_quota)} / {fmt_cny(user.quota)}"
                    }
                }
                div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                    div { class: "h-full rounded-full {bar_tone}", style: "width: {pct}%" }
                }
            }

            // 计数行:创建时间以日期为主,悬停看完整时刻
            div { class: "mt-3 space-y-1.5 text-xs",
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "请求数" }
                    span {
                        class: "font-medium text-zinc-200",
                        // 后端 UserView 无此列 → 缺省 0;有值时千分位
                        title: "接口未返回请求数时的默认值",
                        "{fmt_num(user.request_count as u32)}"
                    }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "创建" }
                    span {
                        class: "whitespace-nowrap font-medium text-zinc-200",
                        title: "{user.created_at}",
                        "{created_date}"
                    }
                }
            }

            // 操作区
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    onclick: move |_| on_edit.call(edit_key.clone()),
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300",
                    onclick: move |_| on_topup.call(topup_key.clone()),
                    "充值"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300",
                    onclick: move |_| on_toggle.call((
                        toggle_key.clone(),
                        // wire 动作名是 snake_case 英文(后端 ManageUserAction 拒中文变体);
                        // 按钮文案仍显示中文,仅 value 走 enable/disable
                        if user.status == 1 { "disable".to_string() } else { "enable".to_string() },
                        None,
                    )),
                    if user.status == 1 { {STATUS_DISABLED} } else { {STATUS_ENABLED} }
                }
            }
        }
    }
}
