use dioxus::prelude::*;
use crate::feedback::Avatar;

/// 消息气泡组件 (点击气泡弹出快捷操作栏，支持左右身份对齐)
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

/// 分支切换器 (‹ 1/2 ›)
#[component]
pub fn SwipePicker(
    index: usize,
    total: usize,
    on_prev: EventHandler<()>,
    on_next: EventHandler<()>,
) -> Element {
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
