//! 额度充值弹窗:金额(元) → 内部额度增量,实时展示充值后额度。
//!
//! 换算口径与卡片进度条同源(`data::{cny_to_quota, fmt_cny}`),保证卡片上
//! 看到的额度和这里预览的「充值后额度」是同一把尺子。

use dioxus::prelude::*;

use crate::data::{cny_to_quota, fmt_cny};

use super::labels::{BTN_CANCEL, LBL_QUOTA};
use super::modal::{MODAL_INPUT, Modal};

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
        Modal { title: "额度充值".to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs",
                    div { class: "flex justify-between gap-2",
                        span { class: "text-zinc-400", "当前{LBL_QUOTA}" }
                        span { class: "font-medium text-zinc-200", "{fmt_cny(current_quota)}" }
                    }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "充值金额 (元)" }
                    input {
                        class: "{MODAL_INPUT} font-mono",
                        r#type: "text",
                        value: "{amount}",
                        oninput: move |e| amount.set(e.value()),
                    }
                    p { class: "mt-1 text-xs text-zinc-500", "折合 {fmt_cny(delta_quota.unwrap_or(0))}" }
                }
                div { class: "flex justify-between gap-2 text-xs",
                    span { class: "text-zinc-400", "充值后{LBL_QUOTA}" }
                    span { class: "font-medium text-emerald-400", "{after}" }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    {BTN_CANCEL}
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    disabled: parsed.is_none(),
                    onclick: move |_| on_submit.call((user_key.clone(), delta_quota.unwrap_or(0))),
                    "确认充值"
                }
            }
        }
    }
}
