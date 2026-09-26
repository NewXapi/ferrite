use dioxus::prelude::*;

/// 空状态展示
#[component]
pub fn EmptyState(title: String, hint: String) -> Element {
    rsx! {
        div { class: "flex flex-1 flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-border py-16 text-center",
            span { class: "{crate::TYPE_CARD_TITLE}", "{title}" }
            span { class: "{crate::TYPE_DESC}", "{hint}" }
        }
    }
}
