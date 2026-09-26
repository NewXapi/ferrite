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

/// 额度进度条配色：随用量升高转告警（与页面旧卡同口径）。
fn usage_tone(used_pct: f64) -> &'static str {
    if used_pct >= 90.0 {
        "bg-red-500"
    } else if used_pct >= 70.0 {
        "bg-amber-500"
    } else {
        "bg-emerald-500"
    }
}

/// Renders a three-tab card for an [`AdminUserDto`].
///
/// 页签按 UI 决策记录 §2.3 收敛为 3 个：基本信息（用户名 / 邮箱 / 角色 / 状态 /
/// 分组）、额度（已用 / 总额 / 进度 / 请求数，`500_000` 内部单位 = `¥1`）、
/// 系统（截断 key / 创建时间）。
///
/// 分组展示标签由调用方按页面上下文映射后经 `group_labels` 传入（如取分组管理页
/// 的 remark），卡片本身不感知页面数据源。操作按钮（编辑 / 充值 / 启停）以
/// `actions` 插槽传入，渲染在基本信息页签底部；插槽内的回调语义与 testid 由
/// 调用方（页面）决定。卡片只做展示，不发任何网络请求。
#[component]
pub fn UserCard(
    /// The administrative user DTO displayed by this card.
    user: AdminUserDto,
    /// 分组展示标签，与 `user.groups` 同序同长；调用方负责把分组名映射为页面
    /// 口径的展示文案（取不到时传裸名即可）。
    group_labels: Vec<String>,
    /// 操作区插槽（如编辑 / 充值 / 启停按钮组）；未传时不渲染操作行。
    #[props(default)]
    actions: Option<Element>,
    /// 整张卡片的测试标识；默认 `user-card`。
    #[props(default = "user-card".to_string())]
    testid: String,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "额度", "系统"];

    let role_str = role_label(user.role).to_string();
    let status_str = if user.status == 1 { "启用" } else { "停用" }.to_string();
    let short_k = short_key(&user.key);

    let used_pct = if user.quota > 0 {
        ((user.used_quota as f64 / user.quota as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    // 三个页签内容各自为独立 Element，传给 AdminCard 同格叠加渲染：
    // 容器高度取最高者，切页签时卡片高度不跳动。
    let panel_basic = rsx! {
        div { class: "space-y-2.5",
            div { class: "flex justify-between gap-2 {crate::TYPE_DESC}",
                span { class: "{crate::C_MUTED}", "用户名" }
                span { class: "font-medium text-zinc-200 truncate", "{user.username}" }
            }
            div { class: "flex justify-between gap-2 {crate::TYPE_DESC}",
                span { class: "{crate::C_MUTED}", "邮箱" }
                span { class: "font-medium text-zinc-200 truncate", "{user.email}" }
            }
            div { class: "flex justify-between gap-2 {crate::TYPE_DESC}",
                span { class: "{crate::C_MUTED}", "角色" }
                span { class: "font-medium text-zinc-200", "{role_str}" }
            }
            div { class: "flex justify-between gap-2 {crate::TYPE_DESC}",
                span { class: "{crate::C_MUTED}", "状态" }
                span { class: "font-medium text-zinc-200", "{status_str}" }
            }
            div { class: "space-y-1.5",
                p { class: "{crate::TYPE_LABEL}", "分组" }
                div { class: "flex flex-wrap gap-1.5",
                    for label in &group_labels {
                        span {
                            class: "rounded-full border border-zinc-700 bg-zinc-800/80 px-2 py-0.5 {crate::TYPE_LABEL}",
                            "{label}"
                        }
                    }
                    if group_labels.is_empty() {
                        span { class: "{crate::TYPE_LABEL}", "无分组" }
                    }
                }
            }
            // 操作区插槽（页面构造的按钮组），挂在基本信息页签底部。
            if let Some(actions) = actions {
                {actions}
            }
        }
    };
    let panel_quota = rsx! {
        div { class: "space-y-2",
            div { class: "flex justify-between gap-2 {crate::TYPE_DESC}",
                span { class: "{crate::C_MUTED}", "已用" }
                span { class: "font-medium text-zinc-200", "{fmt_quota_cny(user.used_quota)}" }
            }
            div { class: "flex justify-between gap-2 {crate::TYPE_DESC}",
                span { class: "{crate::C_MUTED}", "总额" }
                span { class: "font-medium text-zinc-200", "{fmt_quota_cny(user.quota)}" }
            }
            div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                div { class: "h-full rounded-full {usage_tone(used_pct)} transition-all", style: "width: {used_pct:.1}%" }
            }
            div { class: "flex justify-between gap-2 {crate::TYPE_DESC}",
                span { class: "{crate::C_MUTED}", "请求数" }
                span { class: "font-medium text-zinc-200", "{user.request_count}" }
            }
        }
    };
    let panel_system = rsx! {
        div { class: "space-y-2 {crate::TYPE_DESC}",
            div { class: "flex justify-between gap-2",
                span { class: "{crate::C_MUTED}", "Key" }
                span { class: "font-mono text-zinc-200", "{short_k}" }
            }
            div { class: "flex justify-between gap-2",
                span { class: "{crate::C_MUTED}", "创建" }
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
            testid: Some(testid),
            panel_0: panel_basic,
            panel_1: panel_quota,
            panel_2: panel_system,
        }
    }
}
