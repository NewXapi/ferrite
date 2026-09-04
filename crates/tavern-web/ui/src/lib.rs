//! tavern-ui — 酒馆公共组件。
//!
//! 对齐 SillyTavern 的交互件:头像、消息气泡(点击菜单)、swipe 选择器、
//! 确认弹窗、滑杆字段、表单字段、空状态。页面 crate 统一从这里取件。

use dioxus::prelude::*;

/// 圆形头像:有图显示图,无图显示首字符。
#[component]
pub fn Avatar(name: String, #[props(default)] src: Option<String>, #[props(default = "h-9 w-9".to_string())] size: String) -> Element {
    rsx! {
        if let Some(src) = src {
            img {
                class: "{size} shrink-0 rounded-full border border-zinc-700 object-cover",
                src: "{src}",
                alt: "{name}",
            }
        } else {
            div { class: "{size} flex shrink-0 items-center justify-center rounded-full bg-zinc-800 text-sm font-semibold text-zinc-300",
                "{name.chars().next().unwrap_or('?')}"
            }
        }
    }
}

/// 幽灵图标按钮:消息操作栏和卡片浮层用。
#[component]
pub fn IconButton(title: &'static str, onclick: EventHandler<MouseEvent>, children: Element) -> Element {
    rsx! {
        button {
            class: "flex h-6 w-6 items-center justify-center rounded-md text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
            title: "{title}",
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

/// 升级款消息气泡:
/// - 点击气泡弹出操作菜单 (对齐用户需求5)
/// - 复制、编辑、分支切换、删除操作
/// - `mine` 用户消息右对齐反色; 角色消息左对齐带头像
#[component]
pub fn MessageBubble(
    name: String,
    time: String,
    content: String,
    mine: bool,
    #[props(default)] avatar_src: Option<String>,
    #[props(default)] actions: Option<Element>,
    #[props(default)] on_click: Option<EventHandler<MouseEvent>>,
    #[props(default = false)] is_active_menu: bool,
) -> Element {
    let align = if mine { "flex-row-reverse" } else { "" };
    let name_align = if mine { "flex-row-reverse" } else { "" };
    let bubble = if mine {
        "bg-zinc-100 text-zinc-900 cursor-pointer selection:bg-purple-400 selection:text-zinc-950"
    } else {
        "bg-zinc-800/90 text-zinc-100 cursor-pointer selection:bg-purple-900 selection:text-white"
    };

    rsx! {
        div { class: "relative flex gap-2.5 {align}",
            Avatar { name: name.clone(), src: avatar_src }
            div { class: "flex min-w-0 max-w-[80%] sm:max-w-[75%] flex-col gap-1",
                div { class: "flex items-baseline gap-2 px-1 {name_align}",
                    span { class: "text-xs font-medium text-zinc-400", "{name}" }
                    span { class: "text-[10px] text-zinc-600", "{time}" }
                }
                div {
                    class: "relative group",
                    div {
                        class: "whitespace-pre-wrap rounded-2xl px-3.5 py-2.5 text-sm leading-6 {bubble} transition-all duration-200 hover:ring-1 hover:ring-purple-500/50 active:scale-[0.99]",
                        onclick: move |e| {
                            if let Some(cb) = &on_click {
                                cb.call(e);
                            }
                        },
                        "{content}"
                    }

                    // 点击气泡后弹出的精致悬浮操作条 (对齐图2需求)
                    if is_active_menu {
                        if let Some(actions) = actions {
                            div {
                                class: "absolute -top-10 right-0 z-30 flex items-center gap-1 rounded-xl border border-purple-500/40 bg-zinc-950/95 px-2 py-1 shadow-2xl shadow-black/80 backdrop-blur-xl animate-in fade-in zoom-in-95 duration-150",
                                onclick: move |e| e.stop_propagation(),
                                {actions}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// ST swipe 选择器:‹ 2/3 ›。
#[component]
pub fn SwipePicker(index: usize, total: usize, on_prev: EventHandler<()>, on_next: EventHandler<()>) -> Element {
    if total <= 1 {
        return rsx! {};
    }
    rsx! {
        div { class: "inline-flex items-center gap-1 text-xs text-zinc-400 font-medium px-1",
            button {
                class: "flex h-5 w-5 items-center justify-center rounded hover:bg-zinc-800 hover:text-zinc-100 disabled:opacity-30",
                disabled: index == 0,
                onclick: move |_| on_prev.call(()),
                "‹"
            }
            span { class: "tabular-nums text-[11px]", "{index + 1}/{total}" }
            button {
                class: "flex h-5 w-5 items-center justify-center rounded hover:bg-zinc-800 hover:text-zinc-100 disabled:opacity-30",
                disabled: index + 1 >= total,
                onclick: move |_| on_next.call(()),
                "›"
            }
        }
    }
}

/// 确认弹窗,对齐 ST `dialogue_popup`:遮罩 + 居中卡 + 取消/确认。
#[component]
pub fn Dialog(
    title: String,
    open: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
    children: Element,
) -> Element {
    if !open {
        return rsx! {};
    }
    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center",
            div {
                class: "absolute inset-0 bg-black/60 backdrop-blur-sm",
                onclick: move |_| on_cancel.call(()),
                "aria-hidden": "true",
            }
            div { class: "relative flex w-80 flex-col gap-4 rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-2xl shadow-black/50",
                span { class: "text-sm font-medium text-zinc-100", "{title}" }
                div { class: "text-xs leading-5 text-zinc-400", {children} }
                div { class: "flex items-center justify-end gap-2",
                    button {
                        class: "rounded-full px-3 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    button {
                        class: "rounded-full bg-zinc-100 px-3 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300",
                        onclick: move |_| on_confirm.call(()),
                        "确认"
                    }
                }
            }
        }
    }
}

/// 表单字段:小标签 + 控件。
#[component]
pub fn Field(label: &'static str, children: Element) -> Element {
    rsx! {
        label { class: "flex flex-col gap-1.5",
            span { class: "text-xs font-medium text-zinc-400", "{label}" }
            {children}
        }
    }
}

/// 滑杆字段,对齐 ST 采样抽屉:标签 + range + 数值。
#[component]
pub fn SliderField(
    label: &'static str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    on_change: EventHandler<f64>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-3",
            span { class: "w-28 shrink-0 text-xs font-medium text-zinc-400", "{label}" }
            input {
                r#type: "range",
                class: "h-1 flex-1 accent-zinc-100",
                min: "{min}",
                max: "{max}",
                step: "{step}",
                value: "{value}",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse() {
                        on_change.call(v);
                    }
                },
            }
            span { class: "w-12 shrink-0 text-right text-xs tabular-nums text-zinc-300", "{value:.2}" }
        }
    }
}

/// 空状态:居中标题 + 提示。
#[component]
pub fn EmptyState(title: String, hint: String) -> Element {
    rsx! {
        div { class: "flex flex-1 flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-zinc-800 py-16 text-center",
            span { class: "text-sm font-medium text-zinc-300", "{title}" }
            span { class: "text-xs text-zinc-500", "{hint}" }
        }
    }
}

/// 加载中转圈。
#[component]
pub fn Loading(label: Option<String>) -> Element {
    rsx! {
        div { class: "flex flex-1 items-center justify-center gap-2 py-16 text-zinc-500",
            div { class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-200" }
            if let Some(label) = label {
                span { class: "text-xs", "{label}" }
            }
        }
    }
}

/// 图2风格: 文游状态/备忘录卡片 (如任务系统、档案、备忘录)
#[component]
pub fn StatusCard(
    title: String,
    #[props(default = "emerald".to_string())] color: String,
    children: Element,
) -> Element {
    let border_color = match color.as_str() {
        "purple" => "border-purple-500/40 bg-purple-950/20 text-purple-200",
        "amber" => "border-amber-500/40 bg-amber-950/20 text-amber-200",
        "rose" => "border-rose-500/40 bg-rose-950/20 text-rose-200",
        "cyan" => "border-cyan-500/40 bg-cyan-950/20 text-cyan-200",
        _ => "border-emerald-500/40 bg-emerald-950/20 text-emerald-200",
    };
    let badge_color = match color.as_str() {
        "purple" => "bg-purple-500/20 text-purple-300 border-purple-500/30",
        "amber" => "bg-amber-500/20 text-amber-300 border-amber-500/30",
        "rose" => "bg-rose-500/20 text-rose-300 border-rose-500/30",
        "cyan" => "bg-cyan-500/20 text-cyan-300 border-cyan-500/30",
        _ => "bg-emerald-500/20 text-emerald-300 border-emerald-500/30",
    };
    rsx! {
        div { class: "flex flex-col gap-2 rounded-2xl border p-3.5 shadow-md {border_color}",
            div { class: "flex items-center gap-2",
                span { class: "rounded-lg border px-2 py-0.5 text-xs font-semibold {badge_color}",
                    "{title}"
                }
            }
            div { class: "text-xs leading-5 text-zinc-300",
                {children}
            }
        }
    }
}

/// 玩家决策单项
#[derive(Clone, PartialEq)]
pub struct ChoiceOption {
    pub key: String,
    pub text: String,
}

/// 图2核心: 玩家决策 (行动选项) 卡片
#[component]
pub fn ChoiceCard(
    title: String,
    options: Vec<ChoiceOption>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "flex flex-col overflow-hidden rounded-2xl border border-purple-500/40 bg-zinc-900/90 shadow-xl shadow-purple-950/20",
            div { class: "flex items-center justify-between border-b border-purple-500/30 bg-purple-950/60 px-4 py-2.5",
                div { class: "flex items-center gap-2",
                    span { class: "text-sm", "🎮" }
                    span { class: "text-xs font-semibold tracking-wide text-purple-200", "{title}" }
                }
                span { class: "text-[10px] text-purple-300/60", "点击选择分支行动" }
            }
            div { class: "flex flex-col divide-y divide-zinc-800/60 p-1.5",
                for opt in options {
                    {
                        let opt_text = opt.text.clone();
                        let opt_key = opt.key.clone();
                        rsx! {
                            button {
                                key: "{opt.key}",
                                class: "group flex items-start gap-3 rounded-xl p-2.5 text-left transition-colors hover:bg-purple-900/20",
                                onclick: move |_| on_select.call(opt_text.clone()),
                                span { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded-md bg-purple-900/60 text-xs font-bold text-purple-300 group-hover:bg-purple-600 group-hover:text-white transition-colors",
                                    "{opt_key}"
                                }
                                span { class: "text-xs leading-5 text-zinc-300 group-hover:text-zinc-100 transition-colors",
                                    "{opt.text}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
