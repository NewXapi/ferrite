//! tavern-ui — 酒馆公共组件。
//!
//! 对齐 SillyTavern 的交互件:头像、消息气泡(hover 操作栏)、swipe 选择器、
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

/// 幽灵图标按钮:消息 hover 操作栏和卡片浮层用。
#[component]
pub fn IconButton(title: &'static str, onclick: EventHandler<MouseEvent>, children: Element) -> Element {
    rsx! {
        button {
            class: "flex h-6 w-6 items-center justify-center rounded-md text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
            title: "{title}",
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

/// ST 式消息气泡:头像 + 名字/时间行 + 内容;hover 浮出操作栏(右上)。
/// `mine` 用户消息右对齐反色;角色消息左对齐带头像。
#[component]
pub fn MessageBubble(
    name: String,
    time: String,
    content: String,
    mine: bool,
    #[props(default)] avatar_src: Option<String>,
    #[props(default)] actions: Option<Element>,
) -> Element {
    let align = if mine { "flex-row-reverse" } else { "" };
    let name_align = if mine { "flex-row-reverse" } else { "" };
    let bubble = if mine {
        "bg-zinc-100 text-zinc-900"
    } else {
        "bg-zinc-800/80 text-zinc-100"
    };
    rsx! {
        div { class: "group flex gap-2.5 {align}",
            Avatar { name: name.clone(), src: avatar_src }
            div { class: "flex min-w-0 max-w-[75%] flex-col gap-1",
                div { class: "flex items-baseline gap-2 px-1 {name_align}",
                    span { class: "text-xs font-medium text-zinc-400", "{name}" }
                    span { class: "text-[10px] text-zinc-600", "{time}" }
                }
                div { class: "relative",
                    div { class: "whitespace-pre-wrap rounded-2xl px-3.5 py-2.5 text-sm leading-6 {bubble}",
                        "{content}"
                    }
                    if let Some(actions) = actions {
                        div { class: "absolute -top-3 right-2 flex items-center gap-0.5 rounded-lg border border-zinc-800 bg-zinc-900/95 px-1 py-0.5 opacity-0 shadow-lg shadow-black/30 transition-opacity group-hover:opacity-100",
                            {actions}
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
        div { class: "inline-flex items-center gap-1 text-xs text-zinc-500",
            button {
                class: "flex h-5 w-5 items-center justify-center rounded transition-colors hover:bg-zinc-800 hover:text-zinc-200 disabled:opacity-30",
                disabled: index == 0,
                onclick: move |_| on_prev.call(()),
                "‹"
            }
            span { class: "tabular-nums", "{index + 1}/{total}" }
            button {
                class: "flex h-5 w-5 items-center justify-center rounded transition-colors hover:bg-zinc-800 hover:text-zinc-200 disabled:opacity-30",
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
                class: "absolute inset-0 bg-black/50",
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
