//! AppShell — Linear 风格三段式布局骨架。
//!
//! 结构：左 rail（slot）+ 主轴列（顶部悬浮导航 slot → 伸缩滚动内容 → 底部
//! 悬浮状态栏 slot）。组件只管骨架与间距，slot 内容全部由调用方注入。

use dioxus::prelude::*;

/// 三段式布局骨架。
///
/// - `rail`：左侧 rail（桌面 w-14 常驻；移动端由组件自身 `hidden md:flex` 隐藏）。
/// - `top_nav`：顶部悬浮导航（page tabs；移动端可加 section 横条）。
/// - `status_bar`：底部悬浮状态栏。
/// - `children`：滚动主内容。
#[component]
pub fn AppShell(
    /// 左侧 rail slot。
    rail: Element,
    /// 顶部悬浮导航 slot。
    top_nav: Element,
    /// 底部悬浮状态栏 slot。
    status_bar: Element,
    /// 主内容。
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "flex h-svh overflow-hidden bg-zinc-950 text-zinc-100",
            {rail}
            // 主轴列：rail 占 w-14，桌面侧移；移动端 rail 隐藏即全宽
            div {
                class: "flex min-w-0 flex-1 flex-col",
                // 顶部悬浮导航区（不随内容滚动）
                div {
                    class: "flex shrink-0 flex-col items-center gap-2 px-4 pt-3",
                    {top_nav}
                }
                // 滚动主内容
                main {
                    class: "min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-4 pt-4 sm:px-6",
                    {children}
                }
                // 底部悬浮状态栏区
                div {
                    class: "flex shrink-0 justify-center px-4 pb-3 pt-2",
                    {status_bar}
                }
            }
        }
    }
}
