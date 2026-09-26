//! 删除确认弹窗 — 走 DELETE /api/token/{key}（rust-ui AlertDialog 重构）。

use contract::api::token::TokenDto;
use dioxus::prelude::*;

use ui::components::rui_alert::{Alert, AlertDescription, AlertVariant};
use ui::components::rui_alert_dialog::{
    AlertDialog, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogTitle,
};
use ui::components::rui_button::{Button, ButtonSize, ButtonVariant};

use crate::api;

/// 【是什么】删除确认弹窗，含密钥信息展示和确认/取消按钮。
///
/// 【做什么】负责渲染删除密钥的确认界面：显示密钥名称 + 预览 + 确认语义，确认按钮危险色。确认后走 DELETE /api/token/{key}，成功则 on_confirmed，失败则以 Destructive Alert 显示错误。
///
/// 【交互逻辑】
/// - 点击「取消」/遮罩/X：调用 on_cancel，关闭弹窗（弹窗条件挂载，父层卸载；AlertDialog 语义不点遮罩关闭，取消/X 生效）。
/// - 点击「确认删除」：提交 DELETE 请求，成功则 on_confirmed.call(())
///   - 失败则更新 err Signal 显示错误信息，并重置 busy 状态。
///
/// 【样式】rust-ui AlertDialog 承载（max-w-md，红边危险主题）；按钮 Outline/Destructive 等宽。
///
/// 【子组件组成】rui AlertDialog 族 + Alert + Button ×2
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

    let confirm = move |_: MouseEvent| {
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
        AlertDialog { class: "w-full max-w-md",
            // 条件挂载弹窗: open 恒 true; AlertDialog 语义关遮罩点击, 经 on_close 走 on_cancel 卸载
            AlertDialogContent { open: true, on_close: move |_| on_cancel.call(()),
                AlertDialogTitle { "删除密钥" }
                AlertDialogDescription {
                    "确认删除「{token.name}」({token.key_preview})？删除后使用该密钥的调用会立即失败, 且无法恢复。"
                }
                if !err().is_empty() {
                    Alert { variant: AlertVariant::Destructive,
                        AlertDescription { "{err()}" }
                    }
                }

                AlertDialogFooter { class: "gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        class: "flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Destructive,
                        size: ButtonSize::Sm,
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
