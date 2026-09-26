use dioxus::prelude::*;

/// 柱状图：总览/仪表盘导航入口。
///
/// `size` 为像素尺寸（默认 16），`class` 为附加样式类。
#[component]
pub fn IconChartBar(
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
            path { d: "M3 9v7m0 0h7m-7 0v3m7-3V9a3 3 0 0 1 6 0v7m-6 0h6" }
            path { d: "M12 12h.01" }
        }
    }
}
