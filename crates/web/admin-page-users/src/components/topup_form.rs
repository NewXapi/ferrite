//! 额度充值弹窗:金额(元) → 内部额度增量,实时展示充值后额度。
//!
//! 换算口径与卡片进度条同源(`data::{cny_to_quota, fmt_cny}`),保证卡片上
//! 看到的额度和这里预览的「充值后额度」是同一把尺子。

use dioxus::prelude::*;

use crate::format::{cny_to_quota, fmt_cny};

use super::modal::{MODAL_INPUT, Modal};
use crate::shared::{
    BTN_CANCEL, BTN_TOPUP_CONFIRM, FIELD_TOPUP_AMOUNT, LBL_QUOTA, MSG_AFTER_QUOTA, MSG_CUR_QUOTA,
    MSG_QUOTA_HINT, TTL_TOPUP,
};

/// 额度充值弹窗。
///
/// 【是什么】把人民币金额换算成内部额度增量的充值浮窗,实时预览充值后额度。
///
/// 【做什么】金额输入 + 合法性校验(>0)+ 换算预览;不做网络请求(确认后经
/// on_submit 抛回页面执行)。
///
/// 【交互逻辑】输入金额 → 实时换算增量与「充值后额度」;确认 →
/// `on_submit((user_key, delta_quota))`;取消 → `on_cancel`。
///
/// 【样式】`Modal` 外壳;当前/充值后两行对照,充值后为 emerald;金额输入 `font-mono`。
///
/// 【子组件组成】`Modal`、`MODAL_INPUT` 样式输入框。
///
/// 【数据流】
/// - 对内(入):`user_key`(操作目标)、`current_quota`(换算预览基准)。
/// - 对外(出):`on_submit((key, 增量额度))` / `on_cancel`。
///   换算口径与卡片进度条同源(`format::{cny_to_quota, fmt_cny}`)。
#[component]
pub fn TopUpForm(
    user_key: String,
    current_quota: i64,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<(String, i64)>,
) -> Element {
    let mut amount = use_signal(|| "50".to_string());
    let parsed = amount().trim().parse::<f64>().ok().filter(|v| *v > 0.0);
    // 充值金额(元) → 内部额度增量;展示充值后额度
    let delta_quota = parsed.map(|v| cny_to_quota(v).max(0));
    let after = delta_quota
        .map(|d| fmt_cny(current_quota + d))
        .unwrap_or_else(|| fmt_cny(current_quota));

    rsx! {
        Modal { title: TTL_TOPUP.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 {ui::TYPE_DESC}",
                    div { class: "flex justify-between gap-2",
                        span { class: "{ui::C_MUTED}", "{MSG_CUR_QUOTA}{LBL_QUOTA}" }
                        span { class: "font-medium text-zinc-200", "{fmt_cny(current_quota)}" }
                    }
                }
                div {
                    label { class: "mb-1.5 block {ui::TYPE_DESC}", "{FIELD_TOPUP_AMOUNT}" }
                    input {
                        class: "{MODAL_INPUT} font-mono",
                        r#type: "text",
                        value: "{amount}",
                        oninput: move |e| amount.set(e.value()),
                    }
                    p { class: "mt-1 {ui::TYPE_DESC}", "{MSG_QUOTA_HINT} {fmt_cny(delta_quota.unwrap_or(0))}" }
                }
                div { class: "flex justify-between gap-2 {ui::TYPE_DESC}",
                    span { class: "{ui::C_MUTED}", "{MSG_AFTER_QUOTA}{LBL_QUOTA}" }
                    span { class: "font-medium {ui::C_SUCCESS}", "{after}" }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 {ui::TYPE_BODY} transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    {BTN_CANCEL}
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 {ui::TYPE_CARD_TITLE} transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    disabled: parsed.is_none(),
                    onclick: move |_| on_submit.call((user_key.clone(), delta_quota.unwrap_or(0))),
                    "{BTN_TOPUP_CONFIRM}"
                }
            }
        }
    }
}
