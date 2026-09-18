//! 货币表单区：新增 / 编辑的录入行 + 提交/取消。
//! 纯展示组件：表单状态以 `Signal` 注入（Signal 是可拷贝的全局句柄），
//! 校验与写回逻辑留在 `page` 的 `on_submit` 里；
//! 成功/错误提示由页面统一渲染（与列表/重试提示同列）。

use dioxus::prelude::*;

use super::shared::{SEC_FORM, Kind};

/// 货币新增/编辑表单。
///
/// - `editing`：`None` = 新增；`Some(code)` = 编辑（code 输入框禁用）。
/// - `on_submit`：提交按钮（含必填/fiat 符号/precision/正汇率校验，见 page）。
/// - `on_cancel`：「取消（转新增）」——仅在编辑态出现。
#[component]
pub fn CurrencyForm(
    editing: Option<String>,
    f_code: Signal<String>,
    f_name: Signal<String>,
    f_symbol: Signal<String>,
    f_kind: Signal<Kind>,
    f_rate: Signal<String>,
    f_precision: Signal<String>,
    f_enabled: Signal<bool>,
    f_remark: Signal<String>,
    on_submit: EventHandler<MouseEvent>,
    on_cancel: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        section { class: "space-y-3 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
            role: "region",
            "aria-label": "货币表单",
            "data-testid": "currency-form-section",
            h3 { class: "text-sm font-semibold text-zinc-300",
                if let Some(c) = editing.as_ref() { "编辑货币 {c}" } else { "{SEC_FORM}" }
            }
            div { class: "grid grid-cols-2 gap-3 md:grid-cols-4",
                label { class: "space-y-1 text-xs text-zinc-400",
                    "Code"
                    input {
                        class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-200",
                        "data-testid": "currency-code-input",
                        value: "{f_code()}",
                        disabled: editing.is_some(),
                        oninput: move |e| f_code.set(e.value()),
                    }
                }
                label { class: "space-y-1 text-xs text-zinc-400",
                    "名称"
                    input {
                        class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-200",
                        "data-testid": "currency-name-input",
                        value: "{f_name()}",
                        oninput: move |e| f_name.set(e.value()),
                    }
                }
                label { class: "space-y-1 text-xs text-zinc-400",
                    "符号（如 ¥ / $ / P）"
                    input {
                        class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-200",
                        "data-testid": "currency-symbol-input",
                        value: "{f_symbol()}",
                        oninput: move |e| f_symbol.set(e.value()),
                    }
                }
                label { class: "space-y-1 text-xs text-zinc-400",
                    "kind"
                    select {
                        class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-200",
                        "data-testid": "currency-kind-select",
                        value: "{f_kind().as_str()}",
                        onchange: move |e| f_kind.set(Kind::parse(&e.value())),
                        option { value: "points", "points（余额货币）" }
                        option { value: "fiat", "fiat（仅计价展示）" }
                    }
                }
                label { class: "space-y-1 text-xs text-zinc-400",
                    "汇率（1 单位 = 多少内部单位，500_000 = $1）"
                    if f_code() == "USD" {
                        div { class: "space-y-1",
                            input {
                                class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-500",
                                "data-testid": "currency-rate-input",
                                value: "1",
                                disabled: true,
                            }
                            p { class: "text-xs text-amber-400/90", "USD 是基准货币，汇率恒为 1，不可修改" }
                        }
                    } else {
                        input {
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-200",
                            "data-testid": "currency-rate-input",
                            value: "{f_rate()}",
                            oninput: move |e| f_rate.set(e.value()),
                        }
                    }
                }
                label { class: "space-y-1 text-xs text-zinc-400",
                    "小数位（precision）"
                    input {
                        class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-200",
                        "data-testid": "currency-precision-input",
                        value: "{f_precision()}",
                        oninput: move |e| f_precision.set(e.value()),
                    }
                }
                label { class: "flex items-end space-x-2 pb-1 text-xs text-zinc-400",
                    input {
                        r#type: "checkbox",
                        "data-testid": "currency-enabled-check",
                        checked: f_enabled(),
                        onchange: move |e| f_enabled.set(e.checked()),
                    }
                    "启用"
                }
                label { class: "space-y-1 text-xs text-zinc-400 md:col-span-2",
                    "备注"
                    input {
                        class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-200",
                        "data-testid": "currency-remark-input",
                        value: "{f_remark()}",
                        oninput: move |e| f_remark.set(e.value()),
                    }
                }
            }

            // 维护者定稿的两条警示
            div { class: "space-y-1 text-xs",
                p { class: "text-red-400",
                    "修改汇率会实时影响全体用户可用额度（历史交易不锁汇率）。"
                }
                p { class: "text-red-400/80",
                    "「删除」即软禁用：余额非零的货币不可物理删除，仅可停用。"
                }
            }

            div { class: "flex space-x-2",
                button {
                    class: "rounded-lg bg-sky-700 px-3 py-1.5 text-sm text-white hover:bg-sky-600",
                    "data-testid": "currency-submit",
                    onclick: on_submit,
                    if editing.is_some() { "保存修改" } else { "创建货币" }
                }
                if editing.is_some() {
                    button {
                        class: "rounded-lg border border-zinc-700 px-3 py-1.5 text-sm text-zinc-300 hover:bg-zinc-800",
                        "data-testid": "currency-cancel",
                        onclick: on_cancel,
                        "取消（转新增）"
                    }
                }
            }
        }
    }
}
