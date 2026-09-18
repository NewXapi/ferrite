//! 货币列表区：四态渲染（loading / error / empty / data）+ 表格行。
//! 纯展示组件：数据与写回调由 `page` 注入。

use dioxus::prelude::*;

use super::shared::SEC_LIST;
use crate::api::CurrencyView;

/// 货币定义列表。
///
/// - `defs` / `loading` / `err`：列表三态的数据源（三者由调用方在拉取后置入）。
/// - `on_edit`：点击「编辑」时回传整行，调用方据此回填表单。
/// - `on_disable`：点击「停用」时只回传 code（软禁用按内存定义原样回写）。
#[component]
pub fn CurrencyList(
    defs: Vec<CurrencyView>,
    loading: bool,
    err: Option<String>,
    on_edit: EventHandler<CurrencyView>,
    on_disable: EventHandler<String>,
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
        section { class: "space-y-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
            role: "region",
            "aria-label": "货币定义列表",
            "data-testid": "currency-list-section",
            h3 { class: "text-sm font-semibold text-zinc-300", "{SEC_LIST}" }
            if loading {
                div { class: "space-y-2",
                    for _i in 0..3 {
                        div { class: "h-8 animate-pulse rounded-lg bg-zinc-800/60" }
                    }
                }
            } else if rows.is_empty() && err.is_none() {
                div { class: "rounded-lg border border-dashed border-zinc-700 p-6 text-center text-sm text-zinc-500",
                    "还没有货币定义。用下方表单创建第一个货币。"
                }
            } else {
                div { class: "overflow-x-auto",
                    table { class: "w-full text-left text-sm",
                        "data-testid": "currency-list",
                        thead { class: "text-zinc-500",
                            tr {
                                th { class: "px-2 py-1", "Code" }
                                th { class: "px-2 py-1", "符号" }
                                th { class: "px-2 py-1", "名称" }
                                th { class: "px-2 py-1", "kind" }
                                th { class: "px-2 py-1", "汇率" }
                                th { class: "px-2 py-1", "小数位" }
                                th { class: "px-2 py-1", "状态" }
                                th { class: "px-2 py-1", "" }
                            }
                        }
                        tbody {
                            for (edit_id, disable_id, disable_code, d) in rows {
                                tr { class: "border-t border-zinc-800",
                                    td { class: "px-2 py-1 font-mono text-zinc-200", "{d.code}" }
                                    td { class: "px-2 py-1", "{d.symbol}" }
                                    td { class: "px-2 py-1 text-zinc-400", "{d.name}" }
                                    td { class: "px-2 py-1",
                                        span { class: if d.kind == "fiat" { "rounded bg-amber-900/40 px-1.5 py-0.5 text-xs text-amber-300" } else { "rounded bg-sky-900/40 px-1.5 py-0.5 text-xs text-sky-300" },
                                            "{d.kind}"
                                        }
                                    }
                                    td { class: "px-2 py-1 text-zinc-400", "{d.internal_rate}" }
                                    td { class: "px-2 py-1 text-zinc-400", "{d.precision}" }
                                    td { class: "px-2 py-1",
                                        if d.enabled {
                                            span { class: "text-emerald-400", "启用" }
                                        } else {
                                            span { class: "text-zinc-500", "停用" }
                                        }
                                    }
                                    td { class: "px-2 py-1 text-right",
                                        button {
                                            class: "mr-2 rounded-lg border border-zinc-700 px-2 py-0.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                            "data-testid": edit_id,
                                            onclick: move |_| on_edit(d.clone()),
                                            "编辑"
                                        }
                                        if d.enabled {
                                            button {
                                                class: "rounded-lg border border-red-800 px-2 py-0.5 text-xs text-red-300 hover:bg-red-900/40",
                                                "data-testid": disable_id,
                                                onclick: move |_| on_disable(disable_code.clone()),
                                                "停用"
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
