//! 横向资料项: label 与 value 同行 (label 灰、value 等宽字体)。
//! 由父容器 flex-wrap 控制换行, 单项不自带换行逻辑。
//! `copyable` 时在 value 后挂复制小按钮。注意: value 可短显
//! (如用户 ID 显示 "0000…aa01"), 复制按钮实际复制的内容取 `copy_value`;
//! 未传 (None) 时回退复制 `value` (对非短显项 = 复制原值, 向后兼容)。

use dioxus::prelude::*;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

/// 【是什么】横向资料项组件，展示单个字段名 + 值，支持复制选项。
///
/// 【做什么】负责渲染单个资料字段：左侧灰字 label，右侧等宽字体 value (支持 title 悬停全值)；copyable=true 时右侧追加一个复制小按钮。不自带 flex-wrap 逻辑，由父容器 (ProfileSection) 控制布局。
///
/// 【交互逻辑】纯展示。当 copyable=true 时，点击右侧复制按钮触发 copy_text_to_clipboard，复制成功后短暂变绿色 + ✓ 图标，1.5 秒后恢复。不触发 API 调用，不修改外部状态。
///
/// 【样式】行内 flex 布局，gap-2；label 灰字 (text-zinc-400)，value 等宽字体 (font-mono text-zinc-200)，min-w-0 防溢出；复制按钮 Ghost variant + IconXs size，成功态 text-emerald-400。
///
/// 【子组件组成】ui::components::button::Button (CopyPlaintextButton) × 0 或 1
///
/// 【数据流】
/// - 对内（入）：label (&'static str) 字段名；value (String) 展示值；copyable (bool) 是否显示复制按钮；copy_value (Option<String>) 实际要复制的内容 (不传则复制 value)。
/// - 对外（出）：无回调，纯展示组件。复制操作 fire-and-forget，不通知父组件。
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
/// 复制语义与 ui-components session::copy_text_to_clipboard 一致:
/// Clipboard API fire-and-forget, 提交即视为成功。
#[component]
fn CopyPlaintextButton(text: String, label: String) -> Element {
    let mut copied = use_signal(|| false);
    rsx! {
        Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconXs,
            title: "{label}",
            "aria-label": "{label}",
            class: if copied() { "text-emerald-400" } else { "" },
            onclick: move |_| {
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
