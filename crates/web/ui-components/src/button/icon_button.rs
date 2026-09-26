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
            class: "flex h-6 w-6 items-center justify-center rounded-md {crate::C_MUTED} transition-colors hover:bg-zinc-800 hover:text-zinc-100",
            title: "{title}",
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}
