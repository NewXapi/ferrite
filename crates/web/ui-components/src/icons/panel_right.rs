use dioxus::prelude::*;

/// 右分栏面板：右侧面板开/关切换（抽屉、详情栏等）。
///
/// `size` 为像素尺寸（默认 16），`class` 为附加样式类。
#[component]
pub fn IconPanelRight(
    /// 像素尺寸（默认 16）。
    #[props(default = 16)]
    size: u16,
    /// 附加 class（尺寸/颜色微调用）。
    #[props(default = String::new())]
    class: String,
) -> Element {
    rsx! {
        svg {
            "viewBox": "0 0 24 24",
            width: "{size}",
            height: "{size}",
            fill: "none",
            stroke: "currentColor",
            "stroke-width": "2",
            "stroke-linecap": "round",
            "stroke-linejoin": "round",
            class: "{class}",
            "aria-hidden": "true",
            rect { width: "18", height: "18", x: "3", y: "3", rx: "2" }
            path { d: "M15 3v18" }
        }
    }
}
