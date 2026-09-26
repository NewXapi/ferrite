//! 日志详情弹窗:居中模态,手机近全宽;点遮罩或 × 关闭。

use contract::api::usage::UsageLogDto;
use dioxus::prelude::*;

use crate::usage_support::{fmt_num, fmt_quota, fmt_time_full};

/// 【是什么】日志详情模态弹窗，逐字段展示单条请求日志的完整信息。
///
/// 【做什么】居中渲染模态：标题 + 关闭按钮 + 11 行 DetailRow (模型/时间/密钥/渠道/Tokens/耗时/速度/消耗/是否流式/IP/请求 ID)；速度由补全 token 数除以耗时推导，任一为 0 显示「—」。不取数。
///
/// 【交互逻辑】点遮罩或右上角 ✕ 触发 on_close；点弹窗主体 stop_propagation 防误关。
///
/// 【样式】遮罩 fixed inset-0 z-50 黑半透明 + backdrop-blur-sm；弹窗 max-w-md rounded-2xl 边框卡片 p-5 shadow-xl；行左灰标签右等宽值右对齐。
///
/// 【子组件组成】DetailRow × 11 (本文件私有行组件)
///
/// 【数据流】props 接收 log (UsageLogDto) 与 on_close (EventHandler<()>)；输出仅 on_close。
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

/// 详情弹窗内的一行「标签 : 值」；长值 break-all 换行，不撑破弹窗。
#[component]
fn DetailRow(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex justify-between gap-2",
            span { class: "shrink-0 text-zinc-500", "{label}" }
            span { class: "min-w-0 break-all text-right font-mono text-zinc-200", "{value}" }
        }
    }
}
