//! 胶囊徽标:分组 / 角色 / 状态共用。
//!
//! 三类徽标外观一致(圆角胶囊 + 11px),只有配色 `tone` 不同,故抽成单一
//! 组件由 `user_card` 复用,避免同款 span 在三处各写一遍。

use dioxus::prelude::*;

/// 胶囊徽标:分组 / 角色 / 状态共用。
///
/// 【是什么】圆角胶囊小徽标,三类元数据(分组/角色/状态)外观一致、只有配色不同。
///
/// 【做什么】按传入文本与配色渲染一个 span;无任何逻辑。
///
/// 【交互逻辑】纯展示,无交互。
///
/// 【样式】`rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}`,tone 由调用方传入。
///
/// 【子组件组成】无。
///
/// 【数据流】
/// - 对内(入):`text` / `tone`。
/// - 对外(出):无。
#[component]
pub fn Badge(text: String, tone: &'static str) -> Element {
    rsx! {
        span {
            class: "rounded-full border px-2 py-0.5 {ui::TYPE_LABEL} {tone}",
            "{text}"
        }
    }
}
