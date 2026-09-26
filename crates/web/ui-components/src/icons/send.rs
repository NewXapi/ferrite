use dioxus::prelude::*;

/// 纸飞机：发送消息（聊天输入区提交按钮）。
///
/// `size` 为像素尺寸（默认 16），`class` 为附加样式类。
#[component]
pub fn IconSend(
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
            path { d: "m22 2-7 20-4-9-9-4Z" }
            path { d: "M22 2 11 13" }
        }
    }
}
