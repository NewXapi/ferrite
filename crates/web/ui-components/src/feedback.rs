use dioxus::prelude::*;

/// 圆形头像: 有图显示图，无图显示首字符
#[component]
pub fn Avatar(
    name: String,
    #[props(default)] src: Option<String>,
    #[props(default = "h-9 w-9".to_string())] size: String,
) -> Element {
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

/// 幽灵图标按钮: 动作条和卡片浮层用
#[component]
pub fn IconButton(
    title: &'static str,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: "flex h-6 w-6 items-center justify-center rounded-md text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
            title: "{title}",
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

/// 空状态展示
#[component]
pub fn EmptyState(title: String, hint: String) -> Element {
    rsx! {
        div { class: "flex flex-1 flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-zinc-800 py-16 text-center",
            span { class: "text-sm font-medium text-zinc-300", "{title}" }
            span { class: "text-xs text-zinc-500", "{hint}" }
        }
    }
}

/// 加载中转圈
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
