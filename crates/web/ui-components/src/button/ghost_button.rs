use dioxus::prelude::*;

/// 次要操作按钮（描边幽灵按钮）：错误区「重试」、区段内「导出」等。
///
/// 原 6 个 `list.rs` 与 system 的 `overview.rs` 各写一份。
#[component]
pub fn GhostButton(
    /// 按钮文案
    label: String,
    /// 点击回调
    onclick: EventHandler<MouseEvent>,
    /// 可选测试标识
    #[props(default)]
    testid: Option<String>,
    /// 是否占满剩余宽度（弹窗底部按钮用）
    #[props(default = false)]
    grow: bool,
) -> Element {
    let grow_cls = if grow { "flex-1 " } else { "" };
    rsx! {
        button {
            class: "mt-3 {grow_cls}rounded-xl border {crate::T_border_zinc_700} px-3 py-1.5 {crate::T_text_xs} {crate::T_text_zinc_300} hover:{crate::T_bg_zinc_800}",
            "data-testid": testid.as_deref(),
            onclick: move |e| onclick.call(e),
            "{label}"
        }
    }
}
