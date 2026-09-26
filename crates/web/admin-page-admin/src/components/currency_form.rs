//! 货币表单区：新增 / 编辑的录入行 + 提交/取消。
//! 纯展示组件：表单状态以 `Signal` 注入（Signal 是可拷贝的全局句柄），
//! 校验与写回逻辑留在 `tab-page/currency.rs` 的 `on_submit` 里；
//! 成功/错误提示由页面统一渲染（与列表/重试提示同列）。
//!
//! 边界:字段布局与警示文案在本文件;填写规则(必填 / fiat 符号 / precision /
//! 正汇率)的校验不在本文件,由页面 `submit` 闭包执行;文案常量见根级 `crate::shared`。

use dioxus::prelude::*;

use crate::shared::{
    BTN_CANCEL, BTN_CREATE, BTN_SAVE_CHANGES, FIELD_CODE, FIELD_ENABLED, FIELD_KIND,
    FIELD_PRECISION, FIELD_RATE, FIELD_SYMBOL, Kind, LBL_FIELD_NAME, LBL_FIELD_REMARK,
    LBL_FORM_REGION, MSG_USD_LOCKED, MSG_WARN_DISABLE, MSG_WARN_RATE, OPT_KIND_FIAT,
    OPT_KIND_POINTS, SEC_FORM,
};

/// 货币新增/编辑表单。
///
/// 【是什么】货币 tab 的表单区:标题随编辑态切换 + 八格录入网格 + 两条警示 + 提交按钮组。
///
/// 【做什么】渲染 Code/名称/符号/kind/汇率/小数位/启用/备注八个字段,并在编辑态把
/// Code 与 USD 汇率置为禁用;Code 为 `USD` 时汇率格改渲染只读 `1` 加锁定说明。
/// 不负责校验(页面的 `submit` 闭包)、不负责写回网络(页面的 `submit`/`disable`)、
/// 不负责成功与失败提示(页面统一渲染)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 在任一输入框输入 → 直接 `set` 对应 `Signal`(共享句柄,状态住在页面),无网络。
/// - kind 下拉切换 → `Kind::parse` 后写 `f_kind`;启用勾选 → 写 `f_enabled`。
/// - 点提交 → `on_submit`(MouseEvent)抛回页面,由页面读 Signal 做校验并发起
///   `upsert_currency_api`;点「取消（转新增）」→ `on_cancel` 抛回页面重置表单。
/// 本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 `section.space-y-3 rounded-xl border border-border bg-card/60 p-3`;
/// 录入网格 `grid grid-cols-2 gap-3 md:grid-cols-4`(备注格 `md:col-span-2`);输入框统一
/// `w-full rounded-lg border border-border bg-secondary/60 px-2 py-1`;两条警示 `text-xs`
/// 分别为 `text-destructive` 与 `text-destructive`;提交按钮 `bg-info hover:bg-info`。
///
/// 【子组件组成】无独立子组件,全部 rsx 在本文件内联(`label` / `input` / `select` /
/// `option` / `p` / `button`)。
///
/// 【数据流】
/// - 对内(入):`editing`(`None` = 新增,`Some(code)` = 编辑该货币,决定标题与禁用态)、
///   八个 `f_*` Signal(页面持有,本组件就地读写,是双向绑定而非单向入参)。
/// - 对外(出):`on_submit(MouseEvent)` → 页面 `submit` 校验并 PUT `/api/currency`;
///   `on_cancel(MouseEvent)` → 页面 `start_create` 清空八格并回到新增态。
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
    // USD 汇率恒为 1，后端锁定，前端只读展示，避免提交后被后端拒绝。
    let usd_locked = f_code() == "USD";

    rsx! {
        section { class: "space-y-3 rounded-xl border border-border bg-card/60 p-3",
            role: "region",
            "aria-label": LBL_FORM_REGION,
            "data-testid": "currency-form-section",
            h3 { class: "{ui::TYPE_CARD_TITLE}",
                if let Some(c) = editing.as_ref() { "编辑货币 {c}" } else { "{SEC_FORM}" }
            }
            div { class: "grid grid-cols-2 gap-3 md:grid-cols-4",
                label { class: "space-y-1 {ui::TYPE_DESC}",
                    "{FIELD_CODE}"
                    input {
                        class: "w-full rounded-lg border border-border bg-secondary/60 px-2 py-1 {ui::TYPE_BODY}",
                        "data-testid": "currency-code-input",
                        value: "{f_code()}",
                        disabled: editing.is_some(),
                        oninput: move |e| f_code.set(e.value()),
                    }
                }
                label { class: "space-y-1 {ui::TYPE_DESC}",
                    "{LBL_FIELD_NAME}"
                    input {
                        class: "w-full rounded-lg border border-border bg-secondary/60 px-2 py-1 {ui::TYPE_BODY}",
                        "data-testid": "currency-name-input",
                        value: "{f_name()}",
                        oninput: move |e| f_name.set(e.value()),
                    }
                }
                label { class: "space-y-1 {ui::TYPE_DESC}",
                    "{FIELD_SYMBOL}"
                    input {
                        class: "w-full rounded-lg border border-border bg-secondary/60 px-2 py-1 {ui::TYPE_BODY}",
                        "data-testid": "currency-symbol-input",
                        value: "{f_symbol()}",
                        oninput: move |e| f_symbol.set(e.value()),
                    }
                }
                label { class: "space-y-1 {ui::TYPE_DESC}",
                    "{FIELD_KIND}"
                    select {
                        class: "w-full rounded-lg border border-border bg-secondary/60 px-2 py-1 {ui::TYPE_BODY}",
                        "data-testid": "currency-kind-select",
                        value: "{f_kind().as_str()}",
                        onchange: move |e| f_kind.set(Kind::parse(&e.value())),
                        option { value: "points", "{OPT_KIND_POINTS}" }
                        option { value: "fiat", "{OPT_KIND_FIAT}" }
                    }
                }
                label { class: "space-y-1 {ui::TYPE_DESC}",
                    "{FIELD_RATE}"
                    if usd_locked {
                        div { class: "space-y-1",
                            input {
                                class: "w-full rounded-lg border border-border bg-secondary/60 px-2 py-1 {ui::TYPE_BODY}",
                                "data-testid": "currency-rate-input",
                                value: "1",
                                disabled: true,
                            }
                            p { class: "text-xs {ui::C_WARNING}", "{MSG_USD_LOCKED}" }
                        }
                    } else {
                        input {
                            class: "w-full rounded-lg border border-border bg-secondary/60 px-2 py-1 {ui::TYPE_BODY}",
                            "data-testid": "currency-rate-input",
                            value: "{f_rate()}",
                            oninput: move |e| f_rate.set(e.value()),
                        }
                    }
                }
                label { class: "space-y-1 {ui::TYPE_DESC}",
                    "{FIELD_PRECISION}"
                    input {
                        class: "w-full rounded-lg border border-border bg-secondary/60 px-2 py-1 {ui::TYPE_BODY}",
                        "data-testid": "currency-precision-input",
                        value: "{f_precision()}",
                        oninput: move |e| f_precision.set(e.value()),
                    }
                }
                label { class: "flex items-end space-x-2 pb-1 {ui::TYPE_DESC}",
                    input {
                        r#type: "checkbox",
                        "data-testid": "currency-enabled-check",
                        checked: f_enabled(),
                        onchange: move |e| f_enabled.set(e.checked()),
                    }
                    "{FIELD_ENABLED}"
                }
                label { class: "space-y-1 {ui::TYPE_DESC} md:col-span-2",
                    "{LBL_FIELD_REMARK}"
                    input {
                        class: "w-full rounded-lg border border-border bg-secondary/60 px-2 py-1 {ui::TYPE_BODY}",
                        "data-testid": "currency-remark-input",
                        value: "{f_remark()}",
                        oninput: move |e| f_remark.set(e.value()),
                    }
                }
            }

            // 维护者定稿的两条警示
            div { class: "space-y-1 {ui::TYPE_DESC}",
                p { class: "{ui::C_DANGER}",
                    "{MSG_WARN_RATE}"
                }
                p { class: "{ui::C_DANGER}",
                    "{MSG_WARN_DISABLE}"
                }
            }

            div { class: "flex space-x-2",
                button {
                    class: "rounded-lg bg-info px-3 py-1.5 {ui::TYPE_BODY} hover:bg-info",
                    "data-testid": "currency-submit",
                    onclick: on_submit,
                    if editing.is_some() { "{BTN_SAVE_CHANGES}" } else { "{BTN_CREATE}" }
                }
                if editing.is_some() {
                    button {
                        class: "rounded-lg border border-border px-3 py-1.5 {ui::TYPE_BODY} hover:bg-secondary",
                        "data-testid": "currency-cancel",
                        onclick: on_cancel,
                        "{BTN_CANCEL}"
                    }
                }
            }
        }
    }
}
