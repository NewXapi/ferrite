use dioxus::prelude::*;

/// 空状态展示
#[component]
pub fn EmptyState(title: String, hint: String) -> Element {
    rsx! {
        div { class: "flex flex-1 flex-col items-center justify-center gap-2 rounded-xl border border-dashed {crate::T_border_zinc_800} py-16 text-center",
            span { class: "{crate::T_text_sm} {crate::T_font_medium} {crate::T_text_zinc_300}", "{title}" }
            span { class: "{crate::TYPE_DESC}", "{hint}" }
        }
    }
}
