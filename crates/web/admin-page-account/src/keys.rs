use dioxus::prelude::*;

use client::ApiClient;
use contract::api::token::{CreateTokenRequest, TokenDto, UpdateTokenRequest};

use crate::api::{self, create_token_api, delete_token_api, list_tokens_api, update_token_api};

/// 密钥·资料面板 - 左侧 ScrollSpyNav + 内滚动三区 (统计 / 个人资料 / 我的密钥)
///
/// 数据来自真实后端:挂载时 `use_effect` 拉 `list_tokens_api`,写入
/// `tokens` signal;新建走 `create_token_api`,删除 / 改状态走 `delete_token_api`
/// / `update_token_api`。统计卡与个人资料卡仍用 mock 文案(纯展示,无对应端点)。
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

    // 真实数据 + 加载/错误态
    let mut tokens = use_signal(Vec::<TokenDto>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_tokens_api(&client).await {
                Ok(page) => {
                    tokens.set(page);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    // 统计卡 / 个人资料:mock 文案(纯展示,无对应端点),保持不变。
    let stats = api::fetch_key_stats();
    let profile = api::fetch_profile();

    // 写操作助手工厂:每个返回独立闭包,分别交给 KeyCard(删除 / 改状态)、
    // NewKeyForm(新建)。Signal 是 Copy;每次调用先复制一份再 move 进 async,
    // 避免把闭包捕获的 signal 移动出去(FnMut 不允许)。
    let make_create = || {
        let captured = (busy, notice, reload);
        move |name: String, group: String, quota: String| {
            let (mut b, mut n, mut r) = captured;
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                let req = CreateTokenRequest {
                    name,
                    group: if group.is_empty() { None } else { Some(group) },
                    quota: quota.trim().parse().unwrap_or(0),
                    unlimited_quota: false,
                    expires_at: None,
                };
                match create_token_api(&client, &req).await {
                    Ok(t) => {
                        n.set(Some(format!("密钥创建成功:{}", t.name)));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("创建失败:{e}"))),
                }
                b.set(false);
            });
        }
    };
    let create = make_create();

    let make_delete = || {
        let captured = (busy, notice, reload);
        move |key: String| {
            let (mut b, mut n, mut r) = captured;
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                match delete_token_api(&client, &key).await {
                    Ok(_) => {
                        n.set(Some("密钥已删除".to_string()));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("删除失败:{e}"))),
                }
                b.set(false);
            });
        }
    };
    let delete = make_delete();

    let make_toggle = || {
        let captured = (busy, notice, reload);
        move |key: String, status: u8| {
            let (mut b, mut n, mut r) = captured;
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                let req = UpdateTokenRequest {
                    name: None,
                    group: None,
                    quota: None,
                    unlimited_quota: None,
                    status: Some(status),
                    expires_at: None,
                };
                match update_token_api(&client, &key, &req).await {
                    Ok(_) => {
                        n.set(Some("状态已更新".to_string()));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("更新失败:{e}"))),
                }
                b.set(false);
            });
        }
    };
    let toggle = make_toggle();

    rsx! {
            div { class: "flex flex-col gap-6", "data-testid": "keys-panel",

                    // 通知条(成功/错误/进行中)
                    if let Some(msg) = notice() {
                        div { class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
                            "{msg}"
                            if busy() { " ···" }
                        }
                    }

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
                            // 个人资料卡 (占 3 栏)
                            div { class: "md:col-span-2 xl:col-span-3 rounded-xl border border-zinc-800 bg-zinc-900/60 p-6",
                                div { class: "flex items-start justify-between",
                                    div {
                                        div { class: "space-y-4",
                                            ProfileRow { label: "用户名", value: profile.username }
                                            ProfileRow { label: "邮箱", value: profile.email }
                                            ProfileRow { label: "用户ID", value: profile.user_id }
                                            ProfileRow { label: "注册时间", value: profile.registered_at }
                                        }
                                    }
                                    svg {
                                    class: "h-9 w-9 text-zinc-700",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    stroke_width: "1.5",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.5 20.25a7.5 7.5 0 0115 0" }
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
                        div { class: "flex items-center justify-between gap-3",
                            h2 { class: "text-lg font-medium text-zinc-100", "{SEC_KEYS}" }
                            span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                                if loading() { "加载中…" } else { "{tokens().len()} 个" }
                            }
                            button {
                                class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                                "data-testid": "refresh-keys",
                                onclick: move |_| reload.set(reload() + 1),
                                "刷新"
                            }
                        }

                        if let Some(e) = err() {
                            div { class: "mt-4 rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                                p { class: "text-sm text-red-300", "加载密钥失败" }
                                p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                                button {
                                    class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                    onclick: move |_| reload.set(reload() + 1),
                                    "重试"
                                }
                            }
                        } else if loading() {
                            div { class: "mt-4 rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                                p { class: "text-zinc-400", "正在加载密钥…" }
                            }
                        } else if tokens().is_empty() {
                            div { class: "mt-4 rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                                p { class: "text-zinc-400", "还没有任何密钥" }
                            }
                        } else {
                            // 密钥卡片网格:卡片各占 1 栏 → 手机 1 张/排,平板 3 张/排,Web 5 张/排。
                            div { class: "mt-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5", "data-testid": "keys-list",
                                for entry in tokens() {
                                    KeyCard {
                                        key: "{entry.key}",
                                        entry: entry.clone(),
                                        on_delete: move |k: String| delete(k),
                                        on_toggle: move |(k, s): (String, u8)| toggle(k, s),
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
                    on_submit: move |(name, group, quota): (String, String, String)| {
                        create(name, group, quota);
                        new_name.set(String::new());
                        show_new_form.set(false);
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
fn ProfileRow(label: &'static str, value: &'static str) -> Element {
    rsx! {
        div { class: "flex flex-col gap-0.5 text-sm sm:flex-row sm:gap-4",
            span { class: "shrink-0 text-zinc-400 sm:w-16", "{label}" }
            span { class: "min-w-0 break-all font-mono text-zinc-200", "{value}" }
        }
    }
}

#[component]
fn KeyCard(
    entry: TokenDto,
    on_delete: EventHandler<String>,
    on_toggle: EventHandler<(String, u8)>,
) -> Element {
    let enabled = entry.status == 1;

    let status_color = if enabled {
        "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-amber-500/20 text-amber-400 border-amber-500/30"
    };

    // 配额:无限则显示 ∞,否则 used / quota
    let quota_text = if entry.unlimited_quota {
        "∞".to_string()
    } else {
        format!("{} / {}", entry.used_quota, entry.quota)
    };

    // 三个回调各自持有 key 的副本(EventHandler 是 move 捕获,String 不可 Copy)。
    let del_key = entry.key.clone();
    let toggle_key = entry.key.clone();
    let toggle_to = if enabled { 0u8 } else { 1u8 };

    rsx! {
        div {
            class: "group rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            role: "listitem",
            "aria-label": "密钥 {entry.name}",
            "data-testid": "key-card",

            div { class: "mb-3 flex items-start justify-between gap-2",
                div { class: "min-w-0",
                    h3 { class: "text-sm font-medium text-zinc-100", "{entry.name}" }
                    p { class: "mt-0.5 truncate font-mono text-[11px] text-zinc-500", "{entry.masked_key}" }
                }
                span {
                    class: "rounded-full border px-2.5 py-0.5 text-xs font-medium {status_color}",
                    if enabled { "启用" } else { "停用" }
                }
            }

            div { class: "space-y-2 text-xs",
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "分组" }
                    span { class: "whitespace-nowrap font-medium text-zinc-200",
                        if let Some(g) = &entry.group { "{g}" } else { "默认" }
                    }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "用量" }
                    span { class: "whitespace-nowrap font-medium text-zinc-200", "{quota_text}" }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "过期" }
                    span { class: "whitespace-nowrap font-mono text-zinc-400",
                        if let Some(e) = &entry.expires_at { "{e}" } else { "—" }
                    }
                }
            }

            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300",
                    onclick: move |_| on_toggle.call((toggle_key.clone(), toggle_to)),
                    if enabled { "停用" } else { "启用" }
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-zinc-700 hover:text-red-300",
                    onclick: move |_| on_delete.call(del_key.clone()),
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
    on_submit: EventHandler<(String, String, String)>,
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
                            value: "{group}",
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
                        onclick: move |_| on_submit.call((name(), group(), quota())),
                        "创建密钥"
                    }
                }
            }
        }
    }
}
