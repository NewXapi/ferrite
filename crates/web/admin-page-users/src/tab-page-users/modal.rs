//! 弹窗外壳:遮罩 + 居中卡 + 标题栏关闭按钮。
//!
//! 新建/编辑弹窗与充值弹窗共用同一外壳,只有内容不同。点遮罩关闭,点内容区
//! 需 `stop_propagation` 阻断冒泡,否则编辑到一半点输入框也会关掉弹窗。

use dioxus::prelude::*;

/// 弹窗内输入框的统一样式(表单弹窗与充值弹窗共用)。
pub const MODAL_INPUT: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none";

/// 弹窗外壳:遮罩 + 居中卡 + 标题栏关闭按钮
#[component]
pub fn Modal(title: String, on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "{title}" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
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
                {children}
            }
        }
    }
}
