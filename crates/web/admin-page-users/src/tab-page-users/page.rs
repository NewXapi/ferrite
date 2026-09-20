//! 用户管理面板 —— 统计 / 筛选 / 用户卡片三区。
//!
//! 文件名按 spec §1.4 统一为 `page.rs`(跨 crate 重构要按统一文件名定位页面层);
//! 公开组件符号名仍保持 `UsersPanel` 不变(公开面逐字保留)。
//!
//! 本文件只放「状态 + 拉取 effect + 区段组合」,不含渲染细节:
//! 卡片内部展示在 ui-components 的共享 `ui::UserCard`(AdminCard 三页签),
//! `user_card` 只做页面上下文映射与操作插槽,两个弹窗分别在 `user_form` /
//! `topup_form`,分组与角色选择器在 `group_chips` / `role_chips`,文案常量在
//! `shared`。
//!
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_users_api`,写入
//! `users` signal;筛选项(搜索 / 分组 / 状态 / 角色)在前端对返回列表过滤,
//! 与 mock 时期的 UX 一致。enable/disable 与充值走 `manage_user_api`。

use dioxus::prelude::*;
use ui::SegmentedCapsule;
use ui::StatCard;
use ui::{CARD_PAGE_SIZE, CardGrid, Pager, SectionHeader, page_slice};

use client::ApiClient;
use contract::api::admin::{AdminUserDto, ManageUserRequest};

use crate::api::{self, current_month_prefix, list_users_api, manage_user_api};
use crate::data::fmt_cny;

use super::shared::{
    BTN_NEW_USER, BTN_REFRESH, BTN_RETRY, LBL_CONSUMED_TOTAL, LBL_ENABLED_USERS, LBL_GRANTED_TOTAL,
    LBL_NEW_THIS_MONTH, LBL_PERSON, LBL_TOTAL_USERS, MSG_ACTION_ERR, MSG_ACTION_OK, MSG_CREATE_ERR,
    MSG_LOAD_USERS_FAIL, MSG_LOADING, MSG_LOADING_USERS, MSG_NO_MATCH, MSG_SEARCH_HINT,
    MSG_USER_CREATED, OPT_ALL, SEC_FILTER, SEC_LIST, SEC_STATS,
};
use super::topup_form::TopUpForm;
use super::user_card::UserCard;
use super::user_form::UserForm;

/// 弹窗状态:关闭 / 新建 / 编辑某用户(key 标识)
#[derive(Clone, PartialEq)]
enum Form {
    Closed,
    New,
    Edit(String),
}

#[component]
pub fn UsersPanel() -> Element {
    // —— 区段标题 ——
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

    // 拉取触发依赖:仅 reload 计数(挂载 + 写操作成功后的刷新)。
    // 列表长度必须 `peek()`:`.read()` 会让本 effect 订阅自己即将写入的 `users`
    // signal,每条响应 set 都重跑 effect → 重拉死循环(对齐 channels.rs 同款 effect)。
    use_effect(move || {
        let _ = reload();
        // stale-while-revalidate:只有首次(列表为空)才显示「加载中」占位符;
        // 写操作(禁用/充值/编辑)触发的刷新保留旧列表原地更新,不整片闪掉。
        // 闪烁根因:loading=true 会把已渲染的卡片网格换成占位符,拉完再换回。
        if users.peek().is_empty() {
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
        let mut v = vec![(OPT_ALL.to_string(), String::new())];
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
        (total.to_string(), LBL_TOTAL_USERS),
        (enabled.to_string(), LBL_ENABLED_USERS),
        (new_this_month.to_string(), LBL_NEW_THIS_MONTH),
        (fmt_cny(granted), LBL_GRANTED_TOTAL),
        (fmt_cny(consumed), LBL_CONSUMED_TOTAL),
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

    // 分页：列表内部 UI 状态（不跨组件）；筛选后条数变小时 clamp 到最后一页，
    // 不落空页（与 admin-page-admin 各列表同款策略）。
    let mut page = use_signal(|| 0usize);
    let visible = page_slice(&filtered, page(), CARD_PAGE_SIZE).to_vec();

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
                        n.set(Some(MSG_ACTION_OK.to_string()));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("{MSG_ACTION_ERR}:{e}"))),
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
                    role: if msg.starts_with(MSG_ACTION_ERR) { "alert" } else { "status" },
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
                    h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                    div { class: "flex gap-2",
                        button {
                            class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                            "data-testid": "refresh-users",
                            onclick: move |_| reload.set(reload() + 1),
                            "{BTN_REFRESH}"
                        }
                        button {
                            class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                            "data-testid": "new-user",
                            onclick: open_new,
                            "{BTN_NEW_USER}"
                        }
                    }
                }

                input {
                    class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                    r#type: "text",
                    placeholder: MSG_SEARCH_HINT,
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
                SectionHeader {
                    title: SEC_LIST.to_string(),
                    badge: if loading() { MSG_LOADING.to_string() } else { format!("{} {LBL_PERSON}", filtered.len()) },
                    trailing: rsx! {
                        Pager {
                            total: filtered.len(),
                            page,
                            on_change: move |p| page.set(p),
                            testid: "users-pager",
                        }
                    },
                }

                if let Some(e) = err() {
                    div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                        p { class: "text-sm text-red-300", "{MSG_LOAD_USERS_FAIL}" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            onclick: move |_| reload.set(reload() + 1),
                            "{BTN_RETRY}"
                        }
                    }
                } else if loading() {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "{MSG_LOADING_USERS}" }
                    }
                } else if filtered.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "{MSG_NO_MATCH}" }
                    }
                } else {
                    CardGrid { aria_label: SEC_LIST.to_string(), testid: "users-list".to_string(),
                        for user in visible {
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
                                n.set(Some(MSG_USER_CREATED.to_string()));
                                r.set(r() + 1);
                            }
                            Err(e) => n.set(Some(format!("{MSG_CREATE_ERR}:{e}"))),
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
