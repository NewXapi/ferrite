use dioxus::prelude::*;

/// 加载中转圈
#[component]
pub fn Loading(label: Option<String>) -> Element {
    rsx! {
        div { class: "flex flex-1 items-center justify-center gap-2 py-16 {crate::C_MUTED}",
            div { class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-200" }
            if let Some(label) = label {
                span { class: "text-xs", "{label}" }
            }
        }
    }
}
