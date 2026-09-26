//! Progress — rust-ui (github.com/rust-ui/ui) registry 原版拷贝（shadcn copy-paste 层）。
//!
//! 与 crate 内同名自研组件并存，rust-ui 线走 `rui_` 前缀；仅按需去上游依赖。
use dioxus::prelude::*;
use tw_merge::tw_merge;

/// 拷贝自上游 progress.rs，追加 `bar_class`：KeyCard 用量条需要随用量换
/// emerald/amber/red 三档语义色，轨道色单靠 class 只能改外框，改不了内条。
#[component]
pub fn Progress(
    #[props(default = 0.0)] value: f64,
    #[props(default = 100.0)] max: f64,
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] bar_class: Option<String>,
) -> Element {
    let pct = (value / max * 100.0).clamp(0.0, 100.0);
    let style = format!("transform: translateX(-{}%)", 100.0 - pct);

    let merged = tw_merge!(
        "relative h-2 w-full overflow-hidden rounded-full bg-secondary",
        class.as_deref().unwrap_or("")
    );
    let bar = tw_merge!(
        "flex-1 w-full h-full transition-all duration-300 ease-in-out bg-primary",
        bar_class.as_deref().unwrap_or("")
    );

    rsx! {
        div {
            "data-name": "Progress",
            role: "progressbar",
            aria_valuemin: "0",
            aria_valuemax: "{max}",
            aria_valuenow: "{value}",
            class: "{merged}",
            div {
                class: "{bar}",
                style: "{style}",
            }
        }
    }
}
