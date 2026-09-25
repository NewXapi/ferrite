use dioxus::prelude::*;

/// 确认弹窗 (遮罩 + 居中浮层卡 + 取消/确认)
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
