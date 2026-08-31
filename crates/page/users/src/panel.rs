use dioxus::prelude::*;
use ui::{ScrollSpyNav, SegmentedCapsule};

use crate::api::{self, THIS_MONTH, User};
use crate::data::*;

/// 弹窗状态:关闭 / 新建 / 编辑某用户
#[derive(Clone, PartialEq)]
enum Form {
    Closed,
    New,
    Edit(u32),
}

/// 用户管理面板 - 左侧 ScrollSpyNav + 统计 / 筛选 / 用户卡片三区
#[component]
pub fn UsersPanel() -> Element {
    let mut search = use_signal(String::new);
    let mut group_idx = use_signal(|| 0usize);
    let mut status_idx = use_signal(|| 0usize);
    let mut role_idx = use_signal(|| 0usize);

    let mut form = use_signal(|| Form::Closed);
    let mut topup = use_signal(|| None::<u32>);

    // 弹窗字段(新建与编辑共用同款表单)
    let mut f_username = use_signal(String::new);
    let mut f_email = use_signal(String::new);
    let mut f_quota = use_signal(|| "5000000".to_string());
    let mut f_group = use_signal(|| "default".to_string());

    let users = api::fetch_users();
    let groups = api::fetch_groups();
    let statuses = api::fetch_statuses();
    let roles = api::fetch_roles();

    let total = users.len();
    let enabled = users.iter().filter(|u| u.status == 1).count();
    let new_this_month = users
        .iter()
        .filter(|u| u.created.starts_with(THIS_MONTH))
        .count();
    let granted: i64 = users.iter().map(|u| u.quota).sum();
    let consumed: i64 = users.iter().map(|u| u.used_quota).sum();

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总用户"),
        (enabled.to_string(), "启用中"),
        (new_this_month.to_string(), "本月新增"),
        (fmt_cny(granted), "总发放额度"),
        (fmt_cny(consumed), "总消耗"),
    ];

    let filtered: Vec<&'static User> = {
        let q = search().trim().to_lowercase();
        let want_group = groups[group_idx()].1;
        let want_status = statuses[status_idx()].1;
        let want_role = roles[role_idx()].1;
        users
            .iter()
            .filter(|u| {
                if !q.is_empty()
                    && !u.username.to_lowercase().contains(&q)
                    && !u.email.to_lowercase().contains(&q)
                {
                    return false;
                }
                if !want_group.is_empty() && u.group != want_group {
                    return false;
                }
                if want_status != 0 && u.status != want_status {
                    return false;
                }
                if want_role != 0 && u.role != want_role {
                    return false;
                }
                true
            })
            .collect()
    };

    // 打开新建/编辑弹窗时预填字段
    let mut open_new = move |_| {
        f_username.set(String::new());
        f_email.set(String::new());
        f_quota.set("5000000".to_string());
        f_group.set("default".to_string());
        form.set(Form::New);
    };
    let mut open_edit = move |id: u32| {
        if let Some(u) = api::fetch_user(id) {
            f_username.set(u.username.to_string());
            f_email.set(u.email.to_string());
            f_quota.set(u.quota.to_string());
            f_group.set(u.group.to_string());
            form.set(Form::Edit(id));
        }
    };

    rsx! {
        div { class: "pl-8",

            ScrollSpyNav {
                container: "panel-scroll",
                items: vec![
                    ("用户概览".to_string(), "users-sec-stats".to_string()),
                    ("筛选".to_string(), "users-sec-filter".to_string()),
                    ("用户列表".to_string(), "users-sec-list".to_string()),
                ],
            }

            div { class: "flex flex-col gap-6",

                // 1. 统计区
                section { id: "users-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "用户概览" }
                    // 宽度约定:手机 1 栏 / 平板 3 栏 / Web 5 栏,每卡各占 1 栏。
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选区
                section {
                    id: "users-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        h2 { class: "text-sm font-medium text-zinc-300", "筛选用户" }
                        button {
                            class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                            onclick: move |e| open_new(e),
                            "✚ 新建用户"
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 outline-none transition focus:border-zinc-500",
                        r#type: "search",
                        placeholder: "搜索用户名或邮箱",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                    }

                    // 分组 / 状态 / 角色:胶囊分段,手机每行最多 3 段
                    div { class: "flex flex-col gap-3",
                        SegmentedCapsule {
                            items: groups.iter().map(|(l, _)| l.to_string()).collect(),
                            active: group_idx(),
                            on_select: move |i: usize| group_idx.set(i),
                        }
                        SegmentedCapsule {
                            items: statuses.iter().map(|(l, _)| l.to_string()).collect(),
                            active: status_idx(),
                            on_select: move |i: usize| status_idx.set(i),
                        }
                        SegmentedCapsule {
                            items: roles.iter().map(|(l, _)| l.to_string()).collect(),
                            active: role_idx(),
                            on_select: move |i: usize| role_idx.set(i),
                        }
                    }
                }

                // 3. 用户卡片网格
                section { id: "users-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "用户列表" }
                        span {
                            class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                            "{filtered.len()} 人"
                        }
                    }

                    if filtered.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的用户" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                            for user in filtered {
                                UserCard {
                                    key: "{user.id}",
                                    user: user,
                                    on_edit: move |id| open_edit(id),
                                    on_topup: move |id| topup.set(Some(id)),
                                }
                            }
                        }
                    }
                }
            }

            // 新建 / 编辑弹窗(同款表单)
            if form() != Form::Closed {
                UserForm {
                    editing: matches!(form(), Form::Edit(_)),
                    username: f_username,
                    email: f_email,
                    quota: f_quota,
                    group: f_group,
                    on_cancel: move |_| form.set(Form::Closed),
                    on_submit: move |_| form.set(Form::Closed),
                }
            }

            // 充值弹窗
            if let Some(id) = topup() {
                if let Some(u) = api::fetch_user(id) {
                    TopUpForm {
                        user: u,
                        on_cancel: move |_| topup.set(None),
                        on_submit: move |_| topup.set(None),
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(value: String, label: &'static str) -> Element {
    rsx! {
        div {
            class: "rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 transition-colors hover:border-zinc-600",
            p { class: "text-xl font-semibold tracking-tight text-white", "{value}" }
            p { class: "mt-0.5 text-xs text-zinc-500", "{label}" }
        }
    }
}

/// 徽标:分组 / 角色 / 状态共用
#[component]
fn Badge(text: String, tone: &'static str) -> Element {
    rsx! {
        span {
            class: "rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}",
            "{text}"
        }
    }
}

#[component]
fn UserCard(
    user: &'static User,
    on_edit: EventHandler<u32>,
    on_topup: EventHandler<u32>,
) -> Element {
    let initial = user
        .display_name
        .chars()
        .next()
        .or_else(|| user.username.chars().next())
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let pct = used_pct(user);
    // 进度条配色随用量升高转告警
    let bar_tone = if pct >= 90 {
        "bg-red-500"
    } else if pct >= 70 {
        "bg-amber-500"
    } else {
        "bg-emerald-500"
    };

    let (status_text, status_tone) = if user.status == 1 {
        ("启用", "border-emerald-500/30 bg-emerald-500/20 text-emerald-400")
    } else {
        ("禁用", "border-zinc-600 bg-zinc-800 text-zinc-400")
    };
    let role_tone = match user.role {
        100 => "border-violet-500/30 bg-violet-500/20 text-violet-300",
        10 => "border-sky-500/30 bg-sky-500/20 text-sky-300",
        _ => "border-zinc-700 bg-zinc-800/80 text-zinc-400",
    };

    rsx! {
        div {
            class: "group flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",

            // 头部:头像字母圈 + 名称
            div { class: "flex items-start gap-3",
                div {
                    class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200",
                    "{initial}"
                }
                div { class: "min-w-0",
                    h3 { class: "truncate text-sm font-medium text-zinc-100", "{user.username}" }
                    p { class: "mt-0.5 truncate text-[11px] text-zinc-500", "{user.display_name}" }
                }
            }
            p { class: "mt-2 truncate font-mono text-[11px] text-zinc-500", "{user.email}" }

            // 徽标行
            div { class: "mt-3 flex flex-wrap gap-1.5",
                Badge { text: group_label(user.group).to_string(), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                Badge { text: role_label(user.role).to_string(), tone: role_tone }
                Badge { text: status_text.to_string(), tone: status_tone }
            }

            // 额度进度条
            div { class: "mt-3 space-y-1.5",
                div { class: "flex justify-between gap-2 text-[11px]",
                    span { class: "text-zinc-400", "额度" }
                    span { class: "whitespace-nowrap font-medium text-zinc-200",
                        "{fmt_cny(user.used_quota)} / {fmt_cny(user.quota)}"
                    }
                }
                div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                    div { class: "h-full rounded-full {bar_tone}", style: "width: {pct}%" }
                }
                p { class: "text-right text-[11px] text-zinc-500", "已用 {pct}%" }
            }

            // 计数行
            div { class: "mt-3 space-y-1.5 text-xs",
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "请求数" }
                    span { class: "font-medium text-zinc-200", "{fmt_num(user.request_count)}" }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "邀请数" }
                    span { class: "font-medium text-zinc-200", "{user.aff_count}" }
                }
            }

            // 操作区
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700 py-1 text-[11px] text-zinc-300 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_edit.call(user.id),
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700 py-1 text-[11px] text-emerald-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_topup.call(user.id),
                    "充值"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700 py-1 text-[11px] text-amber-400 transition-colors hover:bg-zinc-800",
                    if user.status == 1 { "禁用" } else { "启用" }
                }
            }
        }
    }
}

/// 弹窗外壳:遮罩 + 居中卡 + 标题栏关闭按钮
#[component]
fn Modal(title: String, on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "{title}" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        svg {
                            class: "h-5 w-5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                        }
                    }
                }
                {children}
            }
        }
    }
}

const MODAL_INPUT: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none";

#[component]
fn UserForm(
    editing: bool,
    username: Signal<String>,
    email: Signal<String>,
    quota: Signal<String>,
    group: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing { "编辑用户" } else { "新建用户" };
    let submit_label = if editing { "保存修改" } else { "创建用户" };
    let quota_hint = quota()
        .trim()
        .parse::<i64>()
        .map(fmt_cny)
        .unwrap_or_else(|_| "—".to_string());

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "用户名" }
                    input {
                        class: MODAL_INPUT,
                        placeholder: "例如: zhangna",
                        value: "{username}",
                        oninput: move |e| username.set(e.value()),
                    }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "邮箱" }
                    input {
                        class: MODAL_INPUT,
                        r#type: "email",
                        placeholder: "user@example.com",
                        value: "{email}",
                        oninput: move |e| email.set(e.value()),
                    }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "初始额度 (quota)" }
                    input {
                        class: "{MODAL_INPUT} font-mono",
                        r#type: "text",
                        value: "{quota}",
                        oninput: move |e| quota.set(e.value()),
                    }
                    p { class: "mt-1 text-xs text-zinc-500", "折合 {quota_hint}" }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "分组" }
                    select {
                        class: MODAL_INPUT,
                        value: "{group}",
                        onchange: move |e| group.set(e.value()),
                        for (label, value) in api::fetch_groups().iter().skip(1) {
                            option { value: "{value}", "{label}" }
                        }
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                    onclick: move |_| on_submit.call(()),
                    "{submit_label}"
                }
            }
        }
    }
}

#[component]
fn TopUpForm(
    user: &'static User,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let mut amount = use_signal(|| "50".to_string());
    let parsed = amount().trim().parse::<f64>().ok().filter(|v| *v > 0.0);
    let quota_hint = parsed
        .map(|v| format!("{} quota", fmt_num(cny_to_quota(v).max(0) as u32)))
        .unwrap_or_else(|| "请输入有效金额".to_string());
    let after = parsed
        .map(|v| fmt_cny(user.quota + cny_to_quota(v)))
        .unwrap_or_else(|| fmt_cny(user.quota));

    rsx! {
        Modal { title: "额度充值".to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs",
                    div { class: "flex justify-between gap-2",
                        span { class: "text-zinc-400", "用户" }
                        span { class: "font-medium text-zinc-200", "{user.username} · {user.display_name}" }
                    }
                    div { class: "mt-1.5 flex justify-between gap-2",
                        span { class: "text-zinc-400", "当前额度" }
                        span { class: "font-medium text-zinc-200", "{fmt_cny(user.quota)}" }
                    }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "充值金额 (元)" }
                    input {
                        class: "{MODAL_INPUT} font-mono",
                        r#type: "text",
                        value: "{amount}",
                        oninput: move |e| amount.set(e.value()),
                    }
                    p { class: "mt-1 text-xs text-zinc-500", "折合 {quota_hint}" }
                }
                div { class: "flex justify-between gap-2 text-xs",
                    span { class: "text-zinc-400", "充值后额度" }
                    span { class: "font-medium text-emerald-400", "{after}" }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    disabled: parsed.is_none(),
                    onclick: move |_| on_submit.call(()),
                    "确认充值"
                }
            }
        }
    }
}
