use dioxus::prelude::*;

/// 外框 + 右下小方块：把浮出窗口停靠回侧栏（简化版 dock 图标）。
///
/// `size` 为像素尺寸（默认 16），`class` 为附加样式类。
#[component]
pub fn IconDock(
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
            rect { x: "3", y: "3", width: "18", height: "18", rx: "2" }
            rect { x: "13", y: "13", width: "8", height: "8", rx: "1" }
        }
    }
}
