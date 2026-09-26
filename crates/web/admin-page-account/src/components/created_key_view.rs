//! 新建成功视图 — 一次性明文 key 展示 (后端只在创建响应返回一次)。
//! 提供一键复制明文 (这是唯一值得复制的完整密钥), 关闭后无法再查看。

use contract::api::token::CreateTokenResult;
use dioxus::prelude::*;
use ui::button::{Button, ButtonVariant};

/// 【是什么】新建密钥成功后的明文展示弹窗，只出现一次。
///
/// 【做什么】负责渲染新建密钥成功后一次性展示的明文 key：只读输入框显示完整密钥 + 一键复制按钮 + 关闭按钮。提示用户"此信息仅显示一次，关闭后无法再次查看"。不保存明文到任何存储。
///
/// 【交互逻辑】
/// - 点击「完成」/遮罩：调用 on_close，关闭弹窗。
/// - 点击「复制」：通过 Clipboard API fire-and-forget 写入剪贴板，成功后短暂切换按钮文案为「已复制」并变为绿色，1.5 秒后自动恢复。
///
/// 【样式】固定最大宽度 max-w-md，圆角边框、成功主题色 (border-emerald-500/40 bg-zinc-900)，标题与提示使用 emerald-400 绿色。输入框 mono 字体高亮 emerald-300。
///
/// 【子组件组成】ui::button::Button × 1 (完成)；无其他自定义组件。
///
/// 【数据流】
/// - 对内（入）：result (CreateTokenResult) 含明文 key 与元数据 (name 等)；on_close EventHandler 由页面传入。
/// - 对外（出）：仅调用 on_close.call(())，无返回值。复制操作 fire-and-forget，不通知页面。
#[component]
pub fn CreatedKeyView(result: CreateTokenResult, on_close: EventHandler<()>) -> Element {
    let mut copied = use_signal(|| false);
    rsx! {
        div {
            class: "{ui::MODAL_BACKDROP}",
            div {
                class: "w-full max-w-md rounded-2xl border border-emerald-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-4 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-emerald-400", "密钥创建成功" }
                    button {
                        class: "{ui::CLOSE_BTN}",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                p { class: "mb-2 text-xs text-amber-400", "明文密钥只显示这一次,关闭后无法再查看" }
                div { class: "flex items-center gap-2",
                    input {
                        class: "min-w-0 flex-1 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-3 font-mono text-sm text-emerald-300 focus:outline-none",
                        r#type: "text",
                        r#readonly: true,
                        value: "{result.plaintext}"
                    }
                    button {
                        class: if copied() {
                            "shrink-0 rounded-xl border border-emerald-500/40 bg-emerald-500/10 px-3 py-3 text-xs font-medium text-emerald-400"
                        } else {
                            "shrink-0 rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-3 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-800"
                        },
                        "aria-label": "复制明文密钥",
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
                p { class: "mt-2 {ui::TYPE_DESC}", "名称: {result.token.name}" }

                div { class: "mt-5 flex justify-end",
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| on_close.call(()),
                        "完成"
                    }
                }
            }
        }
    }
}
