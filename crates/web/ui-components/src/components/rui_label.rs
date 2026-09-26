//! Label — rust-ui (github.com/rust-ui/ui) registry 原版拷贝（shadcn copy-paste 层）。
//!
//! 与 crate 内同名自研组件并存，rust-ui 线走 `rui_` 前缀；仅按需去上游依赖。
use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Label(
    #[props(into, default)] html_for: Option<String>,
    #[props(into, default)] class: Option<String>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "flex items-center gap-2 text-xs text-zinc-400 leading-none select-none",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        label {
            r#for: html_for.as_deref().unwrap_or(""),
            class: "{class}",
            {children}
        }
    }
}
