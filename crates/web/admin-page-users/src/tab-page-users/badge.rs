//! 胶囊徽标:分组 / 角色 / 状态共用。
//!
//! 三类徽标外观一致(圆角胶囊 + 11px),只有配色 `tone` 不同,故抽成单一
//! 组件由 `user_card` 复用,避免同款 span 在三处各写一遍。

use dioxus::prelude::*;

/// 徽标:分组 / 角色 / 状态共用
#[component]
pub fn Badge(text: String, tone: &'static str) -> Element {
    rsx! {
        span {
            class: "rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}",
            "{text}"
        }
    }
}
