//! 新建成功视图 — 一次性明文 key 展示 (后端只在创建响应返回一次)，rust-ui Dialog 重构。
//! 提供一键复制明文 (这是唯一值得复制的完整密钥), 关闭后无法再查看。

use contract::api::token::CreateTokenResult;
use dioxus::prelude::*;

use ui::components::rui_button::{Button, ButtonSize, ButtonVariant};
use ui::components::rui_dialog::{
    Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
};
use ui::components::rui_input::{Input, InputType};

/// 【是什么】新建密钥成功后的明文展示弹窗，只出现一次。
///
/// 【做什么】负责渲染新建密钥成功后一次性展示的明文 key：只读输入框显示完整密钥 + 一键复制按钮 + 完成按钮。提示用户"此信息仅显示一次，关闭后无法再次查看"。不保存明文到任何存储。
///
/// 【交互逻辑】
/// - 点击「完成」/遮罩/X：调用 on_close，关闭弹窗。
/// - 点击「复制」：通过 Clipboard API fire-and-forget 写入剪贴板，成功后短暂切换按钮文案为「已复制」并变为绿色，1.5 秒后自动恢复。
///
/// 【样式】rust-ui Dialog 承载（max-w-md）；明文 Input 只读等宽 + emerald 高亮；复制/完成 rui Button。
///
/// 【子组件组成】rui Dialog 族 + Input (readonly) + Button ×2 (复制/完成)
///
/// 【数据流】
/// - 对内（入）：result (CreateTokenResult) 含明文 key 与元数据 (name 等)；on_close EventHandler 由页面传入。
/// - 对外（出）：仅调用 on_close.call(())，无返回值。复制操作 fire-and-forget，不通知页面。
#[component]
pub fn CreatedKeyView(result: CreateTokenResult, on_close: EventHandler<()>) -> Element {
    let mut copied = use_signal(|| false);
    rsx! {
        Dialog {
            DialogContent { open: true, on_close: move |_| on_close.call(()),
                DialogHeader {
                    DialogTitle { class: "text-emerald-400", "密钥创建成功" }
                    DialogDescription { class: "text-amber-400",
                        "明文密钥只显示这一次,关闭后无法再查看"
                    }
                }

                DialogBody {
                    div { class: "flex items-center gap-2",
                        Input {
                            r#type: InputType::Text,
                            class: "min-w-0 flex-1 border-emerald-500/30 bg-zinc-950 font-mono text-emerald-300",
                            readonly: true,
                            value: "{result.plaintext}",
                        }
                        Button {
                            variant: if copied() { ButtonVariant::Success } else { ButtonVariant::Outline },
                            size: ButtonSize::Sm,
                            aria_label: "复制明文密钥",
                            onclick: move |_| {
                                // fire-and-forget 写入剪贴板, 提交即视为发起成功
                                let ok = ui::copy_text_to_clipboard(&result.plaintext);
                                copied.set(ok);
                                let mut c = copied;
                                spawn(async move {
                                    gloo_timers::future::TimeoutFuture::new(1500).await;
                                    c.set(false);
                                });
                            },
                            if copied() { "已复制" } else { "复制" }
                        }
                    }
                    p { class: "text-xs text-zinc-500", "名称: {result.token.name}" }
                }

                DialogFooter {
                    Button {
                        size: ButtonSize::Sm,
                        onclick: move |_| on_close.call(()),
                        "完成"
                    }
                }
            }
        }
    }
}
