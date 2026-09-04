use dioxus::prelude::*;
use ui::{ScrollSpyNav, SegmentedCapsule};

use crate::api::current_month_prefix;
use crate::api::{self, User};
use crate::data::*;

/// 弹窗状态:关闭 / 新建 / 编辑某用户
#[derive(Clone, PartialEq)]
enum Form {
    Closed,
    New,
    Edit(u32),
}
/// 编辑弹窗内的页签:基本 / 额度 / 备注 / 绑定
#[derive(Clone, Copy, PartialEq)]
enum FormTab {
    Basic,
    Quota,
    Remark,
    Binding,
}

/// 用户管理面板 - 左侧 ScrollSpyNav + 统计 / 筛选 / 用户卡片三区
#[component]
pub fn UsersPanel() -> Element {
    // —— 区段标题 ——
    const SEC_STATS: &str = "用户概览";
    const SEC_LIST: &str = "用户列表";
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
    let mut f_remark = use_signal(String::new);

    let users = api::fetch_users();
    let groups = api::fetch_groups();
    let statuses = api::fetch_statuses();
    let roles = api::fetch_roles();

    let total = users.len();
    let enabled = users.iter().filter(|u| u.status == 1).count();
    let new_this_month = users
        .iter()
        .filter(|u| u.created.starts_with(&current_month_prefix()))
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
    let open_new = move |_| {
        f_username.set(String::new());
        f_email.set(String::new());
        f_quota.set("5000000".to_string());
        f_group.set("default".to_string());
        f_remark.set(String::new());
        form.set(Form::New);
    };
    let open_edit = move |id: u32| {
        if let Some(u) = api::fetch_user(id) {
            f_username.set(u.username.to_string());
            f_email.set(u.email.to_string());
            f_quota.set(u.quota.to_string());
            f_group.set(u.group.to_string());
            f_remark.set(u.remark.to_string());
            form.set(Form::Edit(id));
        }
    };

    rsx! {
        div { class: "pl-8",

            ScrollSpyNav {
                container: "panel-scroll",
                items: vec![
                    ({SEC_STATS}.to_string(), "users-sec-stats".to_string()),
                    ("筛选".to_string(), "users-sec-filter".to_string()),
                    ({SEC_LIST}.to_string(), "users-sec-list".to_string()),
                ],
            }

            div { class: "flex flex-col gap-6",

                // 1. 统计区
                section { id: "users-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
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
                            onclick: open_new,
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
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
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
                                    on_edit: open_edit,
                                    on_topup: move |id: u32| topup.set(Some(id)),
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
                    remark: f_remark,
                    user: match form() {
                        Form::Edit(id) => api::fetch_user(id),
                        _ => None,
                    },
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

    let inviter_label = user.inviter.unwrap_or("无");
    let last_login_day = user.last_login_at.get(..10).unwrap_or(user.last_login_at);
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

    rsx! {
        div {
            class: "group flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",

            // 头部:头像字母圈 + 名称 + ID
            div { class: "flex items-start gap-3",
                div {
                    class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200",
                    "{initial}"
                }
                div { class: "min-w-0 flex-1",
                    div { class: "flex items-center justify-between gap-2",
                        h3 { class: "truncate text-sm font-medium text-zinc-100", "{user.username}" }
                        span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                            "#{user.id}"
                        }
                    }
                    p { class: "mt-0.5 truncate text-[11px] text-zinc-500", "{user.display_name}" }
                }
            }
            // 徽标行
            div { class: "mt-3 flex flex-wrap gap-1.5",
                Badge { text: group_label(user.group).to_string(), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
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
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "邀请收入" }
                    span { class: "font-medium text-zinc-200", "{fmt_cny(user.aff_income)}" }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "邀请人" }
                    span { class: "font-medium text-zinc-200", "{inviter_label}" }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "创建" }
                    span { class: "font-medium text-zinc-200", title: "{user.created_at}", "{user.created}" }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "最后登录" }
                    span { class: "font-medium text-zinc-200", title: "{user.last_login_at}", "{last_login_day}" }
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
                    if user.status == 1 { {STATUS_DISABLED} } else { {STATUS_ENABLED} }
                }
            }
        }
    }
}

// —— 跨 component 共享文案 (UserCard / UserForm / TopUpForm / UsersPanel 都用) ——
const BTN_CANCEL: &str = "取消";
const STATUS_ENABLED: &str = "启用";
const STATUS_DISABLED: &str = "禁用";
const LBL_EMAIL: &str = "邮箱";
const LBL_QUOTA: &str = "额度";

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
    remark: Signal<String>,
    user: Option<&'static User>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing {
        "编辑用户"
    } else {
        "新建用户"
    };
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建用户"
    };
    let quota_hint = quota()
        .trim()
        .parse::<i64>()
        .map(fmt_cny)
        .unwrap_or_else(|_| "—".to_string());
    let mut tab = use_signal(|| FormTab::Basic);
    const TABS: [(FormTab, &str); 4] = [
        (FormTab::Basic, "基本"),
        (FormTab::Quota, LBL_QUOTA),
        (FormTab::Remark, "备注"),
        (FormTab::Binding, "绑定"),
    ];

    // 绑定页只读行:第三方 + 邮箱;无用户(新建)时全显示 "-"
    let bind_rows: [(&str, &str); 6] = match user {
        Some(u) => [
            ("GitHub", u.bindings.github.unwrap_or("-")),
            ("Discord", u.bindings.discord.unwrap_or("-")),
            ("OIDC", u.bindings.oidc.unwrap_or("-")),
            ("WeChat", u.bindings.wechat.unwrap_or("-")),
            ("Telegram", u.bindings.telegram.unwrap_or("-")),
            (LBL_EMAIL, if u.email.is_empty() { "-" } else { u.email }),
        ],
        None => [
            ("GitHub", "-"),
            ("Discord", "-"),
            ("OIDC", "-"),
            ("WeChat", "-"),
            ("Telegram", "-"),
            (LBL_EMAIL, "-"),
        ],
    };

    let body = match tab() {
        FormTab::Basic => rsx! {
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
                    label { class: "mb-1.5 block text-xs text-zinc-400", "{LBL_EMAIL}" }
                    input {
                        class: MODAL_INPUT,
                        r#type: "email",
                        placeholder: "user@example.com",
                        value: "{email}",
                        oninput: move |e| email.set(e.value()),
                    }
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
        },
        FormTab::Quota => rsx! {
            div { class: "space-y-4",
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "剩余额度 (quota)" }
                    input {
                        class: "{MODAL_INPUT} font-mono",
                        r#type: "text",
                        value: "{quota}",
                        oninput: move |e| quota.set(e.value()),
                    }
                    p { class: "mt-1 text-xs text-zinc-500", "折合 {quota_hint}" }
                }
                if let Some(u) = user {
                    div { class: "space-y-1.5 rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs",
                        div { class: "flex justify-between gap-2",
                            span { class: "text-zinc-400", "当前剩余" }
                            span { class: "font-medium text-zinc-200", "{fmt_cny(u.quota)}" }
                        }
                        div { class: "flex justify-between gap-2",
                            span { class: "text-zinc-400", "已使用" }
                            span { class: "font-medium text-zinc-200", "{fmt_cny(u.used_quota)}" }
                        }
                    }
                }
            }
        },
        FormTab::Remark => rsx! {
            div {
                label { class: "mb-1.5 block text-xs text-zinc-400", "管理员备注(仅管理员可见)" }
                textarea {
                    class: "{MODAL_INPUT} h-28 resize-none",
                    placeholder: "例如: 连续 30 天无登录,待清退",
                    value: "{remark}",
                    oninput: move |e| remark.set(e.value()),
                }
            }
        },
        FormTab::Binding => rsx! {
            div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs",
                p { class: "mb-2 text-[11px] text-zinc-500", "第三方账号绑定(只读,用户在个人资料里管理)" }
                div { class: "space-y-1.5",
                    for (label, value) in bind_rows {
                        div { class: "flex justify-between gap-2",
                            span { class: "text-zinc-400", "{label}" }
                            span { class: "font-medium text-zinc-200", "{value}" }
                        }
                    }
                }
            }
        },
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            // 页签行:弹窗顶部,手机端自动折行
            div { class: "mb-4 flex flex-wrap gap-1.5",
                for (t, label) in TABS {
                    {
                        let on = tab() == t;
                        let tone = if on {
                            "border-zinc-100 bg-zinc-100 text-zinc-900"
                        } else {
                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                        };
                        rsx! {
                            button {
                                class: "rounded-full border px-3 py-1 text-xs font-medium transition-colors {tone}",
                                onclick: move |_| tab.set(t),
                                "{label}"
                            }
                        }
                    }
                }
            }

            {body}

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    {BTN_CANCEL}
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
                    {BTN_CANCEL}
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
