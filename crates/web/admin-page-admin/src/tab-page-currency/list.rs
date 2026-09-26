//! 货币列表区：四态渲染（loading / error / empty / data）+ 表格行。
//! 纯展示组件：数据与写回调由 `page` 注入。
//!
//! 边界:表格自身的列宽/描边/状态色都在本文件;拉取、错误回退与软禁用
//! 的写回逻辑不在这里(在 `page.rs`);文案常量统一来自 `super::shared`。

use dioxus::prelude::*;

use super::shared::{
    BTN_DISABLE, BTN_EDIT, BTN_RETRY, LBL_COL_CODE, LBL_COL_KIND, LBL_COL_NAME, LBL_COL_PRECISION,
    LBL_COL_RATE, LBL_COL_STATUS, LBL_COL_SYMBOL, LBL_LIST_REGION, LBL_STATUS_DISABLED,
    LBL_STATUS_ENABLED, MSG_EMPTY, MSG_LOAD_FAILED_PREFIX, SEC_LIST,
};
use crate::api::CurrencyView;

/// 货币定义列表。
///
/// 【是什么】货币 tab 的列表区:标题 + 四态分支(加载骨架 / 错误条 / 空态 / 数据表格)。
///
/// 【做什么】按 `err` / `loading` / `defs` 的取值渲染四种形态之一;有数据时把每条
/// `CurrencyView` 铺成一行(代号/符号/名称/kind 徽标/汇率/小数位/状态/操作)。不负责
/// 拉取与软禁用的网络写回(在 `page.rs`),不负责表单录入(在 `form.rs`)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点错误条「重试」→ `on_retry` 抛回页面(MouseEvent,页面 `reload + 1` 触发重拉)。
/// - 点行内「编辑」→ `on_edit(d.clone())` 回传整行,页面据此回填表单(无网络)。
/// - 点行内「停用」→ `on_disable(code.clone())` 只回传代号,页面按内存定义原样
///   回写 `enabled=false`(该写回触发 PUT `/api/currency`,由页面发起)。
/// 本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 `section.space-y-2 rounded-xl border border-border bg-card/60 p-3`;
/// 加载态为 3 条 `h-8 animate-pulse rounded-lg bg-secondary/60` 骨架;错误态为红底
/// `rounded-xl border-destructive bg-destructive` 条;空态为虚线描边
/// `border-dashed border-border` 居中块;数据态为 `overflow-x-auto` 包
/// `w-full text-left text-sm` 表格,行以 `border-t border-border` 分隔。
///
/// 【子组件组成】无独立子组件,全部 rsx 在本文件内联:标题 `h3`、骨架 `div`、
/// 错误条、空态块、`table`(`thead` + `tbody`)与行内两个 `button`。
///
/// 【数据流】
/// - 对内(入):`defs`(页面拉到的全量货币定义)、`loading`(加载态)、`err`(错误
///   摘要,Some 时优先于空态渲染)、`on_edit` / `on_disable` / `on_retry` 三个出口。
/// - 对外(出):`on_edit(CurrencyView)` → 页面 `start_edit` 回填表单;
///   `on_disable(String)` → 页面 `disable` 软禁用并 `reload + 1`;
///   `on_retry(MouseEvent)` → 页面 `reload + 1` 重拉。
#[component]
pub fn CurrencyList(
    defs: Vec<CurrencyView>,
    loading: bool,
    err: Option<String>,
    on_edit: EventHandler<CurrencyView>,
    on_disable: EventHandler<String>,
    on_retry: EventHandler<MouseEvent>,
) -> Element {
    // (edit_id, disable_id, disable_code, view) 预构建并预格式化 testid：
    // rsx 属性全走 move、零借用，绕开宏展开里借用/移动顺序不可靠的问题。
    let rows: Vec<(String, String, String, CurrencyView)> = defs
        .into_iter()
        .map(|d| {
            let edit_id = format!("currency-edit-{}", d.code);
            let disable_id = format!("currency-disable-{}", d.code);
            let disable_code = d.code.clone();
            (edit_id, disable_id, disable_code, d)
        })
        .collect();

    rsx! {
        section { class: "space-y-2 rounded-xl border border-border bg-card/60 p-3",
            role: "region",
            "aria-label": LBL_LIST_REGION,
            "data-testid": "currency-list-section",
            h3 { class: "{ui::TYPE_CARD_TITLE}", "{SEC_LIST}" }
            if let Some(e) = err {
                div { class: "rounded-lg border border-destructive bg-destructive p-3 text-sm {ui::C_DANGER}",
                    "data-testid": "currency-list-error",
                    "{MSG_LOAD_FAILED_PREFIX}{e}"
                    button {
                        class: "ml-3 rounded-lg border border-destructive px-2 py-1 {ui::C_DANGER} hover:bg-destructive",
                        "data-testid": "currency-retry",
                        onclick: move |e| on_retry.call(e),
                        "{BTN_RETRY}"
                    }
                }
            } else if loading {
                div { class: "space-y-2",
                    for _i in 0..3 {
                        div { class: "h-8 animate-pulse rounded-lg bg-secondary/60" }
                    }
                }
            } else if rows.is_empty() {
                div { class: "rounded-lg border border-dashed border-border p-6 text-center {ui::TYPE_BODY}",
                    "{MSG_EMPTY}"
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full text-left {ui::TYPE_BODY}",
                        "data-testid": "currency-list",
                        thead { class: "{ui::C_MUTED}",
                            tr {
                                th { class: "px-2 py-1", "{LBL_COL_CODE}" }
                                th { class: "px-2 py-1", "{LBL_COL_SYMBOL}" }
                                th { class: "px-2 py-1", "{LBL_COL_NAME}" }
                                th { class: "px-2 py-1", "{LBL_COL_KIND}" }
                                th { class: "px-2 py-1", "{LBL_COL_RATE}" }
                                th { class: "px-2 py-1", "{LBL_COL_PRECISION}" }
                                th { class: "px-2 py-1", "{LBL_COL_STATUS}" }
                                th { class: "px-2 py-1", "" }
                            }
                        }
                        tbody {
                            for (edit_id, disable_id, disable_code, d) in rows {
                                tr { class: "border-t border-border",
                                    td { class: "px-2 py-1 font-mono text-foreground", "{d.code}" }
                                    td { class: "px-2 py-1", "{d.symbol}" }
                                    td { class: "px-2 py-1 {ui::C_MUTED}", "{d.name}" }
                                    td { class: "px-2 py-1",
                                        span { class: if d.kind == "fiat" { "rounded bg-warning px-1.5 py-0.5 text-xs text-warning-foreground" } else { "rounded bg-info px-1.5 py-0.5 text-xs text-info-foreground" },
                                            "{d.kind}"
                                        }
                                    }
                                    td { class: "px-2 py-1 {ui::C_MUTED}", "{d.internal_rate}" }
                                    td { class: "px-2 py-1 {ui::C_MUTED}", "{d.precision}" }
                                    td { class: "px-2 py-1",
                                        if d.enabled {
                                            span { class: "{ui::C_SUCCESS}", "{LBL_STATUS_ENABLED}" }
                                        } else {
                                            span { class: "{ui::C_MUTED}", "{LBL_STATUS_DISABLED}" }
                                        }
                                    }
                                    td { class: "px-2 py-1 text-right",
                                        button {
                                            class: "mr-2 rounded-lg border border-border px-2 py-0.5 {ui::TYPE_DESC} hover:bg-secondary",
                                            "data-testid": edit_id,
                                            onclick: move |_| on_edit(d.clone()),
                                            "{BTN_EDIT}"
                                        }
                                        if d.enabled {
                                            button {
                                                class: "rounded-lg border border-destructive px-2 py-0.5 text-xs {ui::C_DANGER} hover:bg-destructive",
                                                "data-testid": disable_id,
                                                onclick: move |_| on_disable(disable_code.clone()),
                                                "{BTN_DISABLE}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
