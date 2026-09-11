use dioxus::prelude::*;

/// 圆形头像: 有图显示图，无图显示首字符
///
/// shadcn 对齐迁移后委托 [`crate::components::avatar::Avatar`]（Root span +
/// avatar-image / avatar-fallback 解剖）。兼容策略：旧默认 `"h-9 w-9"`（36px）
/// 与 shadcn 默认 `"size-8"`（32px）不同档，映射到等价档 `"size-9"`（36px）保
/// 页面视觉零变化；调用方显式传入的 `size` 原样透传。
#[component]
pub fn Avatar(
    name: String,
    #[props(default)] src: Option<String>,
    #[props(default = "h-9 w-9".to_string())] size: String,
) -> Element {
    // 旧默认档（36px）映射到 shadcn 尺寸体系的等价档 size-9；
    // 其余显式值（含旧式 h-N w-N）原样透传，尺寸语义由新组件的 class 拼接保持。
    let size = if size == "h-9 w-9" {
        Some("size-9".to_string())
    } else {
        Some(size)
    };
    rsx! {
        crate::components::avatar::Avatar { name: name, src: src, size: size }
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
