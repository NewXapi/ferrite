//! 系统概览区(编号段 1-3):运行指标统计卡 + 实体统计卡 + 运行环境明细行。
//!
//! 三段共享同一数据源(`SystemInfoView`)与 loading/err/reload,抽成一个组件。
//! stats / count_cards / env_rows 三个派生 Vec 只被这三段消费,随渲染一起
//! 搬进组件内部;页面不再持有派生逻辑。
//!
//! 状态归属:data/loading/err 由页面持有(拉取 effect + 刷新按钮跨组件),
//! 通过值 + 事件传入;组件内部零 use_signal。

use dioxus::prelude::*;

use super::shared::{
    BTN_REFRESH, BTN_RETRY, LBL_CONN_POOL, LBL_COUNT_ACTIVE_CHANNELS, LBL_COUNT_CHANNELS,
    LBL_COUNT_MODELS, LBL_COUNT_TOKENS, LBL_COUNT_USERS, LBL_CPU_LOAD, LBL_DB_STATUS, LBL_HOSTNAME,
    LBL_LOAD_AVG, LBL_MEMORY_DETAIL, LBL_OS, LBL_PROCESS_RSS, LBL_SERVICE_VERSION, LBL_STARTED_AT,
    LBL_SYS_MEMORY, LBL_UPTIME, MSG_EMPTY, MSG_EMPTY_HINT, MSG_LOAD_FAILED, MSG_LOADING,
    SEC_COUNTS, SEC_ENV, SEC_ENV_NOTE, SEC_STATS, SystemInfoView, format_bytes, format_db_status,
    format_load, format_started_at, format_uptime,
};
use crate::tab_page_groups::StatCard;

/// 系统概览区:概览统计 / 实体统计 / 运行环境明细(编号段 1-3)。
///
/// 【是什么】系统 tab 的三段合体展示:概览统计卡 + 实体统计卡 + 运行环境明细行。
///
/// 【做什么】把页面拉到的 `SystemInfoView` 拆成三组渲染数据(`stats` /
/// `count_cards` / `env_rows` 三个派生 Vec),按 err / loading / 有数据渲染;
/// 概览段空时另给「后端未返回采集数据」说明。不负责拉数据(页面 effect 负责)、
/// 不负责刷新动作本身(只把点击抛回页面)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点区段头「刷新」或错误态「重试」→ 均调 `on_refresh`,抛回页面
///   (MouseEvent,页面 `reload + 1` 触发 effect 重拉)。
/// 数据交互:本组件**不发网络请求**,拉取链路在页面 `use_effect` 里。
///
/// 【样式】外壳三段:`section#system-sec-stats` 为 `scroll-mt-8 space-y-3`
/// (区段头左 `text-lg font-medium text-zinc-100` 标题 + 右描边按钮);统计卡
/// 网格 `grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5`(手机 1 / 中屏 3
/// / 大屏 5 列);`section#system-sec-counts` 同网格但仅在有数据时渲染;
/// `section#system-sec-env` 为 `rounded-xl border border-zinc-800 bg-zinc-900/60
/// p-5`,明细行 `divide-y divide-zinc-800/80` + 左标签 `text-zinc-500` / 右值
/// `font-mono text-zinc-300`。错误/加载/空态均为虚线或红边圆角块 + 居中文字。
///
/// 【子组件组成】`StatCard`(来自 `tab_page_groups`,概览卡与统计卡共用);
/// 其余为原生元素,无自定义子组件。
///
/// 【数据流】
/// - 对内(入):`data`(页面 effect 拉到的 `SystemInfoView`,`None` = 未拿到)、
///   `loading`(首屏加载中)、`err`(拉取失败摘要,`Some` 时优先于 loading 渲染);
///   三者与 `on_refresh` 均由页面持有并传入。
/// - 对外(出):`on_refresh` → 页面 `reload` signal 递增,触发 effect 重拉
///   `/api/system-info`。
#[component]
pub fn SystemOverview(
    /// `/api/system-info` 采集结果(None = 尚未拿到)
    data: Option<SystemInfoView>,
    /// 首屏加载中
    loading: bool,
    /// 拉取失败摘要
    err: Option<String>,
    /// 刷新按钮 / 错误态重试(跨组件交互,页面重拉)
    on_refresh: EventHandler<MouseEvent>,
) -> Element {
    // 概览统计卡:运行时长 / 内存占用 / CPU 负载 / 数据库 / 进程内存
    let stats: Vec<(String, &'static str)> = match &data {
        Some(v) => vec![
            (format_uptime(v.uptime.uptime_seconds), LBL_UPTIME),
            (
                format!(
                    "{}/{}",
                    format_bytes(v.memory.system_used_bytes),
                    format_bytes(v.memory.system_total_bytes)
                ),
                LBL_SYS_MEMORY,
            ),
            (
                format!("{} · {}核", format_load(v.cpu.load_avg_1m), v.cpu.num_cpus),
                LBL_CPU_LOAD,
            ),
            (format_db_status(&v.database.status), LBL_DB_STATUS),
            (format_bytes(v.memory.process_rss_bytes), LBL_PROCESS_RSS),
        ],
        None => Vec::new(),
    };

    // 实体统计卡:核心业务实体行数
    let count_cards: Vec<(String, &'static str)> = match &data {
        Some(v) => vec![
            (v.counts.users.to_string(), LBL_COUNT_USERS),
            (v.counts.channels.to_string(), LBL_COUNT_CHANNELS),
            (
                v.counts.active_channels.to_string(),
                LBL_COUNT_ACTIVE_CHANNELS,
            ),
            (v.counts.models.to_string(), LBL_COUNT_MODELS),
            (v.counts.tokens.to_string(), LBL_COUNT_TOKENS),
        ],
        None => Vec::new(),
    };

    // 运行环境明细行
    let env_rows: Vec<(String, String)> = match &data {
        Some(v) => vec![
            (LBL_SERVICE_VERSION.to_string(), v.runtime.version.clone()),
            (
                LBL_OS.to_string(),
                format!("{} / {}", v.runtime.os, v.runtime.arch),
            ),
            (LBL_HOSTNAME.to_string(), v.runtime.hostname.clone()),
            (
                LBL_STARTED_AT.to_string(),
                format_started_at(&v.uptime.started_at),
            ),
            (
                LBL_LOAD_AVG.to_string(),
                format!(
                    "1m {} · 5m {} · 15m {}",
                    format_load(v.cpu.load_avg_1m),
                    format_load(v.cpu.load_avg_5m),
                    format_load(v.cpu.load_avg_15m)
                ),
            ),
            (
                LBL_CONN_POOL.to_string(),
                format!(
                    "大小 {} · 空闲 {}",
                    v.database.pool_size, v.database.idle_connections
                ),
            ),
            (
                LBL_MEMORY_DETAIL.to_string(),
                format!(
                    "已用 {} · 可用 {} · 进程 {}",
                    format_bytes(v.memory.system_used_bytes),
                    format_bytes(v.memory.system_available_bytes),
                    format_bytes(v.memory.process_rss_bytes)
                ),
            ),
        ],
        None => Vec::new(),
    };

    rsx! {
        // 1. 系统概览统计区(真实 /api/system-info 采集)
        section {
            id: "system-sec-stats",
            "data-testid": "system-panel",
            class: "scroll-mt-8 space-y-3",
            div { class: "flex items-center justify-between",
                h2 { class: "{ui::TYPE_TITLE}", "{SEC_STATS}" }
                button {
                    class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 {ui::TYPE_DESC} transition-colors hover:bg-zinc-800",
                    "data-testid": "refresh-system",
                    onclick: on_refresh,
                    {BTN_REFRESH}
                }
            }
            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                    p { class: "text-sm {ui::C_DANGER}", {MSG_LOAD_FAILED} }
                    p { class: "mt-1 text-xs {ui::C_DANGER}", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 {ui::TYPE_DESC} hover:bg-zinc-800",
                        onclick: on_refresh,
                        {BTN_RETRY}
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                    p { class: "{ui::C_MUTED}", {MSG_LOADING} }
                }
            } else if stats.is_empty() {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                    p { class: "{ui::C_MUTED}", {MSG_EMPTY} }
                    p { class: "mt-1 {ui::TYPE_DESC}", {MSG_EMPTY_HINT} }
                }
            } else {
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    for (value, label) in stats {
                        StatCard { value, label }
                    }
                }
            }
        }

        // 2. 实体统计(核心业务实体行数)
        if !count_cards.is_empty() {
            section { id: "system-sec-counts", class: "scroll-mt-8 space-y-3",
                h2 { class: "{ui::TYPE_TITLE}", "{SEC_COUNTS}" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    for (value, label) in count_cards {
                        StatCard { value, label }
                    }
                }
            }
        }

        // 3. 运行环境明细行
        if !env_rows.is_empty() {
            section {
                id: "system-sec-env",
                class: "scroll-mt-8 rounded-xl border border-zinc-800 bg-zinc-900/60 p-5 space-y-4",
                div {
                    h2 { class: "{ui::TYPE_CARD_TITLE}", "{SEC_ENV}" }
                    p { class: "{ui::TYPE_DESC}", {SEC_ENV_NOTE} }
                }
                div { class: "divide-y divide-zinc-800/80",
                    for (label, value) in env_rows {
                        div { class: "flex items-center justify-between gap-4 py-2.5",
                            span { class: "shrink-0 {ui::TYPE_DESC}", "{label}" }
                            span { class: "break-all text-right text-xs font-mono text-zinc-300", "{value}" }
                        }
                    }
                }
            }
        }
    }
}
