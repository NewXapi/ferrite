//! 吊销「当前设备」确认弹窗 — 视觉仿 keys.rs 的删除确认弹窗 (DeleteKeyModal)。
//! 只负责确认交互, 真正的 DELETE 由父面板在 on_confirmed 里执行;
//! 「吊销其他设备」不弹窗, 维持现状。

use dioxus::prelude::*;
use ui::components::button::{Button, ButtonVariant};

/// 【是什么】吊销「当前设备」会话的二次确认弹窗。
///
/// 【做什么】居中渲染红边模态：标题 + 风险说明 (吊销后本机立即退出且不可恢复) + 取消/确认双按钮；不发请求，视觉对齐 keys 页的 DeleteKeyModal。
///
/// 【交互逻辑】点遮罩或「取消」触发 on_cancel；「确认吊销」触发 on_confirmed，真正的 DELETE 由父面板执行；点弹窗主体 stop_propagation 防误取消。
///
/// 【样式】遮罩 fixed inset-0 z-50 黑半透明 + backdrop-blur-sm；弹窗 max-w-md rounded-2xl 红边 (border-red-500/40) shadow-xl；按钮 Outline / Destructive 各占一半。
///
/// 【子组件组成】ui::components::button::Button × 2
///
/// 【数据流】无入参数据；输出 on_cancel / on_confirmed 两个 EventHandler<()。
#[component]
pub fn ConfirmRevokeCurrentModal(
    on_cancel: EventHandler<()>,
    on_confirmed: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            role: "dialog",
            "aria-modal": "true",
            "aria-label": "吊销当前设备确认",
            "data-testid": "confirm-revoke-current-modal",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-red-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-base font-semibold text-zinc-100", "吊销当前设备" }
                p { class: "mt-3 text-sm text-zinc-400",
                    "确认吊销当前设备的会话吗？吊销后本设备将立即退出登录, 且无法恢复。"
                }

                div { class: "mt-6 flex gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        class: "flex-1",
                        "data-testid": "cancel-revoke-current",
                        "aria-label": "取消吊销当前设备",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Destructive,
                        class: "flex-1",
                        "data-testid": "confirm-revoke-current",
                        "aria-label": "确认吊销当前设备",
                        onclick: move |_| on_confirmed.call(()),
                        "确认吊销"
                    }
                }
            }
        }
    }
}
