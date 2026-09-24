//! 横向资料项: label 与 value 同行 (label 灰、value 等宽字体)。
//! 由父容器 flex-wrap 控制换行, 单项不自带换行逻辑。
//! `copyable` 时在 value 后挂复制小按钮。注意: value 可短显
//! (如用户 ID 显示 "0000…aa01"), 复制按钮实际复制的内容取 `copy_value`;
//! 未传 (None) 时回退复制 `value` (对非短显项 = 复制原值, 向后兼容)。

use dioxus::prelude::*;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

#[component]
pub fn ProfileItem(
    label: &'static str,
    value: String,
    copyable: bool,
    copy_value: Option<String>,
) -> Element {
    let copy_text = copy_value.unwrap_or_else(|| value.clone());
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "shrink-0 text-zinc-400", "{label}" }
            span {
                class: "min-w-0 break-all font-mono text-zinc-200",
                title: "{value}",
                "{value}"
            }
            if copyable {
                div { class: "shrink-0 self-center",
                    CopyPlaintextButton { text: copy_text, label: format_args!("复制完整{label}").to_string() }
                }
            }
        }
    }
}

/// 复制完整明文的小图标按钮: 点击写入剪贴板, 复制成功有可见反馈
/// (按钮文案瞬时变「已复制」/ 图标变 ✓)。
/// 只用于真正值得复制的完整值 —— 一次性明文密钥 / 完整用户 ID;
/// 掩码预览 (sk-ab****ef) 禁止用此组件, 复制掩码是功能错误。
/// 密钥卡复制按钮常驻行尾 (维护者批注 2026-09-21 15:44); 旧密钥以 disabled_reason 禁用。
/// 复制语义与 ui-components session::copy_text_to_clipboard 一致:
/// Clipboard API fire-and-forget, 提交即视为成功。
#[component]
pub fn CopyPlaintextButton(
    text: String,
    label: String,
    /// Some(原因) = 禁用态: 图标照常渲染在行尾但不可点, 悬停显示原因。
    /// 密钥卡用: 旧密钥明文仅创建时可见 (后端只存 sha256), 无法复制。
    #[props(default)]
    disabled_reason: Option<String>,
) -> Element {
    let mut copied = use_signal(|| false);
    let disabled = disabled_reason.is_some();
    let hint = disabled_reason.unwrap_or_else(|| label.clone());
    rsx! {
        Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconXs,
            title: "{hint}",
            "aria-label": "{hint}",
            disabled,
            class: if disabled {
                "opacity-40"
            } else if copied() {
                "text-emerald-400"
            } else {
                ""
            },
            onclick: move |_| {
                if disabled {
                    return;
                }
                let ok = ui::copy_text_to_clipboard(text.as_str());
                copied.set(ok);
                let mut c = copied;
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(1500).await;
                    c.set(false);
                });
            },
            if copied() {
                span { class: "block h-3.5 w-3.5 text-center text-[11px] leading-[14px]", "✓" }
            } else {
                svg {
                    class: "h-3.5 w-3.5",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    stroke_width: "2",
                    rect { x: "9", y: "9", width: "13", height: "13", rx: "2" }
                    path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                }
            }
        }
    }
}
