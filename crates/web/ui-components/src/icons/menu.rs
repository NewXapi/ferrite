use dioxus::prelude::*;

/// 三横线汉堡菜单：移动端/窄屏的导航抽屉入口。
///
/// `size` 为像素尺寸（默认 16），`class` 为附加样式类。
#[component]
pub fn IconMenu(
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
            line { x1: "4", x2: "20", y1: "6", y2: "6" }
            line { x1: "4", x2: "20", y1: "12", y2: "12" }
            line { x1: "4", x2: "20", y1: "18", y2: "18" }
        }
    }
}
