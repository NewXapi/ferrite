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
//!
//! 边界:文案常量在根级 `shared`;列表与表单的视觉细节分别在 `components/currency_list` / `currency_form`;
//! 本文件负责全部网络进出(拉取、upsert)与跨组件状态。

use crate::api::{CurrencyView, list_currencies_api, upsert_currency_api};
use crate::components::{CurrencyForm, CurrencyList};
use crate::shared::{
    BTN_RETRY, Kind, LBL_PAGE, MSG_ERR_CODE_REQUIRED, MSG_ERR_FIAT_PRECISION, MSG_ERR_FIAT_SYMBOL,
    MSG_ERR_RATE_POSITIVE, MSG_LOAD_FAILED_PREFIX, MSG_OK_CREATED_SUFFIX, MSG_OK_DISABLED_SUFFIX,
    MSG_OK_UPDATED_SUFFIX, SEC_NOTE,
};
use client::ApiClient;
use dioxus::prelude::*;

/// 货币管理面板：`currency_defs` 的列表 / 新增 / 编辑 / 软禁用。
///
/// 【是什么】货币 tab 的页面入口组件:标题 + 说明条 + 提示条 + 列表区 + 表单区。
///
/// 【做什么】负责列表拉取(GET `/api/currency`)、表单状态派生、三种写回(新增/编辑
/// 走 upsert、停用走 `enabled=false` upsert)与页面级三条提示(err / action_err /
/// ok_msg);不负责任何视觉细节 —— 渲染全部交给 `list` / `form` 两个子模块。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 首屏/`reload` 变化 → `use_effect` 触发 `list_currencies_api`,结果写 `defs`。
/// - 点「重试」→ `reload + 1` 重跑 effect。
/// - 点行内「编辑」→ `start_edit` 把整行回填 `f_*` 并置 `editing = Some(code)`(无网络)。
/// - 点行内「停用」→ `disable(code)` 从 `defs` 取该行原样回写 `enabled=false`
///   (PUT `/api/currency`),成功置 `ok_msg` 并 `reload + 1`。
/// - 点表单提交 → `submit` 先本地校验(code 必填 / fiat 需符号 / fiat precision ≥ 1 /
///   新增时 rate > 0),通过后 PUT `/api/currency`;成功清空八格、置 `ok_msg`、
///   `reload + 1`,失败写 `action_err`。
/// - 点「取消（转新增）」→ `start_create` 清空八格回到新增态(无网络)。
///
/// 【样式】顶层 `div.space-y-4`;说明条 `text-sm text-zinc-500`;三条提示分别用
/// `border-red-800 bg-red-950/40`(加载失败)、`border-red-800 bg-red-950/40`(动作错误)、
/// `border-emerald-800 bg-emerald-950/40`(成功)。页面自身不写列表/表单样式。
///
/// 【子组件组成】`CurrencyList`(列表四态 + 行操作)、`CurrencyForm`(录入表单)。
///
/// 【数据流】
/// - 对内(入):无 prop —— 页面组件不接收外部参数。
/// - 对外(出):把 `defs` / `loading` / `err` 与两个回调交给 `CurrencyList`,
///   把 `editing` 与八个 `f_*` Signal 交给 `CurrencyForm`;子组件的 `on_edit` /
///   `on_disable` / `on_submit` / `on_cancel` 闭包全部落回本文件的写回逻辑。
#[component]
pub fn CurrencyPage() -> Element {
    // —— 列表状态 ——
    // 跨 effect 与 list 区共享(拉取写入、四态渲染读取),故放页面层持有。
    let mut defs = use_signal(Vec::<CurrencyView>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // reload 计数:自增即重跑 effect(首屏 / 重试 / 每次写回成功后)
    let mut reload = use_signal(|| 0u32);

    // —— 页面级提示状态 ——
    // action_err 与 ok_msg 由 submit / disable 两个写回闭包写、由页面顶部提示条读,
    // 属于页面自己的反馈通道(与列表的 err 分开,避免写操作的成败覆盖列表四态)。
    let mut action_err = use_signal(|| None::<String>);
    let mut ok_msg = use_signal(|| None::<String>);

    // —— 表单状态 ——
    // 这组状态只服务表单本体,但由三个跨组件动作成组操作:页面的 start_edit /
    // start_create 要整组重置,submit 校验与写回也要整组读,故不能下沉进
    // CurrencyForm 内部,统一提升到页面层,以 Signal prop 注入(组件内零 use_signal)。
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
                action_err.set(Some(MSG_ERR_CODE_REQUIRED.into()));
                return;
            }
            if kind == Kind::Fiat {
                if f_symbol().trim().is_empty() {
                    action_err.set(Some(MSG_ERR_FIAT_SYMBOL.into()));
                    return;
                }
                if precision < 1 {
                    action_err.set(Some(MSG_ERR_FIAT_PRECISION.into()));
                    return;
                }
            }
            if !is_edit && rate.is_finite() && rate <= 0.0 {
                action_err.set(Some(MSG_ERR_RATE_POSITIVE.into()));
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
                        format!("{code}{MSG_OK_UPDATED_SUFFIX}")
                    } else {
                        format!("{code}{MSG_OK_CREATED_SUFFIX}")
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

    let start_edit = move |d: CurrencyView| {
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
                    ok_msg.set(Some(format!("{code}{MSG_OK_DISABLED_SUFFIX}")));
                    reload.set(reload() + 1);
                }
                Err(e) => action_err.set(Some(e.to_string())),
            }
        });
    };

    rsx! {
        div { class: "space-y-4",
            role: "region",
            "aria-label": LBL_PAGE,
            "data-testid": "currency-page",
            h2 { class: "text-lg font-semibold text-zinc-100", "{LBL_PAGE}" }
            p { class: "text-sm text-zinc-500",
                "{SEC_NOTE}"
            }

            // ---------- 错误 / 成功提示 ----------
            if let Some(e) = err() {
                div { class: "rounded-xl border border-red-800 bg-red-950/40 p-4 text-sm text-red-300",
                    "{MSG_LOAD_FAILED_PREFIX}{e}"
                    button {
                        class: "ml-3 rounded-lg border border-red-700 px-2 py-1 text-red-200 hover:bg-red-900/60",
                        "data-testid": "currency-retry",
                        onclick: move |_| reload.set(reload() + 1),
                        "{BTN_RETRY}"
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
                on_edit: start_edit,
                on_disable: disable,
                on_retry: move |_| reload.set(reload() + 1),
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
