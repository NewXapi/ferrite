use dioxus::prelude::*;
use ui::SegmentedCapsule;

use client::ApiClient;
use contract::api::admin::{AdminUserDto, ManageUserRequest};

use crate::api::{self, current_month_prefix, list_users_api, manage_user_api};
use crate::data::*;

/// 弹窗状态:关闭 / 新建 / 编辑某用户(key 标识)
#[derive(Clone, PartialEq)]
enum Form {
    Closed,
    New,
    Edit(String),
}

/// 编辑弹窗内的真实页签。
///
/// 旧版四个 tab(基本/额度/备注/绑定)里「基本/额度/备注」共用同一组字段,
/// 切换只是滚动到不同输入框——视觉上是 tab,语义上不是。现在按字段归属
/// 拆成三组,每组只渲染自己的字段,切换 tab 真正切换内容:
/// - `Basic`:用户名 / 邮箱 / 角色 / 剩余额度(可编辑生效)
/// - `Group`:生效分组(多选) / 管理员备注
/// - `Binding`:第三方绑定(只读,后端暂无此列)
#[derive(Clone, Copy, PartialEq)]
enum FormTab {
    Basic,
    Group,
    Binding,
}

const TAB_LABELS: [(FormTab, &str); 3] = [
    (FormTab::Basic, "基本信息"),
    (FormTab::Group, "分组与备注"),
    (FormTab::Binding, "绑定"),
];

/// 用户管理面板 - 左侧 ScrollSpyNav + 统计 / 筛选 / 用户卡片三区。
///
/// 数据来自真实后端:挂载时 `use_effect` 拉 `list_users_api`,写入
/// `users` signal;筛选项(搜索 / 分组 / 状态 / 角色)在前端对返回列表过滤,
/// 与 mock 时期的 UX 一致。enable/disable 与充值走 `manage_user_api`。
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
    let mut topup = use_signal(|| None::<String>);

    // 弹窗字段(新建与编辑共用同款表单)
    let mut f_username = use_signal(String::new);
    let mut f_email = use_signal(String::new);
    let mut f_password = use_signal(String::new);
    let mut f_quota = use_signal(|| "5000000".to_string());
    // 生效分组(多值,对齐 `auth_users.groups`;后端 `set_groups` 整体替换)
    let mut f_group = use_signal(|| vec!["default".to_string()]);
    let mut f_remark = use_signal(String::new);
    let mut f_role = use_signal(|| 10u16);

    // 真实数据 + 加载/错误态
    let mut users = use_signal(Vec::<AdminUserDto>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    // 真实分组列表(GET /api/group):筛选胶囊与弹窗 chips 共用。
    // 提供为 context,让 GroupChips 不经 props 也能取到同一份列表。
    let mut groups = use_signal(Vec::<(String, String)>::new);
    let _ = use_context_provider(|| groups);
    use_effect(move || {
        spawn(async move {
            let client = ApiClient::shared().clone();
            if let Ok(list) = api::list_groups_api(&client).await {
                groups.set(list);
            }
        });
    });

    use_effect(move || {
        let _ = reload();
        // stale-while-revalidate:只有首次(列表为空)才显示「加载中」占位符;
        // 写操作(禁用/充值/编辑)触发的刷新保留旧列表原地更新,不整片闪掉。
        // 闪烁根因:loading=true 会把已渲染的卡片网格换成占位符,拉完再换回。
        if users.read().is_empty() {
            loading.set(true);
        }
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            // size=100:后端默认 20 会静默截断,统计卡「总用户」会少算
            match list_users_api(&client, None, Some(1), Some(100)).await {
                Ok(page) => {
                    users.set(page.items);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    // 筛选项:分组走真实列表 + 首项「全部」;状态/角色仍是固定枚举
    let filter_groups = {
        let mut v = vec![("全部".to_string(), String::new())];
        v.extend(groups().clone());
        v
    };
    let statuses = api::fetch_statuses();
    let roles = api::fetch_roles();
    let all = users();

    let total = all.len();
    let enabled = all.iter().filter(|u| u.status == 1).count();
    let new_this_month = all
        .iter()
        .filter(|u| u.created_at.starts_with(&current_month_prefix()))
        .count();
    let granted: i64 = all.iter().map(|u| u.quota).sum();
    let consumed: i64 = all.iter().map(|u| u.used_quota).sum();

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总用户"),
        (enabled.to_string(), "启用中"),
        (new_this_month.to_string(), "本月新增"),
        (fmt_cny(granted), "总发放额度"),
        (fmt_cny(consumed), "总消耗"),
    ];

    let filtered: Vec<AdminUserDto> = {
        let q = search().trim().to_lowercase();
        let want_group = &filter_groups[group_idx()].1;
        let want_status = statuses[status_idx()].1;
        let want_role = roles[role_idx()].1;
        all.iter()
            .filter(|u| {
                if !q.is_empty()
                    && !u.username.to_lowercase().contains(&q)
                    && !u.email.to_lowercase().contains(&q)
                {
                    return false;
                }
                if !want_group.is_empty() && !u.groups.contains(want_group) {
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
            .cloned()
            .collect()
    };

    // 打开新建/编辑弹窗时预填字段
    let open_new = move |_: MouseEvent| {
        f_username.set(String::new());
        f_email.set(String::new());
        f_password.set(String::new());
        f_quota.set("5000000".to_string());
        f_group.set(vec!["default".to_string()]);
        f_remark.set(String::new());
        f_role.set(1);
        form.set(Form::New);
    };
    let open_edit = move |key: String| {
        if let Some(u) = users().iter().find(|u| u.key == key) {
            f_username.set(u.username.clone());
            f_email.set(u.email.clone());
            f_quota.set(u.quota.to_string());
            // 后端 groups 可空(清空分组态),原样回填,chips 全不选即表达
            f_group.set(u.groups.clone());
            f_remark.set(String::new());
            f_role.set(u.role);
            form.set(Form::Edit(key));
        }
    };

    // 写操作助手工厂:返回独立闭包,分别交给 UserCard(启用/禁用)、
    // UserForm(编辑回写)、TopUpForm(充值)。Signal 是 Copy;每次调用先复制一份
    // 再 move 进 async,避免把闭包捕获的 signal 移动出去(FnMut 不允许)。
    let make_manage = || {
        let captured = (busy, notice, reload);
        move |key: String, action: String, value: Option<String>| {
            let (mut b, mut n, mut r) = captured;
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                let req = ManageUserRequest { key, action, value };
                match manage_user_api(&client, &req).await {
                    Ok(_) => {
                        n.set(Some("操作成功".to_string()));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("操作失败:{e}"))),
                }
                b.set(false);
            });
        }
    };
    let manage_toggle = make_manage();
    let manage_form = make_manage();
    let manage_topup = make_manage();

    // 弹窗编辑态:从 form() 派生,放在 rsx! 之外(宏内不允许 let 语句)
    let editing = matches!(form(), Form::Edit(_));
    let edit_key = match form() {
        Form::Edit(k) => Some(k),
        _ => None,
    };
    // 充值弹窗的当前额度:同样放在 rsx! 之外计算
    let topup_quota = topup()
        .and_then(|k| users().iter().find(|u| u.key == k).map(|u| u.quota))
        .unwrap_or(0);

    rsx! {
        div { class: "flex flex-col gap-6", "data-testid": "users-panel",

            // 通知条(成功/错误/进行中)
            if let Some(msg) = notice() {
                div { class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
                    role: if msg.starts_with("操作失败") { "alert" } else { "status" },
                    "{msg}"
                    if busy() { " ···" }
                }
            }

            // 1. 统计区
            section { id: "users-sec-stats", class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                // 宽度约定:手机 1 栏 / 平板 3 栏 / Web 5 栏,每卡各占 1 栏。
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
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
                    div { class: "flex gap-2",
                        button {
                            class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                            "data-testid": "refresh-users",
                            onclick: move |_| reload.set(reload() + 1),
                            "刷新"
                        }
                        button {
                            class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                            "data-testid": "new-user",
                            onclick: open_new,
                            "✚ 新建用户"
                        }
                    }
                }

                input {
                    class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                    r#type: "text",
                    placeholder: "搜索用户名或邮箱",
                    "data-testid": "users-search",
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }

                // 分组 / 状态 / 角色:胶囊分段,手机每行最多 3 段
                div { class: "flex flex-col gap-3",
                    SegmentedCapsule {
                        items: filter_groups.iter().map(|(l, _)| l.clone()).collect(),
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
                        if loading() { "加载中…" } else { "{filtered.len()} 人" }
                    }
                }

                if let Some(e) = err() {
                    div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                        p { class: "text-sm text-red-300", "加载用户失败" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            onclick: move |_| reload.set(reload() + 1),
                            "重试"
                        }
                    }
                } else if loading() {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "正在加载用户…" }
                    }
                } else if filtered.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "没有匹配的用户" }
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for user in filtered {
                            UserCard {
                                key: "{user.key}",
                                user,
                                on_edit: open_edit,
                                on_topup: move |key: String| topup.set(Some(key)),
                                on_toggle: move |(k, a, v): (String, String, Option<String>)| manage_toggle(k, a, v),
                            }
                        }
                    }
                }
            }
        }

        // 新建 / 编辑弹窗(同款表单)
        if form() != Form::Closed {
            UserForm {
                editing,
                edit_key: edit_key.clone(),
                username: f_username,
                email: f_email,
                password: f_password,
                quota: f_quota,
                group: f_group,
                remark: f_remark,
                role: f_role,
                on_cancel: move |_| form.set(Form::Closed),
                // 编辑态:按 tab 回写单字段(set_role / set_group)
                on_submit: move |(action, value): (String, Option<String>)| {
                    if let Some(k) = edit_key.clone() {
                        manage_form(k, action, value);
                    }
                    form.set(Form::Closed);
                },
                // 新建态:一次性 POST /api/user/users 建号
                on_create: move |req: api::CreateUserRequest| {
                    let mut n = notice;
                    let mut r = reload;
                    let mut b = busy;
                    n.set(None);
                    // 与 manage 路径同款:创建期间置 busy,通知条显示进行中
                    b.set(true);
                    spawn(async move {
                        let client = ApiClient::shared().clone();
                        match api::create_user_api(&client, &req).await {
                            Ok(_) => {
                                n.set(Some("用户已创建".to_string()));
                                r.set(r() + 1);
                            }
                            Err(e) => n.set(Some(format!("创建失败:{e}"))),
                        }
                        b.set(false);
                    });
                    form.set(Form::Closed);
                },
            }
        }

        // 充值弹窗
        if let Some(key) = topup() {
            TopUpForm {
                user_key: key,
                current_quota: topup_quota,
                on_cancel: move |_| topup.set(None),
                on_submit: move |(k, amount): (String, i64)| {
                    manage_topup(k, "adjust_quota".to_string(), Some(amount.to_string()));
                    topup.set(None);
                },
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

// —— 跨 component 共享文案 (UserCard / UserForm / TopUpForm 都用) ——
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
    edit_key: Option<String>,
    username: Signal<String>,
    email: Signal<String>,
    password: Signal<String>,
    quota: Signal<String>,
    group: Signal<Vec<String>>,
    remark: Signal<String>,
    role: Signal<u16>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<(String, Option<String>)>,
    on_create: EventHandler<api::CreateUserRequest>,
) -> Element {
    let _ = edit_key;
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
    // 额度输入的元换算提示:内部单位 → 人民币,不暴露 quota 字样
    let quota_hint = quota()
        .trim()
        .parse::<i64>()
        .map(fmt_cny)
        .unwrap_or_else(|_| "—".to_string());
    let mut tab = use_signal(|| FormTab::Basic);

    // 绑定页邮箱展示:避免对临时 String 取引用导致其被提前释放。
    let email_str = email();
    let email_bound: &str = if email_str.is_empty() {
        "-"
    } else {
        &email_str
    };

    // 保存:编辑态按 tab 回写单字段,新建态走 on_create 一次性建号。
    let do_submit = move || {
        if editing {
            match tab() {
                FormTab::Basic => {
                    on_submit.call(("set_role".to_string(), Some(role().to_string())));
                }
                // 多分组整体替换:后端 set_groups 收逗号/空白分隔列表
                FormTab::Group => {
                    on_submit.call(("set_groups".to_string(), Some(group().join(","))));
                }
                // 绑定页只读,无写路径(按钮也不渲染)
                FormTab::Binding => {}
            }
        } else {
            on_create.call(api::CreateUserRequest {
                username: username(),
                password: password(),
                email: if email().trim().is_empty() {
                    None
                } else {
                    Some(email())
                },
                role: role(),
                quota: quota().trim().parse().unwrap_or(0),
                groups: group(),
            });
        }
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            // 页签行:弹窗顶部,手机端自动折行
            div { class: "mb-4 flex flex-wrap gap-1.5",
                for (t, label) in TAB_LABELS {
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
                                "data-testid": "user-form-tab-{label}",
                                onclick: move |_| tab.set(t),
                                "{label}"
                            }
                        }
                    }
                }
            }

            // —— Tab 1:基本信息(用户名/邮箱/初始密码/角色/额度) ——
            if tab() == FormTab::Basic {
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
                    // 初始密码仅新建时填写;编辑态改密走独立 reset_password 动作
                    if !editing {
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "初始密码" }
                            input {
                                class: MODAL_INPUT,
                                r#type: "password",
                                placeholder: "至少 8 位",
                                value: "{password}",
                                oninput: move |e| password.set(e.value()),
                            }
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
                        label { class: "mb-1.5 block text-xs text-zinc-400", "角色权限" }
                        RoleChips { role, on_change: move |v: u16| role.set(v) }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{LBL_QUOTA}" }
                        input {
                            class: "{MODAL_INPUT} font-mono",
                            r#type: "text",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                        p { class: "mt-1 text-xs text-zinc-500", "折合 {quota_hint}" }
                    }
                }
            }

            // —— Tab 2:分组与备注 ——
            if tab() == FormTab::Group {
                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "生效分组" }
                        GroupChips { group, on_change: move |v: Vec<String>| group.set(v) }
                        p { class: "mt-1 text-xs text-zinc-500", "点击分组切换选中,可多选;首个分组为计费生效分组。" }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "管理员备注(仅管理员可见)" }
                        textarea {
                            class: "{MODAL_INPUT} h-24 resize-none",
                            placeholder: "例如: 连续 30 天无登录,待清退",
                            value: "{remark}",
                            oninput: move |e| remark.set(e.value()),
                        }
                    }
                }
            }

            // —— Tab 3:绑定(只读) ——
            if tab() == FormTab::Binding {
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs",
                    p { class: "mb-2 text-[11px] text-zinc-500", "第三方账号绑定(后端暂未返回,只读)" }
                    div { class: "space-y-1.5",
                        for (label, value) in [("GitHub", "-"), ("Discord", "-"), ("OIDC", "-"), ("WeChat", "-"), ("Telegram", "-"), (LBL_EMAIL, email_bound)] {
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "{label}" }
                                span { class: "font-medium text-zinc-200", "{value}" }
                            }
                        }
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    {BTN_CANCEL}
                }
                if tab() != FormTab::Binding {
                    button {
                        class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                        // 每个 tab 只回写自己负责的字段(角色 / 分组)。
                        onclick: move |_| do_submit(),
                        "{submit_label}"
                    }
                }
            }
        }
    }
}

/// 分组选择器:形似输入框的覆盖面板,chips 多选,点选切换选中态。
///
/// 后端 `auth_users.groups` 是 TEXT[] 多值(`set_groups` 整体替换);
/// 选中集合在父级 signal 里维护,本组件只负责渲染与上报增删。
/// 首个选中项即生效分组(计费组倍率 / token 未设组时的回落值)。
#[component]
fn GroupChips(group: Signal<Vec<String>>, on_change: EventHandler<Vec<String>>) -> Element {
    // 分组列表由面板拉取后注入;本组件只读,不认识来源
    let groups = use_context::<Signal<Vec<(String, String)>>>();
    let list = groups();

    rsx! {
        div {
            class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2.5 focus-within:border-zinc-500",
            "data-testid": "user-group-chips",
            role: "group",
            "aria-label": "生效分组选择",

            if list.is_empty() {
                p { class: "text-xs text-zinc-500", "暂无分组(后端 /api/group 为空)" }
            } else {
                div { class: "flex flex-wrap gap-1.5",
                    for (label, value) in list.iter() {
                        {
                            let on = group().iter().any(|g| g == value);
                            let tone = if on {
                                "border-zinc-100 bg-zinc-100 text-zinc-900"
                            } else {
                                "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                            };
                            let v = value.clone();
                            rsx! {
                                button {
                                    class: "rounded-full border px-2.5 py-0.5 text-xs font-medium transition-colors {tone}",
                                    "data-testid": "user-group-chip-{value}",
                                    "aria-pressed": "{on}",
                                    onclick: move |_| {
                                        let mut next: Vec<String> = group()
                                            .iter()
                                            .filter(|g| *g != &v)
                                            .cloned()
                                            .collect();
                                        if next.len() == group().len() {
                                            // 原集合不含本项 = 新增
                                            next.push(v.clone());
                                        }
                                        on_change.call(next);
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 角色选择器:单选 chips 面板(替代原生 `<select>`,与分组 chips 同款外观)。
///
/// 角色是单值枚举(1 | 10 | 100),选中即替换;候选取 `fetch_roles()`
/// 的非「全部」项,标签与筛选胶囊/卡片徽标同源。
#[component]
fn RoleChips(role: Signal<u16>, on_change: EventHandler<u16>) -> Element {
    let items: Vec<(&'static str, u16)> = api::fetch_roles()
        .iter()
        .filter(|(_, v)| *v != 0)
        .copied()
        .collect();

    rsx! {
        div {
            class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2.5 focus-within:border-zinc-500",
            "data-testid": "user-role-chips",
            role: "group",
            "aria-label": "角色权限选择",

            div { class: "flex flex-wrap gap-1.5",
                for (label, value) in items {
                    {
                        let on = role() == value;
                        let tone = if on {
                            "border-zinc-100 bg-zinc-100 text-zinc-900"
                        } else {
                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                        };
                        rsx! {
                            button {
                                class: "rounded-full border px-2.5 py-0.5 text-xs font-medium transition-colors {tone}",
                                "data-testid": "user-role-chip-{value}",
                                "aria-pressed": "{on}",
                                onclick: move |_| on_change.call(value),
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TopUpForm(
    user_key: String,
    current_quota: i64,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<(String, i64)>,
) -> Element {
    let mut amount = use_signal(|| "50".to_string());
    let parsed = amount().trim().parse::<f64>().ok().filter(|v| *v > 0.0);
    // 充值金额(元) → 内部额度增量;展示充值后额度
    let delta_quota = parsed.map(|v| cny_to_quota(v).max(0));
    let after = delta_quota
        .map(|d| fmt_cny(current_quota + d))
        .unwrap_or_else(|| fmt_cny(current_quota));

    rsx! {
        Modal { title: "额度充值".to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs",
                    div { class: "flex justify-between gap-2",
                        span { class: "text-zinc-400", "当前{LBL_QUOTA}" }
                        span { class: "font-medium text-zinc-200", "{fmt_cny(current_quota)}" }
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
                    p { class: "mt-1 text-xs text-zinc-500", "折合 {fmt_cny(delta_quota.unwrap_or(0))}" }
                }
                div { class: "flex justify-between gap-2 text-xs",
                    span { class: "text-zinc-400", "充值后{LBL_QUOTA}" }
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
                    onclick: move |_| on_submit.call((user_key.clone(), delta_quota.unwrap_or(0))),
                    "确认充值"
                }
            }
        }
    }
}
