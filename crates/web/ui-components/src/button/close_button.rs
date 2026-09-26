use dioxus::prelude::*;

use crate::panel::CLOSE_BTN;
/// 关闭按钮:圆角 X svg 图标按钮(弹窗/面板标题栏用)。
/// 全仓统一样式与可及性属性(`aria-label` 固定「关闭」),页面不自建同型按钮。
#[component]
pub fn CloseButton(on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: "{CLOSE_BTN}",
            onclick: move |_| on_click.call(()),
            "aria-label": "关闭",
            svg {
                class: "h-5 w-5",
                fill: "none",
                stroke: "currentColor",
                view_box: "0 0 24 24",
                stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
            }
        }
    }
}
