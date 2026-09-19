//! 吊销「当前设备」确认弹窗 — 视觉仿 keys.rs 的删除确认弹窗 (DeleteKeyModal)。
//! 只负责确认交互, 真正的 DELETE 由父面板在 on_confirmed 里执行;
//! 「吊销其他设备」不弹窗, 维持现状。

use dioxus::prelude::*;
use ui::components::button::{Button, ButtonVariant};

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
