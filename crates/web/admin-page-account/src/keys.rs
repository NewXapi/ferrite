use dioxus::prelude::*;

use contract::api::user::{role_label, UpdateSelfRequest, UserDto};

use crate::api::{self, ApiKey};

/// 密钥·资料面板 - 左侧 ScrollSpyNav + 内滚动三区 (统计 / 个人资料 / 我的密钥)
/// 遵循 admin/network.rs 参考用法，父容器 relative，内容区 pl-8 留位
#[component]
pub fn KeysPanel() -> Element {
    // —— 区段标题 (ScrollSpyNav + h2 同用) ——
    const SEC_STATS: &str = "个人数据";
    const SEC_PROFILE: &str = "个人资料";
    const SEC_KEYS: &str = "我的密钥";

    let mut show_new_form = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let new_group = use_signal(|| "default".to_string());
    let new_quota = use_signal(|| "5000000".to_string());

    // 编辑资料/改密弹窗
    let mut show_edit = use_signal(|| false);

    let stats = api::fetch_key_stats();
    let keys = api::fetch_keys();

    // ---- 真实用户信息 (GET /api/user/self) ----
    let mut self_user = use_signal(|| None::<UserDto>);
    let self_err = use_signal(String::new);
    use_hook(move || {
        let client = client::ApiClient::shared().clone();
        let mut su = self_user.clone();
        let mut se = self_err.clone();
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
    });

    rsx! {
            div { class: "flex flex-col gap-6",

                    // 1. 统计区
                    section {
                        id: "keys-sec-stats",
                        class: "scroll-mt-8 space-y-3",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                        // 宽度约定:总栅格 = 手机 1 栏 / 平板 3 栏 / Web 5 栏,所有卡片各占 1 栏。
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for &(value, label) in stats {
                                StatCard { value, label }
                            }
                        }
                    }

                    // 2. 个人资料区
                    section {
                        id: "keys-sec-profile",
                        class: "scroll-mt-8 space-y-3",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_PROFILE}" }
                        div { class: "grid grid-cols-1 gap-4 md:grid-cols-3 lg:grid-cols-5",
                            // 个人资料卡 (真实 /self 数据, 占 3 栏)
                            div { class: "md:col-span-2 xl:col-span-3 rounded-xl border border-zinc-800 bg-zinc-900/60 p-6",
                                div { class: "flex items-start justify-between gap-4",
                                    div { class: "min-w-0",
                                        if let Some(user) = self_user() {
                                            div { class: "mb-4 flex items-center gap-2",
                                                span { class: "truncate text-sm font-medium text-zinc-100", "{user.username}" }
                                                span { class: "shrink-0 rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] font-medium text-zinc-400", "{role_label(user.role)}" }
                                            }
                                            div { class: "grid grid-cols-1 gap-3 sm:grid-cols-2",
                                                ProfileRow { label: "显示名", value: user.display_name.clone() }
                                                ProfileRow { label: "邮箱", value: user.email.clone() }
                                                ProfileRow { label: "用户ID", value: user.key.clone() }
                                                ProfileRow { label: "分组", value: user.group.clone() }
                                                ProfileRow { label: "注册时间", value: user.created_at.clone() }
                                            }
                                        } else if !self_err().is_empty() {
                                            p { class: "text-sm text-amber-400", "无法加载用户信息 (未登录或请求失败): {self_err()}" }
                                        } else {
                                            p { class: "text-sm text-zinc-500", "加载中…" }
                                        }
                                    }
                                    button {
                                        class: "shrink-0 rounded-lg border border-zinc-700 bg-zinc-800/60 px-3 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                                        onclick: move |_| show_edit.set(true),
                                        "编辑资料 / 改密"
                                    }
                                }
                            }

                            // 快捷操作卡 (新建按钮)
                            div { class: "md:col-span-1 xl:col-span-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-6 self-start",
                                div {
                                    h3 { class: "text-lg font-medium text-zinc-100", "快捷操作" }
                                    p { class: "mt-2 text-sm text-zinc-500", "为新的应用或环境签发一个独立密钥,可随时停用" }
                                }
                                button {
                                    class: "mt-4 w-full py-3 rounded-xl bg-white text-zinc-900 font-medium hover:bg-zinc-100 active:bg-zinc-200 transition-colors flex items-center justify-center gap-2",
                                    onclick: move |_| show_new_form.set(!show_new_form()),
                                    "✚ 新建密钥"
                                }
                            }
                        }
                    }

                    // 3. 我的密钥区
                    section {
                        id: "keys-sec-keys",
                        class: "scroll-mt-8",
                        div { class: "space-y-4",
                            div { class: "flex items-center justify-between",
                                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_KEYS}" }
                                span { class: "text-xs px-3 py-1 rounded-full bg-zinc-800 text-zinc-400", "{keys.len()} 个" }
                            }

                            // 密钥卡片网格:卡片各占 1 栏 → 手机 1 张/排,平板 3 张/排,Web 5 张/排。
                            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                for key in keys {
                                    KeyCard { entry: key }
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
                    on_submit: move || {
                        new_name.set(String::new());
                        show_new_form.set(false);
                    }
                }
            }

            // 编辑资料 / 改密弹窗
            if show_edit() {
                SelfEditModal {
                    user: self_user().unwrap_or_default(),
                    on_cancel: move || show_edit.set(false),
                    on_saved: move |u: UserDto| {
                        self_user.set(Some(u));
                        show_edit.set(false);
                    }
                }
            }
    }
}

#[component]
fn StatCard(value: &'static str, label: &'static str) -> Element {
    rsx! {
        div {
            class: "rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 transition-colors hover:border-zinc-600",
            p { class: "text-xl font-semibold tracking-tight text-white", "{value}" }
            p { class: "mt-0.5 text-xs text-zinc-500", "{label}" }
        }
    }
}

#[component]
fn ProfileRow(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex flex-col gap-0.5 text-sm sm:flex-row sm:gap-4",
            span { class: "shrink-0 text-zinc-400 sm:w-16", "{label}" }
            span { class: "min-w-0 break-all font-mono text-zinc-200", "{value}" }
        }
    }
}

/// 编辑资料 / 改密弹窗 — 走 PUT /api/user/self。
/// 改密须原密码 + 新密码成对；成功后刷新共享缓存 (改密会使全端登出)。
#[component]
fn SelfEditModal(
    user: UserDto,
    on_cancel: EventHandler<()>,
    on_saved: EventHandler<UserDto>,
) -> Element {
    let mut name = use_signal(|| user.display_name.clone());
    let mut old_pwd = use_signal(String::new);
    let mut new_pwd = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);

    let submit = move |_| {
        let n = name().trim().to_string();
        let op = old_pwd().clone();
        let np = new_pwd().clone();

        let change_pwd = !op.is_empty() || !np.is_empty();
        if change_pwd && (op.is_empty() || np.is_empty()) {
            err.set(Some("原密码与新密码需同时填写".into()));
            return;
        }

        let req = UpdateSelfRequest {
            display_name: if n.is_empty() { None } else { Some(n) },
            original_password: if change_pwd { Some(op) } else { None },
            new_password: if change_pwd { Some(np) } else { None },
        };

        busy.set(true);
        err.set(None);
        let client = client::ApiClient::shared().clone();
        let on_saved = on_saved.clone();
        let mut b = busy.clone();
        let mut er = err.clone();
        spawn(async move {
            match api::update_self_api(&client, &req).await {
                Ok(u) => {
                    if let Ok(s) = serde_json::to_string(&u) {
                        ui::set_storage_item("ferrite_current_user", &s);
                    }
                    on_saved.call(u);
                }
                Err(e) => {
                    er.set(Some(e.to_string()));
                }
            }
            b.set(false);
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
                    h3 { class: "text-base font-semibold text-zinc-100", "编辑资料 / 修改密码" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_cancel.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "显示名 (留空不改)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }

                    div { class: "border-t border-zinc-800 pt-4",
                        p { class: "mb-3 text-xs text-zinc-500", "修改密码 (两项都填才生效；改密后所有设备需重新登录)" }
                        div { class: "space-y-3",
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                                r#type: "password",
                                placeholder: "原密码",
                                value: "{old_pwd}",
                                oninput: move |e| old_pwd.set(e.value()),
                            }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                                r#type: "password",
                                placeholder: "新密码 (8-128 位)",
                                value: "{new_pwd}",
                                oninput: move |e| new_pwd.set(e.value()),
                            }
                        }
                    }

                    if let Some(m) = err() {
                        p { class: "text-xs text-red-400", "{m}" }
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

#[component]
fn KeyCard(entry: &'static ApiKey) -> Element {
    let masked = if entry.key.len() > 10 {
        format!("sk-•••{}", &entry.key[entry.key.len() - 4..])
    } else {
        entry.key.to_string()
    };

    let status_color = if entry.status == "启用" {
        "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-amber-500/20 text-amber-400 border-amber-500/30"
    };

    rsx! {
        div {
            class: "group rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "mb-3 flex items-start justify-between gap-2",
                div { class: "min-w-0",
                    h3 { class: "text-sm font-medium text-zinc-100", "{entry.name}" }
                    p { class: "mt-0.5 truncate font-mono text-[11px] text-zinc-500", "{masked}" }
                }
                span {
                    class: "rounded-full border px-2.5 py-0.5 text-xs font-medium {status_color}",
                    "{entry.status}"
                }
            }

            div { class: "space-y-2 text-xs",
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "本月用量" }
                    span { class: "whitespace-nowrap font-medium text-zinc-200", "{entry.usage}" }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "创建时间" }
                    span { class: "whitespace-nowrap font-mono text-zinc-400", "{entry.created}" }
                }
            }

            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300",
                    "停用"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-zinc-700 hover:text-red-300",
                    "删除"
                }
            }
        }
    }
}

#[component]
fn NewKeyForm(
    name: Signal<String>,
    group: Signal<String>,
    quota: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    rsx! {
        // 遮罩:点击空白处关闭
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            // 弹窗本体:手机近全宽,桌面限宽居中
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "新建 API 密钥" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_cancel.call(()),
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
                        label { class: "mb-1.5 block text-xs text-zinc-400", "分组" }
                        select {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            onchange: move |e| group.set(e.value()),
                            option { value: "default", "默认" }
                            option { value: "prod", "生产环境" }
                            option { value: "test", "测试环境" }
                            option { value: "mobile", "移动端" }
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "额度 (tokens)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 font-mono text-sm focus:border-zinc-500 focus:outline-none",
                            r#type: "text",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                        p { class: "mt-1 text-xs text-zinc-500", "留空则不限制" }
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
                        "创建密钥"
                    }
                }
            }
        }
    }
}
