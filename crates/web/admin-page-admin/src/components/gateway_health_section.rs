//! 网关渠道健康面板:五张概览卡(冷却/慢启动/正常计数)的纯渲染区段。
//!
//! 概览卡语义色（green/yellow/red）与三态徽标共用同一组常量;
//! 文案常量在 `gateway_health_shared`。
//!
//! 本文件不放状态、写回逻辑与任何交互,只负责渲染图表占用面积,数据由
//! 上层 `GatewayHealthPanel` 的派生状态 `cooling_count` / `slow_count` / `ok_count`
//! 传入。
//!
//! 边界:本目录只服务网关这一个 tab;概览卡的**视觉外壳**(四页签 + 多 tab 叠放
//!   高度)定义在 ui-components 的 `admin_card`,这里不复制卡片样式。

use dioxus::prelude::*;

use super::gateway_health_shared::{LBL_COUNT_COOLING_PREFIX, LBL_COUNT_OK_PREFIX, LBL_COUNT_SLOW_PREFIX, LBL_POLLING, LBL_SYNCED, SEC_PANEL, TONE_COOLING, TONE_OK, TONE_SLOW_START};

/// 网关渠道健康概览统计区。
///
/// 【是什么】三张计数徽标(冷却 / 慢启动 / 正常)和状态胶囊(轮询中 / 已同步)
/// 的组合区域,对应面板顶部标题 + 统计 + 刷新按钮。
///
/// 【做什么】渲染纯展示组件,不带交互回调,数据全部来自 `GatewayHealthPanel` 的
/// 派生状态 `cooling_count` / `slow_count` / `ok_count` 与 `polling`。
///
/// 【交互逻辑】纯渲染，无交互(无 onclick、不改状态、不发网络)。
///
/// 【样式】标题 `text-lg font-medium text-zinc-100`;统计徽标叠放:冷却=红 / 慢启动=黄 / 正常=绿;
/// 状态胶囊 `rounded-full bg-zinc-800 px-2 py-0.5 text-[11px] text-zinc-400`。
///
/// 【子组件组成】无子组件,只有内联 `div` / `span`。
///
/// 【数据流】
/// - 对内(入):`cooling_count` / `slow_count` / `ok_count` / `polling` (Signal 派生) 由
///   上层 `GatewayHealthPanel` 持有并更新。
/// - 对外(出):无 EventHandler。
#[component]
pub fn GatewayHealthSection(
    cooling_count: usize,
    slow_count: usize,
    ok_count: usize,
    polling: bool,
) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center gap-2",
            div { class: "flex items-center gap-2",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_PANEL}" }
                span { class: "rounded-full bg-zinc-800 px-2 py-0.5 text-[11px] text-zinc-400",
                    if polling { "{LBL_POLLING}" } else { "{LBL_SYNCED}" }
                }
            }
            div { class: "flex flex-wrap items-center gap-1.5 text-[11px]",
                span { class: "rounded-full border px-2 py-0.5 {TONE_COOLING}",
                    "{LBL_COUNT_COOLING_PREFIX}{cooling_count}" }
                span { class: "rounded-full border px-2 py-0.5 {TONE_SLOW_START}",
                    "{LBL_COUNT_SLOW_PREFIX}{slow_count}" }
                span { class: "rounded-full border px-2 py-0.5 {TONE_OK}",
                    "{LBL_COUNT_OK_PREFIX}{ok_count}" }
            }
        }
    }
}