use dioxus::prelude::*;

/// 容器型字段包装器 (包裹任意子控件 input / textarea)
#[component]
pub fn Field(label: &'static str, children: Element) -> Element {
    rsx! {
        label { class: "block space-y-1.5",
            span { class: "block {crate::T_text_xs} {crate::T_font_medium} {crate::T_text_zinc_400} uppercase tracking-wide", "{label}" }
            {children}
        }
    }
}
