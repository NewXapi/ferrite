//! Skeleton — rust-ui (github.com/rust-ui/ui) registry 原版拷贝（shadcn copy-paste 层）。
//!
//! 与 crate 内同名自研组件并存，rust-ui 线走 `rui_` 前缀；仅按需去上游依赖。
use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Skeleton(#[props(into, optional)] class: Option<String>) -> Element {
    let merged_class = tw_merge!(
        "animate-pulse rounded-md bg-muted",
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div { class: "{merged_class}" }
    }
}
