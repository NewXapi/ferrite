use dioxus::prelude::*;

/// 加号：新建对话。
///
/// `size` 为像素尺寸（默认 16），`class` 为附加样式类。
#[component]
pub fn IconPlus(
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
            path { d: "M5 12h14" }
            path { d: "M12 5v14" }
        }
    }
}
