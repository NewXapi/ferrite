use super::card::{AdminCard, short_key};
use super::editable::PopoverPanel;
use super::inline_edit::InlineEdit;
use contract::api::admin::AdminUserDto;
use dioxus::prelude::*;

/// 角色单选选项：(wire 位值, 展示文案)。与页面筛选器 `fetch_roles` 同集。
const ROLE_OPTIONS: [(u16, &str); 3] = [(1, "普通用户"), (10, "管理员"), (100, "超级管理员")];

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
/// 分组展示标签由调用方按页面上下文经 `all_groups`（(标签, 分组名) 全集）传入
/// （如取分组管理页的 remark），卡片按名字映射文案。卡片只做展示与行内编辑，
/// 不发任何网络请求；卡内**不再有操作按钮行**（维护者 2026-09-21 批注删除）——
/// 启停 / 保存 / 删除等操作将由卡牌外的图标按钮承担（布局设计待维护者确认后接入）。
///
/// 角色与分组是卡内草稿型 popover（v1 不落库）：角色单选（批注 63a9b93f）、
/// 分组多选 chips + 点击/悬停出全部分组面板（批注 105e61a4）；草稿只改卡内
/// 展示，保存按钮上线后统一写回。
///
/// 用户名 / 邮箱两处是原地编辑行（[`InlineEdit`]，编辑块绝对定位零位移）：
/// 点标题/行 → 值变无边框输入框，Enter 收关、Escape 还原；v1 草稿只留卡内前端
/// 状态，不提交后端——卡牌外「保存」按钮上线后经 `on_commit` 抛回页面统一写回。
#[component]
pub fn UserCard(
    /// The administrative user DTO displayed by this card.
    user: AdminUserDto,
    /// 全部分组选项 (展示标签, 分组名)，来自页面上下文（分组列表 remark 口径）；
    /// 用于分组多选 popover 的全部选项与 chip 文案映射。
    all_groups: Vec<(String, String)>,
    /// 整张卡片的测试标识；默认 `user-card`。
    #[props(default = "user-card".to_string())]
    testid: String,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "额度", "系统"];

    // 卡内草稿（v1 只留前端状态，卡牌外「保存」上线后经 on_commit 抛回页面）：
    // 角色单选 / 分组多选的 popover 只改草稿与展示，不发任何网络请求。
    let mut role_draft = use_signal(|| user.role);
    let mut groups_draft: Signal<Vec<String>> = use_signal(|| user.groups.clone());
    let mut role_open = use_signal(|| false);
    let mut groups_open = use_signal(|| false);

    let role_str = role_label(role_draft()).to_string();
    let status_str = if user.status == 1 { "启用" } else { "停用" }.to_string();
    let short_k = short_key(&user.key);

    let used_pct = if user.quota > 0 {
        ((user.used_quota as f64 / user.quota as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    // 分组名 → 展示标签：取 all_groups 的 remark 口径（与页面 chips 同口径），
    // 取不到回落裸名。
    let group_label = |name: &str| -> String {
        all_groups
            .iter()
            .find(|(_, n)| n == name)
            .map(|(l, _)| l.clone())
            .unwrap_or_else(|| name.to_string())
    };

    // 三个页签内容各自为独立 Element，传给 AdminCard 同格叠加渲染：
    // 容器高度取最高者，切页签时卡片高度不跳动。
    let panel_basic = rsx! {
        div { class: "space-y-2.5",
            // 用户名即卡牌标题（见 title_slot，同一 InlineEdit 组件）；本tab只留
            // 邮箱原地编辑（点行 → 值变无边框输入框；Enter 收关，草稿留卡内——
            // v1 不提交后端，保存按钮上线后经 on_commit 抛回页面）。
            InlineEdit {
                label: "邮箱".to_string(),
                value: user.email.clone(),
                testid: "user-inline-email".to_string(),
                placeholder: "未填写".to_string(),
            }
            // 角色：popover 单选（批注 63a9b93f）。行样式与其他只读行逐字对齐。
            div { class: "relative",
                onkeydown: move |e: KeyboardEvent| {
                    if e.key() == Key::Escape {
                        role_open.set(false);
                    }
                },
                button {
                    class: "flex w-full cursor-pointer items-center justify-between gap-2 text-left text-xs",
                    "data-testid": "user-role-trigger",
                    "aria-expanded": "{role_open()}",
                    "aria-label": "选择角色",
                    onclick: move |_| role_open.toggle(),
                    span { class: "text-zinc-400", "角色" }
                    span { class: "font-medium text-zinc-200", "{role_str}" }
                }
                PopoverPanel {
                    open: role_open,
                    label: "选择角色".to_string(),
                    testid: "user-role-popover".to_string(),
                    content_class: Some("w-40 space-y-1 rounded-xl border border-zinc-700 bg-zinc-900 p-1.5 shadow-2xl shadow-black/50".to_string()),
                    content: rsx! {
                        for (value, label) in ROLE_OPTIONS {
                            button {
                                key: "{value}",
                                class: if role_draft() == value {
                                    "w-full rounded-md bg-zinc-800 px-2 py-1.5 text-left text-xs font-medium text-zinc-100"
                                } else {
                                    "w-full rounded-md px-2 py-1.5 text-left text-xs text-zinc-400 transition-colors hover:bg-zinc-800/60 hover:text-zinc-200"
                                },
                                "data-testid": "user-role-option-{value}",
                                "aria-pressed": "{role_draft() == value}",
                                onclick: move |_| {
                                    role_draft.set(value);
                                    role_open.set(false);
                                },
                                "{label}"
                            }
                        }
                    },
                }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "状态" }
                span { class: "font-medium text-zinc-200", "{status_str}" }
            }
            // 分组：多选 chips + 点击/悬停出全部分组 popover（批注 105e61a4）。
            // 展示只列选中的（无 emoji/icon），文案走 all_groups 标签映射。
            div { class: "relative",
                onkeydown: move |e: KeyboardEvent| {
                    if e.key() == Key::Escape {
                        groups_open.set(false);
                    }
                },
                div {
                    class: "cursor-pointer",
                    "data-testid": "user-group-trigger",
                    "aria-expanded": "{groups_open()}",
                    "aria-label": "选择分组",
                    onclick: move |_| groups_open.toggle(),
                    onmouseenter: move |_| groups_open.set(true),
                    p { class: "text-[11px] text-zinc-400", "分组" }
                    div { class: "mt-1.5 flex flex-wrap gap-1.5",
                        for name in groups_draft() {
                            span {
                                key: "{name}",
                                class: "rounded-full border border-zinc-700 bg-zinc-800/80 px-2 py-0.5 text-[11px] text-zinc-300",
                                "{group_label(&name)}"
                            }
                        }
                        if groups_draft().is_empty() {
                            span { class: "text-[11px] text-zinc-500", "无分组" }
                        }
                    }
                }
                PopoverPanel {
                    open: groups_open,
                    label: "选择分组".to_string(),
                    testid: "user-group-popover".to_string(),
                    content_class: Some("w-64 rounded-xl border border-zinc-700 bg-zinc-900 p-2 shadow-2xl shadow-black/50".to_string()),
                    content: rsx! {
                        div { class: "flex flex-wrap gap-1.5", role: "group", "aria-label": "全部分组",
                            for (label, name) in all_groups.clone() {
                                button {
                                    key: "{name}",
                                    class: if groups_draft().iter().any(|n| n == &name) {
                                        "rounded-full border border-zinc-500 bg-zinc-700 px-2 py-0.5 text-[11px] text-zinc-100"
                                    } else {
                                        "rounded-full border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-400 transition-colors hover:border-zinc-500 hover:text-zinc-200"
                                    },
                                    "data-testid": "user-group-chip-{name}",
                                    "aria-pressed": "{groups_draft().iter().any(|n| n == &name)}",
                                    onclick: move |_| {
                                        let mut cur = groups_draft();
                                        if cur.iter().any(|n| n == &name) {
                                            cur.retain(|n| n != &name);
                                        } else {
                                            cur.push(name.clone());
                                        }
                                        groups_draft.set(cur);
                                    },
                                    "{label}"
                                }
                            }
                            if all_groups.is_empty() {
                                span { class: "text-[11px] text-zinc-500", "暂无分组可选" }
                            }
                        }
                    },
                }
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
                div { class: "h-full rounded-full {usage_tone(used_pct)} transition-all", style: "width: {used_pct:.1}%" }
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
            // 标题（用户名）本身即原地编辑入口：与邮箱行共用同一个 InlineEdit 组件
            // （title_mode chrome），卡上不再有第二处用户名。
            title_slot: rsx! {
                InlineEdit {
                    label: "用户名".to_string(),
                    value: user.username.clone(),
                    testid: "user-inline-username".to_string(),
                    title_mode: true,
                }
            },
            title: "{user.username}",
            // 副标题（显示名）与用户名去重：种子数据里两者常相同，卡上只留一个。
            subtitle: (user.display_name != user.username && !user.display_name.is_empty())
                .then(|| user.display_name.clone()),
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
