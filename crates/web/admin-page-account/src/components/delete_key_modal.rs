//! 删除确认弹窗 — 走 DELETE /api/token/{key}。

use contract::api::token::TokenDto;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonVariant};

use crate::api;

/// 【是什么】删除确认弹窗，含密钥信息展示和确认/取消按钮。
///
/// 【做什么】负责渲染删除密钥的确认界面：显示密钥名称 + 预览 + 确认语义，按钮含危险色 styling。确认后走 DELETE /api/token/{key}，成功则 on_confirmed，失败则显示错误。
///
/// 【交互逻辑】
/// - 点击「取消」/遮罩：调用 on_cancel，关闭弹窗。
/// - 点击「确认删除」：提交 DELETE 请求，成功则 on_confirmed.call(())
///   - 失败则更新 err Signal 显示错误信息，并重置 busy 状态。
///
/// 【样式】固定最大宽度 max-w-md，圆角边框、危险主题色 (border-red-500/40 bg-zinc-900)，
/// 按钮区等宽排列，删除按钮使用 Destructive variant。
///
/// 【子组件组成】ui::components::button::Button × 2 (取消/确认删除)
///
/// 【数据流】
/// - 对内（入）：token (TokenDto) 含密钥名称 + 预览；on_cancel/on_confirmed EventHandler 由页面传入。
/// - 对外（出）：确认成功时 on_confirmed.call(())，页面刷新列表；失败时仅更新 err 状态。
#[component]
pub fn DeleteKeyModal(
    token: TokenDto,
    on_cancel: EventHandler<()>,
    on_confirmed: EventHandler<()>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);

    let confirm = move |_| {
        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let key_id = token.key.clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::delete_token_api(&client, &key_id).await {
                Ok(_) => on_confirmed.call(()),
                Err(e) => {
                    er.set(e.to_string());
                    b.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "{ui::MODAL_BACKDROP}",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-red-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-base font-semibold text-zinc-100", "删除密钥" }
                p { class: "mt-3 text-sm text-zinc-400",
                    "确认删除「{token.name}」({token.key_preview})？删除后使用该密钥的调用会立即失败, 且无法恢复。"
                }
                if !err().is_empty() {
                    p { class: "mt-3 text-xs text-red-400", "{err()}" }
                }

                div { class: "mt-6 flex gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        class: "flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Destructive,
                        class: "flex-1",
                        disabled: busy(),
                        onclick: confirm,
                        "确认删除"
                    }
                }
            }
        }
    }
}
