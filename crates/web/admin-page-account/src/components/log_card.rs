//! 日志卡片:模型色点 + 时间 + Tokens/耗时/消耗摘要,点击打开详情弹窗。

use contract::api::usage::UsageLogDto;
use dioxus::prelude::*;

use crate::usage_support::{fmt_num, fmt_quota};

/// 【是什么】单条请求日志卡片，展示模型、时间、Tokens、耗时与消耗摘要，整卡可点开详情。
///
/// 【做什么】渲染一条日志摘要：模型色点 + 模型名 (等宽截断)、本地化时间与消耗额、Tokens (提示/补全) 与耗时两行明细；模型色按名称映射固定色板，未知模型回退灰色，耗时为 0 显示「—」。不取数。
///
/// 【交互逻辑】整卡是 button，点击触发 on_open 并回传本条 log。
///
/// 【样式】整卡 w-full rounded-2xl 边框卡片 p-4 text-left + hover:border-zinc-500；模型名 font-mono truncate；消耗额 emerald-400 tabular-nums；明细 12px 灰字。
///
/// 【子组件组成】无 (纯 rsx，色值与文案在组件内派生)
///
/// 【数据流】props 接收 log (UsageLogDto) 与 on_open (EventHandler<UsageLogDto>)；输出仅 on_open。
#[component]
pub fn LogCard(log: UsageLogDto, on_open: EventHandler<UsageLogDto>) -> Element {
    let time_str = crate::usage_support::fmt_time(&log.created_at);
    let model_color = match log.model_name.as_str() {
        "gpt-4o" | "gpt-4o-mini" => "bg-emerald-400",
        "claude-3.5-sonnet" | "claude-3-haiku" => "bg-purple-400",
        "deepseek-r1" => "bg-blue-400",
        "qwen2.5-72b" => "bg-orange-400",
        _ => "bg-zinc-400",
    };
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
    let cost_str = fmt_quota(log.quota);

    rsx! {
        button {
            class: "w-full cursor-pointer rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4 text-left transition-colors hover:border-zinc-500 hover:bg-zinc-900",
            onclick: move |_| on_open.call(log.clone()),

            // 头部:模型 + 消耗
            div { class: "flex items-center gap-2",
                span { class: "h-2.5 w-2.5 shrink-0 rounded-full {model_color}" }
                span { class: "truncate font-mono text-sm text-zinc-200", "{log.model_name}" }
            }
            div { class: "mt-2 flex items-baseline justify-between gap-2",
                span { class: "font-mono {ui::TYPE_DESC}", "{time_str}" }
                span { class: "shrink-0 font-medium tabular-nums text-sm {ui::STATE_SUCCESS_TEXT}", "{cost_str}" }
            }

            // 摘要两行:Tokens、耗时
            div { class: "mt-3 space-y-1.5 text-xs",
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-500", "Tokens" }
                    span { class: "whitespace-nowrap font-medium tabular-nums text-zinc-200",
                        "{tokens_pair}"
                    }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-500", "耗时" }
                    span { class: "whitespace-nowrap tabular-nums text-zinc-400", "{timing_str}" }
                }
            }
        }
    }
}
