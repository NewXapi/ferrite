//! 货币管理面板：`currency_defs` 的列表 / 新增 / 编辑 / 软禁用。
//!
//! 数据来自真实后端 `/api/currency`（admin bearer）。语义对齐 0014 换算层：
//! - `kind`: `points` = 可扣费余额货币；`fiat` = 仅计价展示（不进余额）
//! - `internal_rate`: 1 该货币单位 = 多少内部单位（500_000 = $1 基准）
//! - USD 是基准货币，rate 恒为 1（后端锁定，前端禁用输入）
//! - 「删除」即软禁用（enabled=false 提交）：余额非零的货币不可物理删，
//!   这是维护者定稿的语义（删按钮=停用）。
//!
//! 四态渲染（loading / error / empty / data）与 data-testid 对齐
//! RedemptionsPage / RewardsPanel 惯例；交互元素带 `data-testid`，容器带
//! `role` + `aria-label`（仓库 UI 验证约定，PR smoke 走 ariaSnapshot）。

use dioxus::prelude::*;

use crate::api::{CurrencyView, list_currencies_api, upsert_currency_api};
use client::ApiClient;

const SEC_LIST: &str = "货币列表";
const SEC_FORM: &str = "新增 / 编辑货币";

#[derive(Clone, PartialEq)]
enum Kind {
    Points,
    Fiat,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Points => "points",
            Kind::Fiat => "fiat",
        }
    }
    fn parse(s: &str) -> Self {
        if s == "fiat" {
            Kind::Fiat
        } else {
            Kind::Points
        }
    }
}

#[component]
pub fn CurrencyPage() -> Element {
    let mut defs = use_signal(Vec::<CurrencyView>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut action_err = use_signal(|| None::<String>);
    let mut ok_msg = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    // None = 新增；Some(code) = 编辑该货币
    let mut editing = use_signal(|| None::<String>);
    let mut f_code = use_signal(String::new);
    let mut f_name = use_signal(String::new);
    let mut f_symbol = use_signal(String::new);
    let mut f_kind = use_signal(|| Kind::Points);
    let mut f_rate = use_signal(|| "1".to_string());
    let mut f_precision = use_signal(|| "0".to_string());
    let mut f_enabled = use_signal(|| true);
    let mut f_remark = use_signal(String::new);

    // 拉取（挂载 / 刷新 / 写回后）
    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_currencies_api(&client).await {
                Ok(items) => {
                    defs.set(items);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let submit = move |_| {
        let code = f_code().trim().to_string();
        let rate: f64 = f_rate().parse().unwrap_or(0.0);
        let precision: i16 = f_precision().parse().unwrap_or(0);
        let kind = f_kind();
        let is_edit = editing().is_some();
        spawn(async move {
            action_err.set(None);
            ok_msg.set(None);
            if code.is_empty() {
                action_err.set(Some("code 必填".into()));
                return;
            }
            if kind == Kind::Fiat {
                if f_symbol().trim().is_empty() {
                    action_err.set(Some("fiat 货币必须配展示符号（如 ¥ / $）".into()));
                    return;
                }
                if precision < 1 {
                    action_err.set(Some("fiat 货币 precision 必须 ≥ 1".into()));
                    return;
                }
            }
            if !is_edit && rate.is_finite() && rate <= 0.0 {
                action_err.set(Some("internal_rate 必须为正数".into()));
                return;
            }
            let client = ApiClient::shared().clone();
            let req = CurrencyView {
                code: code.clone(),
                name: f_name().trim().to_string(),
                internal_rate: rate,
                enabled: f_enabled(),
                remark: f_remark().trim().to_string(),
                symbol: f_symbol().trim().to_string(),
                kind: kind.as_str().to_string(),
                precision,
            };
            match upsert_currency_api(&client, &req).await {
                Ok(_) => {
                    ok_msg.set(Some(if is_edit {
                        format!("{code} 已更新")
                    } else {
                        format!("{code} 已创建")
                    }));
                    editing.set(None);
                    f_code.set(String::new());
                    f_name.set(String::new());
                    f_symbol.set(String::new());
                    f_kind.set(Kind::Points);
                    f_rate.set("1".into());
                    f_precision.set("0".into());
                    f_enabled.set(true);
                    f_remark.set(String::new());
                    reload.set(reload() + 1);
                }
                Err(e) => action_err.set(Some(e.to_string())),
            }
        });
    };

    let mut start_edit = move |d: CurrencyView| {
        editing.set(Some(d.code.clone()));
        f_code.set(d.code.clone());
        f_name.set(d.name.clone());
        f_symbol.set(d.symbol.clone());
        f_kind.set(Kind::parse(&d.kind));
        f_rate.set(format!("{}", d.internal_rate));
        f_precision.set(format!("{}", d.precision));
        f_enabled.set(d.enabled);
        f_remark.set(d.remark.clone());
        action_err.set(None);
        ok_msg.set(None);
    };

    let start_create = move |_| {
        editing.set(None);
        f_code.set(String::new());
        f_name.set(String::new());
        f_symbol.set(String::new());
        f_kind.set(Kind::Points);
        f_rate.set("1".into());
        f_precision.set("0".into());
        f_enabled.set(true);
        f_remark.set(String::new());
        action_err.set(None);
        ok_msg.set(None);
    };

    let disable = move |code: String| {
        spawn(async move {
            action_err.set(None);
            ok_msg.set(None);
            let client = ApiClient::shared().clone();
            // 软禁用 = 按内存里的定义原样回写 enabled=false（code 为 PK，存在即覆盖）。
            let Some(d) = defs().iter().find(|d| d.code == code).cloned() else {
                return;
            };
            let req = CurrencyView {
                enabled: false,
                ..d
            };
            match upsert_currency_api(&client, &req).await {
                Ok(_) => {
                    ok_msg.set(Some(format!("{code} 已停用")));
                    reload.set(reload() + 1);
                }
                Err(e) => action_err.set(Some(e.to_string())),
            }
        });
    };

    // (edit_id, disable_id, disable_code, view) 预构建并预格式化 testid：
    // rsx 属性全走 move、零借用，绕开宏展开里借用/移动顺序不可靠的问题。
    let rows: Vec<(String, String, String, CurrencyView)> = defs()
        .into_iter()
        .map(|d| {
            let edit_id = format!("currency-edit-{}", d.code);
            let disable_id = format!("currency-disable-{}", d.code);
            let disable_code = d.code.clone();
            (edit_id, disable_id, disable_code, d)
        })
        .collect();

    rsx! {
        div { class: "space-y-4",
            role: "region",
            "aria-label": "货币管理",
            "data-testid": "currency-page",
            h2 { class: "text-lg font-semibold text-zinc-100", "货币管理" }
            p { class: "text-sm text-zinc-500",
                "kind=points 的货币进钱包余额并可扣费；kind=fiat 仅作计价/展示（不进余额）。基准 USD 的汇率恒为 1。"
            }

            // ---------- 错误 / 成功提示 ----------
            if let Some(e) = err() {
                div { class: "rounded-xl border border-red-800 bg-red-950/40 p-4 text-sm text-red-300",
                    "数据加载失败：{e}"
                    button {
                        class: "ml-3 rounded-lg border border-red-700 px-2 py-1 text-red-200 hover:bg-red-900/60",
                        "data-testid": "currency-retry",
                        onclick: move |_| reload.set(reload() + 1),
                        "重试"
                    }
                }
            }
            if let Some(e) = action_err() {
                div { class: "rounded-xl border border-red-800 bg-red-950/40 p-3 text-sm text-red-300",
                    "data-testid": "currency-action-error",
                    "{e}"
                }
            }
            if let Some(m) = ok_msg() {
                div { class: "rounded-xl border border-emerald-800 bg-emerald-950/40 p-3 text-sm text-emerald-300",
                    "data-testid": "currency-action-ok",
                    "{m}"
                }
            }

            // ---------- 列表 ----------
            section { class: "space-y-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
                role: "region",
                "aria-label": "货币定义列表",
                "data-testid": "currency-list-section",
                h3 { class: "text-sm font-semibold text-zinc-300", "{SEC_LIST}" }
                if loading() {
                    div { class: "space-y-2",
                        for _i in 0..3 {
                            div { class: "h-8 animate-pulse rounded-lg bg-zinc-800/60" }
                        }
                    }
                } else if defs().is_empty() && err().is_none() {
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
                                                onclick: move |_| start_edit(d.clone()),
                                                "编辑"
                                            }
                                            if d.enabled {
                                                button {
                                                    class: "rounded-lg border border-red-800 px-2 py-0.5 text-xs text-red-300 hover:bg-red-900/40",
                                                    "data-testid": disable_id,
                                                    onclick: move |_| disable(disable_code.clone()),
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

            // ---------- 表单 ----------
            section { class: "space-y-3 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
                role: "region",
                "aria-label": "货币表单",
                "data-testid": "currency-form-section",
                h3 { class: "text-sm font-semibold text-zinc-300",
                    if let Some(c) = editing() { "编辑货币 {c}" } else { "{SEC_FORM}" }
                }
                div { class: "grid grid-cols-2 gap-3 md:grid-cols-4",
                    label { class: "space-y-1 text-xs text-zinc-400",
                        "Code"
                        input {
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-200",
                            "data-testid": "currency-code-input",
                            value: "{f_code()}",
                            disabled: editing().is_some(),
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
                            input {
                                class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/60 px-2 py-1 text-sm text-zinc-500",
                                "data-testid": "currency-rate-input",
                                value: "1",
                                disabled: true,
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
                        onclick: submit,
                        if editing().is_some() { "保存修改" } else { "创建货币" }
                    }
                    if editing().is_some() {
                        button {
                            class: "rounded-lg border border-zinc-700 px-3 py-1.5 text-sm text-zinc-300 hover:bg-zinc-800",
                            "data-testid": "currency-cancel",
                            onclick: start_create,
                            "取消（转新增）"
                        }
                    }
                }
            }
        }
    }
}
