//! 日志详情弹窗:居中模态,手机近全宽;点遮罩或 × 关闭。

use contract::api::usage::UsageLogDto;
use dioxus::prelude::*;

use crate::usage_support::{fmt_num, fmt_quota, fmt_time_full};

#[component]
pub fn LogDetailModal(log: UsageLogDto, on_close: EventHandler<()>) -> Element {
    let time_str = fmt_time_full(&log.created_at);
    let tokens_pair = format!(
        "{} / {}",
        fmt_num(log.prompt_tokens as i64),
        fmt_num(log.completion_tokens as i64)
    );
    let timing_str = if log.use_time_ms > 0 {
        format!("{:.1}s", log.use_time_ms as f64 / 1000.0)
    } else {
        "—".to_string()
    };
    let tps_str = if log.use_time_ms > 0 && log.completion_tokens > 0 {
        format!(
            "{:.0} t/s",
            log.completion_tokens as f64 / (log.use_time_ms as f64 / 1000.0)
        )
    } else {
        "—".to_string()
    };
    let cost_str = fmt_quota(log.quota);

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-4 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "日志详情" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                div { class: "space-y-2.5 text-sm",
                    DetailRow { label: "模型", value: log.model_name.clone() }
                    DetailRow { label: "时间", value: time_str }
                    DetailRow { label: "密钥", value: log.token_name.clone() }
                    DetailRow { label: "渠道", value: log.channel_name.clone() }
                    DetailRow { label: "Tokens(提示/补全)", value: tokens_pair }
                    DetailRow { label: "耗时", value: timing_str }
                    DetailRow { label: "速度", value: tps_str }
                    DetailRow { label: "消耗(估)", value: cost_str }
                    DetailRow { label: "流式", value: if log.is_stream { "是".to_string() } else { "否".to_string() } }
                    DetailRow { label: "IP", value: log.ip.clone() }
                    DetailRow { label: "请求 ID", value: log.request_id.clone() }
                }
            }
        }
    }
}

#[component]
fn DetailRow(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex justify-between gap-2",
            span { class: "shrink-0 text-zinc-500", "{label}" }
            span { class: "min-w-0 break-all text-right font-mono text-zinc-200", "{value}" }
        }
    }
}
