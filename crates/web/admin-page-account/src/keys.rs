use dioxus::prelude::*;
use ui::ScrollSpyNav;

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

    let stats = api::fetch_key_stats();
    let profile = api::fetch_profile();
    let keys = api::fetch_keys();

    rsx! {
        div {
            class: "pl-8",

            ScrollSpyNav {
                container: "panel-scroll",
                items: vec![
                    ({SEC_STATS}.to_string(), "keys-sec-stats".to_string()),
                    ({SEC_PROFILE}.to_string(), "keys-sec-profile".to_string()),
                    ({SEC_KEYS}.to_string(), "keys-sec-keys".to_string()),
                ],
            }

            div { class: "flex flex-col gap-6",

                    // 1. 统计区
                    section {
                        id: "keys-sec-stats",
                        class: "scroll-mt-8 space-y-3",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                        // 宽度约定:总栅格 = 手机 1 栏 / 平板 3 栏 / Web 5 栏,所有卡片各占 1 栏。
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
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
                        div { class: "grid grid-cols-1 gap-4 md:grid-cols-3 xl:grid-cols-5",
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
                        div { class: "space-y-4",
                            div { class: "flex items-center justify-between",
                                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_KEYS}" }
                                span { class: "text-xs px-3 py-1 rounded-full bg-zinc-800 text-zinc-400", "{keys.len()} 个" }
                            }

                            // 密钥卡片网格:卡片各占 1 栏 → 手机 1 张/排,平板 3 张/排,Web 5 张/排。
                            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
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
                    class: "flex-1 rounded-lg border border-zinc-700 py-1 text-[11px] text-zinc-300 transition-colors hover:bg-zinc-800",
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700 py-1 text-[11px] text-amber-400 transition-colors hover:bg-zinc-800",
                    "停用"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700 py-1 text-[11px] text-zinc-400 transition-colors hover:bg-red-950 hover:text-red-400",
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
