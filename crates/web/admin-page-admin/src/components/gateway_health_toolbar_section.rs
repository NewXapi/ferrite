//! 网关渠道健康筛选与操作区:仅包含刷新/重试按钮的纯渲染区段。
//!
//! 本文件不放状态、写回逻辑与任何交互,只负责渲染按钮和状态依赖的样式,
//! 点击回调由上层 `GatewayHealthPanel` 闭包提供。
//!
//! 边界:本目录只服务网关这一个 tab;按钮样式与交互逻辑都放在页面层，
//! 组件只负责渲染按钮和状态状态。

use dioxus::prelude::*;

use super::gateway_health_shared::{BTN_REFRESH, BTN_RETRY, SEC_PANEL, TONE_COOLING};

/// 网关渠道健康工具栏区。
///
/// 【是什么】面板右侧的操作区:刷新按钮 + 错误态重试按钮。
///
/// 【做什么】渲染纯展示组件,不带任何状态,只负责按钮样式和文案,
/// 点击回调由上层组件闭包提供。
///
/// 【交互逻辑】纯渲染，无交互(无 onclick、不改状态、不发网络),回调由
/// 上层组件闭包提供。
///
/// 【样式】刷新按钮 `rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-1
/// text-xs text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white`;
/// 重试按钮 `mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800`。
///
/// 【子组件组成】无子组件,只有内联 `div` / `span` / `button`。
///
/// 【数据流】
/// - 对内(入):`on_refresh` / `on_retry` EventHandler 由上层组件闭包提供。
/// - 对外(出):无 State Signal 写回。
#[component]
pub fn GatewayHealthToolbarSection(
    on_refresh: EventHandler<MouseEvent>,
    on_retry: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center gap-1.5 text-[11px]",
            button {
                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-1 text-xs text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                "data-testid": "refresh-gateway-health",
                onclick: move |e| on_refresh.call(e),
                "{BTN_REFRESH}"
            }
            button {
                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                "data-testid": "retry-gateway-health",
                onclick: move |e| on_retry.call(e),
                "{BTN_RETRY}"
            }
        }
    }
}