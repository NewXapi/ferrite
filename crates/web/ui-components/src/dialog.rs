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
            div { class: "relative flex w-80 flex-col gap-4 rounded-2xl border {crate::T_border_zinc_800} {crate::T_bg_zinc_900} p-5 shadow-2xl shadow-black/50",
                span { class: "{crate::TYPE_CARD_TITLE}", "{title}" }
                div { class: "{crate::T_text_xs} leading-5 {crate::T_text_zinc_400}", {children} }
                div { class: "flex items-center justify-end gap-2",
                    button {
                        class: "rounded-full px-3 py-1.5 {crate::T_text_sm} {crate::T_text_zinc_400} transition-colors hover:{crate::T_bg_zinc_800} hover:{crate::T_text_zinc_100}",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    button {
                        class: "rounded-full {crate::T_bg_zinc_100} px-3 py-1.5 {crate::T_text_sm} {crate::T_font_medium} {crate::T_text_zinc_900} transition-colors hover:{crate::T_bg_zinc_300}",
                        onclick: move |_| on_confirm.call(()),
                        "确认"
                    }
                }
            }
        }
    }
}
