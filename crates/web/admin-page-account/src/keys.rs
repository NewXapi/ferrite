use dioxus::prelude::*;

use contract::api::token::{CreateTokenRequest, CreateTokenResult, TokenDto, UpdateTokenRequest};
use contract::api::user::{UserDto, role_label};

use crate::api;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

/// 拉取当前用户的密钥列表 (GET /api/token, owner 模式) 并写回三个 Signal。
/// 首次加载与 create/update/delete 成功后刷新共用此入口。
/// `Signal` 为 `Rc` 句柄 (Copy), 直接按值传入即可共享同一底层值。
fn load_keys(mut k: Signal<Vec<TokenDto>>, mut kl: Signal<bool>, mut ke: Signal<String>) {
    let client = client::ApiClient::shared().clone();
    spawn(async move {
        match api::list_tokens_api(&client).await {
            Ok(v) => {
                ke.set(String::new());
                k.set(v);
                kl.set(true);
            }
            Err(e) => ke.set(e.to_string()),
        }
    });
}

/// 密钥·资料面板 — 资料/密钥区全部走真实 API:
/// - 个人资料: GET/PUT /api/user/self
/// - 我的密钥: GET /api/token (owner 模式) + create/update/delete
/// - 个人数据卡: 密钥总数 (列表长度) / 剩余额度 (/self quota-used_quota)
///   / 成功率与近 30 天聚合暂无数据源 (daily 端点待 overview 会话合入后接回), 显示「—」
#[component]
pub fn KeysPanel() -> Element {
    // —— 区段标题 (ScrollSpyNav + h2 同用) ——
    const SEC_STATS: &str = "个人数据";
    const SEC_PROFILE: &str = "个人资料";
    const SEC_KEYS: &str = "我的密钥";

    let mut show_new_form = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let mut new_group = use_signal(String::new);
    let mut new_quota = use_signal(String::new);

    // ---- 真实用户信息 (GET /api/user/self) ----
    let self_user = use_signal(|| None::<UserDto>);
    let self_err = use_signal(String::new);

    // ---- 我的密钥 (GET /api/token) + 近 30 天按天统计 ----
    let keys = use_signal(Vec::<TokenDto>::new);
    let keys_loaded = use_signal(|| false);
    let keys_err = use_signal(String::new);

    // 新建成功后的明文展示 (只出现一次)
    let mut created_key = use_signal(|| None::<CreateTokenResult>);
    // 编辑 / 删除目标
    let mut edit_key = use_signal(|| None::<TokenDto>);
    let mut del_key = use_signal(|| None::<TokenDto>);

    use_hook(move || {
        let client = client::ApiClient::shared().clone();

        // self 资料
        let mut su = self_user;
        let mut se = self_err;
        spawn(async move {
            match api::get_self_api(&client).await {
                Ok(u) => {
                    // 同步刷新共享缓存 (UserBadge / 顶栏读取 ferrite_current_user)
                    if let Ok(s) = serde_json::to_string(&u) {
                        ui::set_storage_item("ferrite_current_user", &s);
                    }
                    su.set(Some(u));
                }
                Err(e) => se.set(e.to_string()),
            }
        });

        // 密钥列表首载 (create/update/delete 成功后同入口刷新)
        load_keys(keys, keys_loaded, keys_err);
    });

    let remaining: Option<i64> = self_user().as_ref().map(|u| u.quota - u.used_quota);

    let pending = String::from("…");
    let none_v = String::from("—");

    rsx! {
        div { class: "flex flex-col gap-6",
            // 1. 统计区
            section {
                id: "keys-sec-stats",
                class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    StatCard {
                        value: if keys_loaded() { keys().len().to_string() } else { pending.clone() },
                        label: "密钥总数",
                    }
                    StatCard { value: none_v.clone(), label: "近 30 天消耗 (暂无数据)" }
                    StatCard { value: none_v.clone(), label: "近 30 天请求 (暂无数据)" }
                    StatCard {
                        value: match remaining {
                            Some(v) => v.to_string(),
                            None if self_err().is_empty() => pending.clone(),
                            None => none_v.clone(),
                        },
                        label: "剩余额度",
                    }
                    StatCard {
                        value: none_v,
                        label: "成功率 (暂无数据)",
                    }
                }
            }

            // 2. 个人资料区
            section {
                id: "keys-sec-profile",
                class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_PROFILE}" }
                div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6",
                    div { class: "flex items-start justify-between gap-4",
                        div { class: "min-w-0",
                            if let Some(user) = self_user() {
                                div { class: "mb-4 flex items-center gap-2",
                                    span { class: "truncate text-sm font-medium text-zinc-100", "{user.username}" }
                                    span { class: "shrink-0 rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] font-medium text-zinc-400", "{role_label(user.role)}" }
                                }
                                // 资料项横向流式排布, 一行放不下自动换行。
                                // 无「分组」行: group 是创建密钥时的分组语义, 不属于用户资料。
                                div { class: "flex flex-wrap items-baseline gap-x-14 gap-y-4 text-sm",
                                    ProfileItem { label: "显示名", value: user.display_name.clone() }
                                    ProfileItem { label: "邮箱", value: if user.email.is_empty() { "—".to_string() } else { user.email.clone() } }
                                    ProfileItem { label: "用户ID", value: user.key.clone() }
                                    ProfileItem { label: "注册时间", value: user.created_at.chars().take(10).collect::<String>() }
                                }
                            } else if !self_err().is_empty() {
                                p { class: "text-sm text-amber-400", "无法加载用户信息 (未登录或请求失败): {self_err()}" }
                            } else {
                                p { class: "text-sm text-zinc-500", "加载中…" }
                            }
                        }
                    }
                }
            }

            // 3. 我的密钥区
            section {
                id: "keys-sec-keys",
                class: "scroll-mt-8",
                div { class: "space-y-4",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-lg font-medium text-zinc-100", "{SEC_KEYS}" }
                            span { class: "text-xs px-3 py-1 rounded-full bg-zinc-800 text-zinc-400",
                                if keys_loaded() { "{keys().len()} 个" } else { "…" }
                            }
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Sm,
                            onclick: move |_| show_new_form.set(true),
                            "✚ 新建密钥"
                        }
                    }

                    if !keys_err().is_empty() {
                        p { class: "text-sm text-amber-400", "无法加载密钥 (未登录或请求失败): {keys_err()}" }
                    } else if !keys_loaded() {
                        p { class: "text-sm text-zinc-500", "加载中…" }
                    } else if keys().is_empty() {
                        p { class: "text-sm text-zinc-500", "还没有密钥,点「✚ 新建密钥」签发第一个" }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for t in keys() {
                                KeyCard {
                                    entry: t,
                                    on_edit: move |tk: TokenDto| {
                                        edit_key.set(Some(tk));
                                    },
                                    on_toggle: move |tk: TokenDto| {
                                        let mut k = keys;
                                        let mut kl = keys_loaded;
                                        let mut ke = keys_err;
                                        let client = client::ApiClient::shared().clone();
                                        spawn(async move {
                                            let next_status = if tk.status == 1 { 2 } else { 1 };
                                            let req = UpdateTokenRequest {
                                                status: Some(next_status),
                                                ..Default::default()
                                            };
                                            if api::update_token_api(&client, &tk.key, &req).await.is_ok() {
                                                match api::list_tokens_api(&client).await {
                                                    Ok(v) => { ke.set(String::new()); k.set(v); kl.set(true); }
                                                    Err(e) => ke.set(e.to_string()),
                                                }
                                            }
                                        });
                                    },
                                    on_delete: move |tk: TokenDto| {
                                        del_key.set(Some(tk));
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }

        // 新建密钥弹窗
        if show_new_form() {
            NewKeyForm {
                name: new_name,
                group: new_group,
                quota: new_quota,
                on_cancel: move || show_new_form.set(false),
                on_created: move |res: CreateTokenResult| {
                    new_name.set(String::new());
                    new_group.set(String::new());
                    new_quota.set(String::new());
                    show_new_form.set(false);
                    created_key.set(Some(res));
                    load_keys(keys, keys_loaded, keys_err);
                },
            }
        }

        // 新建成功: 一次性明文展示
        if let Some(res) = created_key() {
            CreatedKeyView {
                result: res,
                on_close: move || {
                    created_key.set(None);
                },
            }
        }

        // 编辑密钥弹窗
        if let Some(t) = edit_key() {
            EditKeyModal {
                token: t,
                on_cancel: move || edit_key.set(None),
                on_saved: move |_| {
                    edit_key.set(None);
                    load_keys(keys, keys_loaded, keys_err);
                },
            }
        }

        // 删除确认弹窗
        if let Some(t) = del_key() {
            DeleteKeyModal {
                token: t,
                on_cancel: move || del_key.set(None),
                on_confirmed: move |_| {
                    del_key.set(None);
                    load_keys(keys, keys_loaded, keys_err);
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

/// 横向资料项: label 与 value 同行 (label 灰、value 等宽字体)。
/// 由父容器 flex-wrap 控制换行, 单项不自带换行逻辑。
#[component]
fn ProfileItem(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "shrink-0 text-zinc-400", "{label}" }
            span { class: "min-w-0 break-all font-mono text-zinc-200", "{value}" }
        }
    }
}

/// 密钥卡片 — 数据来自 TokenDto (GET /api/token)。
/// 操作: 编辑 (改名) / 停用·启用 (status 1↔2) / 删除, 成功后由父面板刷新列表。
#[component]
fn KeyCard(
    entry: TokenDto,
    on_edit: EventHandler<TokenDto>,
    on_toggle: EventHandler<TokenDto>,
    on_delete: EventHandler<TokenDto>,
) -> Element {
    let enabled = entry.status == 1;
    let status_color = if enabled {
        "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-amber-500/20 text-amber-400 border-amber-500/30"
    };
    // 每个 handler 闭包各持一份 clone, 避免 3 个 move 闭包连环占用 entry
    let e_edit = entry.clone();
    let e_toggle = entry.clone();
    let e_del = entry.clone();
    // RFC3339 → 展示取日期段 (无数据时不渲染)
    let created: String = entry
        .created_at
        .chars()
        .take(10)
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    let usage = if entry.unlimited_quota {
        "∞".to_string()
    } else {
        format!("{}/{}", entry.used_quota, entry.quota)
    };

    rsx! {
        div {
            class: "group rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "mb-3 flex items-start justify-between gap-2",
                div { class: "min-w-0",
                    h3 { class: "truncate text-sm font-medium text-zinc-100", "{entry.name}" }
                    div { class: "mt-0.5 flex items-center gap-1",
                        p { class: "min-w-0 truncate font-mono text-[11px] text-zinc-500", "{entry.key_preview}" }
                        CopyKeyButton { text: entry.key_preview.clone() }
                    }
                }
                span {
                    class: "shrink-0 rounded-full border px-2.5 py-0.5 text-xs font-medium {status_color}",
                    if enabled { "启用" } else { "停用" }
                }
            }

            div { class: "space-y-2 text-xs",
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "已用额度" }
                    span { class: "whitespace-nowrap font-medium text-zinc-200", "{usage}" }
                }
                if !created.is_empty() {
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 whitespace-nowrap text-zinc-400", "创建时间" }
                        span { class: "whitespace-nowrap font-mono text-zinc-400", "{created}" }
                    }
                }
            }

            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                Button {
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Xs,
                    class: "flex-1",
                    onclick: move |_| on_edit.call(e_edit.clone()),
                    "编辑"
                }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Xs,
                    class: "flex-1",
                    onclick: move |_| on_toggle.call(e_toggle.clone()),
                    if enabled { "停用" } else { "启用" }
                }
                Button {
                    variant: ButtonVariant::Destructive,
                    size: ButtonSize::Xs,
                    class: "flex-1",
                    onclick: move |_| on_delete.call(e_del.clone()),
                    "删除"
                }
            }
        }
    }
}

/// 新建密钥弹窗 — 提交走 POST /api/token。
/// 额度输入留空 = 不限额 (unlimited_quota)。
/// 成功后由父组件弹出一次性明文视图。
#[component]
fn NewKeyForm(
    name: Signal<String>,
    group: Signal<String>,
    quota: Signal<String>,
    on_cancel: EventHandler<()>,
    on_created: EventHandler<CreateTokenResult>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);

    let submit = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            err.set("密钥名称必填".into());
            return;
        }
        let g = group().trim().to_string();
        let q_raw = quota().trim().to_string();
        let (quota_v, unlimited) = if q_raw.is_empty() {
            (0, true)
        } else {
            (q_raw.parse::<i64>().unwrap_or(0), false)
        };
        let req = CreateTokenRequest {
            name: n,
            group: if g.is_empty() { None } else { Some(g) },
            quota: quota_v,
            unlimited_quota: unlimited,
            // 新建暂不设置过期 (MVP); 需要时由 CreateTokenRequest.expires_at 传入 RFC3339。
            expires_at: None,
        };

        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::create_token_api(&client, &req).await {
                Ok(res) => on_created.call(res),
                Err(e) => {
                    er.set(e.to_string());
                    b.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "新建 API 密钥" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_cancel.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "密钥名称" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            placeholder: "例如: 生产环境密钥",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "分组 (可选)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            placeholder: "留空 = 跟随用户默认分组",
                            value: "{group}",
                            oninput: move |e| group.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "额度限制 (额度单位)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 font-mono text-sm focus:border-zinc-500 focus:outline-none",
                            r#type: "text",
                            placeholder: "留空 = 不限额",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                    }

                    if !err().is_empty() {
                        p { class: "text-xs text-red-400", "{err()}" }
                    }
                }

                div { class: "mt-6 flex gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        class: "flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Primary,
                        class: "flex-1",
                        disabled: busy(),
                        onclick: submit,
                        "创建密钥"
                    }
                }
            }
        }
    }
}

/// 新建成功视图 — 一次性明文 key 展示 (后端只在创建响应返回一次)。
/// 手动复制 (只读输入框, 选中文案后复制)。
#[component]
fn CreatedKeyView(result: CreateTokenResult, on_close: EventHandler<()>) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            div {
                class: "w-full max-w-md rounded-2xl border border-emerald-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-4 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-emerald-400", "密钥创建成功" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                p { class: "mb-2 text-xs text-amber-400", "明文密钥只显示这一次,关闭后无法再查看" }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-3 font-mono text-sm text-emerald-300 focus:outline-none",
                    r#type: "text",
                    r#readonly: true,
                    value: "{result.plaintext}"
                }
                p { class: "mt-2 text-xs text-zinc-500", "名称: {result.token.name} — 选中上方内容后复制 (Ctrl/Cmd+C)" }

                div { class: "mt-5 flex justify-end",
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| on_close.call(()),
                        "完成"
                    }
                }
            }
        }
    }
}

/// 编辑密钥弹窗 — 走 PUT /api/token/{key}, 只改名称 (备注字段后端不存在)。
#[component]
fn EditKeyModal(
    token: TokenDto,
    on_cancel: EventHandler<()>,
    on_saved: EventHandler<()>,
) -> Element {
    let mut name = use_signal(|| token.name.clone());
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);

    let submit = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            err.set("名称不能为空".into());
            return;
        }
        let req = UpdateTokenRequest {
            name: Some(n),
            ..Default::default()
        };
        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let key_id = token.key.clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::update_token_api(&client, &key_id, &req).await {
                Ok(_) => on_saved.call(()),
                Err(e) => {
                    er.set(e.to_string());
                    b.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "编辑密钥" }
                    p { class: "truncate font-mono text-xs text-zinc-500", "{token.key_preview}" }
                }

                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "密钥名称" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    if !err().is_empty() {
                        p { class: "text-xs text-red-400", "{err()}" }
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
                        disabled: busy(),
                        onclick: submit,
                        "保存"
                    }
                }
            }
        }
    }
}

/// 删除确认弹窗 — 走 DELETE /api/token/{key}。
#[component]
fn DeleteKeyModal(
    token: TokenDto,
    on_cancel: EventHandler<()>,
    on_confirmed: EventHandler<()>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);

    let confirm = move |_| {
        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let key_id = token.key.clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::delete_token_api(&client, &key_id).await {
                Ok(_) => on_confirmed.call(()),
                Err(e) => {
                    er.set(e.to_string());
                    b.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-red-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-base font-semibold text-zinc-100", "删除密钥" }
                p { class: "mt-3 text-sm text-zinc-400",
                    "确认删除「{token.name}」({token.key_preview})？删除后使用该密钥的调用会立即失败, 且无法恢复。"
                }
                if !err().is_empty() {
                    p { class: "mt-3 text-xs text-red-400", "{err()}" }
                }

                div { class: "mt-6 flex gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        class: "flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Destructive,
                        class: "flex-1",
                        disabled: busy(),
                        onclick: confirm,
                        "确认删除"
                    }
                }
            }
        }
    }
}

/// 复制密钥预览的小图标按钮: 点击写入剪贴板, 成功后图标短暂变 ✓。
#[component]
fn CopyKeyButton(text: String) -> Element {
    let mut copied = use_signal(|| false);
    rsx! {
        Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconXs,
            title: "复制密钥",
            "aria-label": "复制密钥",
            class: if copied() { "text-emerald-400" } else { "" },
            onclick: move |_| {
                let ok = ui::copy_text_to_clipboard(text.as_str());
                copied.set(ok);
                let mut c = copied;
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(1500).await;
                    c.set(false);
                });
            },
            if copied() {
                span { class: "block h-3.5 w-3.5 text-center text-[11px] leading-[14px]", "✓" }
            } else {
                svg {
                    class: "h-3.5 w-3.5",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    stroke_width: "2",
                    rect { x: "9", y: "9", width: "13", height: "13", rx: "2" }
                    path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                }
            }
        }
    }
}
