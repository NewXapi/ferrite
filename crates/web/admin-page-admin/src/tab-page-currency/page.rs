//! 货币管理面板：`currency_defs` 的列表 / 新增 / 编辑 / 软禁用。
//!
//! 数据来自真实后端 `/api/currency`（admin bearer）。语义对齐 0014 换算层：
//! - `kind`: `points` = 可扣费余额货币；`fiat` = 仅计价展示（不进余额）
//! - `internal_rate`: 1 该货币单位 = 多少内部单位（500_000 = $1 基准）
//! - USD 是基准货币，rate 恒为 1（后端锁定，前端禁用输入）
//! - 「删除」即软禁用（enabled=false 提交）：余额非零的货币不可物理删，
//!   这是维护者定稿的语义（删按钮=停用）。
//!
//! 本文件只保留状态与写回逻辑（拉取 / submit / start_edit / disable）；
//! 渲染拆成 `list`（列表四态）与 `form`
//! （录入表单）两个组件。四态约定与 data-testid 对齐 RedemptionsPage 惯例。

use dioxus::prelude::*;

use super::form::CurrencyForm;
use super::list::CurrencyList;
use super::shared::Kind;
use crate::api::{CurrencyView, list_currencies_api, upsert_currency_api};
use client::ApiClient;

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

            // 列表区:货币定义四态(加载/错误/空/表格)+ 每行编辑/停用按钮。
            // 纯渲染,defs 由页面拉取后传入;on_edit 回传整行回填表单,
            // on_disable 只回传 code(软禁用按内存定义原样回写),见 list.rs。
            CurrencyList {
                defs: defs(),
                loading: *loading.read(),
                err: err(),
                on_edit: move |d| start_edit(d),
                on_disable: move |code| disable(code),
            }

            // 表单区:新增/编辑录入(Code/名称/符号/kind/汇率/小数位/启用/备注)。
            // f_* 以 Signal 注入(Signal 可拷贝句柄),校验与写回逻辑留在页面的
            // submit 闭包;editing 区分新增/编辑态(code 编辑时禁用),见 form.rs。
            CurrencyForm {
                editing: editing(),
                f_code,
                f_name,
                f_symbol,
                f_kind,
                f_rate,
                f_precision,
                f_enabled,
                f_remark,
                on_submit: submit,
                on_cancel: start_create,
            }
        }
    }
}
