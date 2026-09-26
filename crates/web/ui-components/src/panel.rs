//! 选择器面板外壳:形似输入框的聚焦面板,供 chips 选择器(分组/角色/搜索)复用。
//!
//! `focus-within` 高亮是主题样式 token —— 全仓只在本文件出现一次
//! (`FIELD_PANEL`),主题化时改这一个常量即可;页面组件不自建同型外壳。

use dioxus::prelude::*;

/// 面板外壳样式:形似输入框的容器 + 聚焦时边框提亮。
/// 模态遮罩：全屏居中 + 半透明黑底 + 背景模糊。点遮罩关闭由调用方挂 onclick。
pub const MODAL_BACKDROP: &str =
    "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm";

/// 模态内容卡：居中圆角卡，最大宽度 `max-w-md`。
pub const MODAL_CARD: &str =
    "w-full max-w-md rounded-2xl border border-border bg-card p-5 shadow-xl";

/// 模态标题行：左标题 + 右关闭按钮。
pub const MODAL_HEADER: &str = "mb-5 flex items-center justify-between";
/// 关闭按钮 class（与 `CloseButton` 组件同款）。页面若自建按钮结构，复用此常量。
pub const CLOSE_BTN: &str =
    "rounded-lg p-1.5 {crate::C_MUTED} transition-colors hover:bg-zinc-800 hover:text-zinc-200";
pub const FIELD_PANEL: &str =
    "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2.5 focus-within:border-zinc-500";

/// 选择器面板外壳:容器 div(`FIELD_PANEL`)+ testid / role / aria-label,
/// 内容槽 `children`。语义(选中集合怎么变)归调用侧,本组件只管外观与可及性。
#[component]
pub fn FieldPanel(
    /// 面板 data-testid(快照定位用)
    testid: String,
    /// role 属性(选择器面板固定 `"group"`)
    role: &'static str,
    /// aria-label(面板用途文案)
    aria_label: String,
    /// 面板内容(chips 行 / 空态文案等)
    children: Element,
) -> Element {
    rsx! {
        div {
            class: FIELD_PANEL,
            "data-testid": "{testid}",
            role: "{role}",
            "aria-label": "{aria_label}",
            {children}
        }
    }
}
