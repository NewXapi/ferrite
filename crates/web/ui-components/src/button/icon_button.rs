use dioxus::prelude::*;

/// 幽灵图标按钮: 动作条和卡片浮层用
#[component]
pub fn IconButton(
    title: &'static str,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: "flex h-6 w-6 items-center justify-center rounded-md {crate::T_text_zinc_400} transition-colors hover:{crate::T_bg_zinc_800} hover:{crate::T_text_zinc_100}",
            title: "{title}",
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}
