//! Separator — rust-ui (github.com/rust-ui/ui) registry 原版拷贝（shadcn copy-paste 层）。
//!
//! 与 crate 内同名自研组件并存，rust-ui 线走 `rui_` 前缀；仅按需去上游依赖。
use dioxus::prelude::*;
use tw_merge::tw_merge;

#[allow(dead_code)] // registry 原版: 变体按需使用, 未用的保留枚举完整性
#[derive(Default, Clone, PartialEq, Eq)]
pub enum SeparatorOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[component]
pub fn Separator(
    #[props(into, optional)] class: Option<String>,
    #[props(default = SeparatorOrientation::default())] orientation: SeparatorOrientation,
) -> Element {
    let orientation_class = match orientation {
        SeparatorOrientation::Horizontal => "w-full h-[1px]",
        SeparatorOrientation::Vertical => "h-full w-[1px]",
    };
    let merged_class = tw_merge!(
        "shrink-0 bg-border",
        orientation_class,
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div { class: "{merged_class}", role: "separator" }
    }
}
