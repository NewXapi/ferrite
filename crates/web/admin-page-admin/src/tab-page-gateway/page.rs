//! 网关渠道健康面板:实时观测 gateway dispatch 的渠道冷却 / 慢启动状态。
//!
//! 数据源:`GET /api/gateway/health` (admin 白名单,见
//! `admin-observe/gateway_health.rs`)。响应**只含有记录的渠道**——
//! 正常态是空数组,不是错误。
//!
//! 本文件只留面板外壳:状态 (拉取/错误/刷新) + 轮询拉取 + 四态分支;
//! 单行渲染 (渠道名 / 模型 Badge / 三态 Badge / 倒计时) 见
//! [`super::row::GatewayHealthRow`],三态语义色见
//! [`super::shared`]。
//!
//! 渲染约定(仓库铁律):
//! - 卡片面板模式,禁 `<table>`:`rounded-xl border bg-card divide-y`
//!   容器 + `flex flex-wrap` 行;
//! - 三态:loading=skeleton、error=柔和红边卡、empty=虚线占位卡;
//! - 轮询:挂载即拉一次;仅当存在 cooling / slow_start 条目时按 5s
//!   间隔继续轮询,全 ok 或空时停止 (防无意义轮询)。
//!
//! 交互元素带 `data-testid`,容器带 `role` + `aria-label`
//! (仓库 UI 验证约定,PR smoke 走 ariaSnapshot)。
//!
//! 边界:文案常量与语义色在 `shared`,单行渲染在 `row`;本文件不发写请求
//! (本 tab 只读),也不做失败重试策略(交给用户点按钮)。

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use client::fetch_gateway_health;
use client::{ApiClient, GatewayHealthItem, HealthItemState};

use super::row::GatewayHealthRow;
use super::shared::{
    BTN_REFRESH, BTN_RETRY, LBL_COUNT_COOLING_PREFIX, LBL_COUNT_OK_PREFIX, LBL_COUNT_SLOW_PREFIX,
    LBL_POLLING, LBL_SYNCED, SEC_EMPTY_HINT, SEC_EMPTY_NOTE, SEC_LOAD_FAILED, SEC_PANEL,
    TONE_COOLING, TONE_OK, TONE_SLOW_START,
};

/// 轮询间隔 (仅存在 cooling / slow_start 条目时)。
const POLL_INTERVAL_MS: u32 = 5000;

/// 网关渠道健康面板。
///
/// 【是什么】网关健康 tab 的页面入口组件:标题 + 三态计数徽标 + 刷新按钮 + 四态列表容器。
///
/// 【做什么】负责一次拉取与后续轮询(`fetch_gateway_health`)、四态分支
/// (loading / error / empty / data)、以及三个计数的派生;数据态逐项渲染
/// [`GatewayHealthRow`]。不负责单行内部的回退与取色(在 `row.rs`),
/// 不做写操作(本 tab 只读)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 挂载 → `use_effect`(依赖 `reload`)启动轮询循环,循环内先拉一次快照,
///   若存在 cooling / slow_start 条目则 `TimeoutFuture` 等 5s 再拉,否则 `break`。
/// - 点「刷新」或错误态「重试」→ `reload + 1`,effect 重跑重启循环。
/// - 拉取失败时保留旧快照的轮询意愿(用 `items` 里的异常项决定是否继续),
///   避免一次网络抖动把冷却中的渠道从面板上摘掉。
///
/// 【样式】外层 `section.flex flex-col gap-3`;标题 `text-lg font-medium
/// text-zinc-100`;状态胶囊 `rounded-full bg-zinc-800 px-2 py-0.5 text-[11px]
/// text-zinc-400`;三个计数徽标各自叠 `TONE_*` 语义色;列表容器
/// `rounded-xl border border-zinc-800 bg-card p-4 divide-y divide-zinc-800`;
/// 错误态 `border-red-900/50 bg-red-950/20`;空态 `border-dashed` ;loading 为 3 行
/// `animate-pulse` 骨架。
///
/// 【子组件组成】`GatewayHealthRow`(数据态逐项);其余(标题、徽标、按钮、四态块)
/// 都是本文件内联 rsx,未再拆分。
///
/// 【数据流】
/// - 对内(入):无 prop —— 面板不接收外部参数,数据全部来自 `fetch_gateway_health`。
/// - 对外(出):无 EventHandler;`items` / `loading` / `err` / `reload` 四个 Signal
///   都只在本文件内读写(`reload` 是唯一的「入口」,由两个按钮自增)。
#[component]
pub fn GatewayHealthPanel() -> Element {
    // —— 面板状态:全部只服务本组件,不跨组件,故就地持有 ——
    let items = use_signal(Vec::<GatewayHealthItem>::new);
    let loading = use_signal(|| true);
    let err = use_signal(|| None::<String>);

    // 手动刷新计数:每次点「刷新/重试」自增,use_effect 依赖它重跑循环。
    let mut reload = use_signal(|| 0u32);

    // 轮询循环:挂载或手动刷新时启动一次 (reload 不变则 effect 不重跑,
    // 无重复循环)。每轮拉快照,有 cooling / slow_start 则 5s 后继续,
    // 无 (全 ok / 空) 则停止,防无意义轮询。错误时保留快照的轮询意愿,
    // 网络抖动不应把冷却中的渠道从面板上摘掉。
    use_effect(move || {
        let _ = reload();
        let mut items_sig = items;
        let mut loading_sig = loading;
        let mut err_sig = err;
        spawn(async move {
            loop {
                let client = ApiClient::shared().clone();
                let keep_polling = match fetch_gateway_health(&client).await {
                    Ok(view) => {
                        items_sig.set(view.items.clone());
                        err_sig.set(None);
                        loading_sig.set(false);
                        view.items.iter().any(|i| i.state != HealthItemState::Ok)
                    }
                    Err(e) => {
                        err_sig.set(Some(e.to_string()));
                        loading_sig.set(false);
                        items_sig
                            .read()
                            .iter()
                            .any(|i| i.state != HealthItemState::Ok)
                    }
                };
                if !keep_polling {
                    break;
                }
                TimeoutFuture::new(POLL_INTERVAL_MS).await;
            }
        });
    });

    let list = items();
    let cooling_count = list
        .iter()
        .filter(|i| i.state == HealthItemState::Cooling)
        .count();
    let slow_count = list
        .iter()
        .filter(|i| i.state == HealthItemState::SlowStart)
        .count();
    let ok_count = list
        .iter()
        .filter(|i| i.state == HealthItemState::Ok)
        .count();
    let polling = cooling_count + slow_count > 0;

    rsx! {
        section {
            class: "flex flex-col gap-3",
            role: "region",
            "aria-label": SEC_PANEL,
            "data-testid": "gateway-health-panel",

            // 标题 + 统计 + 手动刷新 (交互元素带 data-testid)
            div { class: "flex flex-wrap items-center justify-between gap-2",
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
                    button {
                        class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-1 text-xs text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                        "data-testid": "refresh-gateway-health",
                        onclick: move |_| reload.set(reload() + 1),
                        "{BTN_REFRESH}"
                    }
                }
            }

            // 卡片面板容器 (禁 table;rounded-xl border bg-card divide-y + flex-wrap 行)
            div {
                class: "rounded-xl border border-zinc-800 bg-card p-4 divide-y divide-zinc-800",
                "data-testid": "gateway-health-list",

                if let Some(e) = err() {
                    // 错误态:柔和红边卡 (非满屏红),保留重试入口
                    div { class: "rounded-lg border border-red-900/50 bg-red-950/20 px-4 py-6 text-center",
                        "data-testid": "gateway-health-error",
                        p { class: "text-sm text-red-300", "{SEC_LOAD_FAILED}" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            "data-testid": "retry-gateway-health",
                            onclick: move |_| reload.set(reload() + 1),
                            "{BTN_RETRY}"
                        }
                    }
                } else if loading() {
                    // loading=skeleton (animate-pulse 骨架行 ×3)
                    for _ in 0..3 {
                        div { class: "flex flex-wrap items-center gap-2 py-3 first:pt-1 last:pb-1",
                            div { class: "h-4 w-32 animate-pulse rounded bg-zinc-800" }
                            div { class: "h-4 w-24 animate-pulse rounded bg-zinc-800/70" }
                            div { class: "h-4 w-16 animate-pulse rounded bg-zinc-800/50" }
                        }
                    }
                } else if list.is_empty() {
                    // 空态:虚线占位卡 (正常态,后端只返回有记录渠道)
                    div { class: "rounded-lg border border-dashed border-zinc-700 bg-zinc-900/40 px-4 py-8 text-center",
                        "data-testid": "gateway-health-empty",
                        p { class: "text-sm text-zinc-400", "{SEC_EMPTY_NOTE}" }
                        p { class: "mt-1 text-xs text-zinc-500",
                            "{SEC_EMPTY_HINT}" }
                    }
                } else {
                    // 数据态:每个上报过错的渠道一行;行渲染与状态徽标在 row.rs,
                    // 页面只负责拉取/轮询与四态分支。
                    for item in list {
                        {
                            rsx! {
                                GatewayHealthRow { key: "{item.unit_key}", item }
                            }
                        }
                    }
                }
            }
        }
    }
}
