use dioxus::prelude::*;

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
