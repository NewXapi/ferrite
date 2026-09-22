use leptos::prelude::*;
use singlestage::*;
use crate::ui::{CardGrid, Card, Dialog, DialogTrigger, CardHeader, CardTitle, CardContent};

#[derive(Clone)]
struct UsageLog {
    id: &'static str,
    model_name: &'static str,
    token_name: &'static str,
    channel_name: &'static str,
    created_at: &'static str,
    prompt_tokens: i64,
    completion_tokens: i64,
    use_time_ms: i64,
    quota: i64,
    is_stream: bool,
    ip: &'static str,
    request_id: &'static str,
}


use super::card::UsageCard;
use super::data::*;

#[component]
pub fn UsagePage() -> impl IntoView {
    let mut detail_modal_open = RwSignal::new(false);
    let mut selected_log = use_signal(|| None::<UsageLog>);

    let mut stats_requests = 0i64;
    let mut stats_quota = 0i64;
    let mut stats_rpm = 0i64;
    let mut stats_tpm = 0i64;

    let mut logs = use_signal(Vec::<UsageLog>::new);
    let mut page = use_signal(|| 0i64);
    let mut total = use_signal(|| 0i64);
    let mut loaded = use_signal(|| false);
    let mut load_err = use_signal(String::new);

    let logs_list = logs();
    let shown_len = logs_list.len();
    let total_len = total() as usize;

    rsx! {
        div { class: "flex flex-col gap-6",
            // 统计卡片区
            section { class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "用量统计" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    Card {
                        class: "px-5 py-4",
                        div { class: "text-2xl font-mono font-semibold text-zinc-100", "{stats_requests}" }
                        div { class: "text-sm text-zinc-400", "今日请求" }
                    }
                    Card {
                        class: "px-5 py-4",
                        div { class: "text-2xl font-mono font-semibold text-zinc-100", "{stats_quota}" }
                        div { class: "text-sm text-zinc-400", "今日消耗 (额度单位)" }
                    }
                    Card {
                        class: "px-5 py-4",
                        div { class: "text-2xl font-mono font-semibold text-zinc-100", "{stats_rpm}" }
                        div { class: "text-sm text-zinc-400", "RPM (近 60s)" }
                    }
                    Card {
                        class: "px-5 py-4",
                        div { class: "text-2xl font-mono font-semibold text-zinc-100", "{stats_tpm}" }
                        div { class: "text-sm text-zinc-400", "TPM (近 60s)" }
                    }
                    Card {
                        class: "px-5 py-4",
                        div { class: "text-2xl font-mono font-semibold text-zinc-100", "—" }
                        div { class: "text-sm text-zinc-400", "成功率 (暂无数据)" }
                    }
                }
            }

            // 日志列表区域
            section { class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "请求日志" }
                if !load_err().is_empty() {
                    p { class: "text-sm text-amber-400", "无法加载日志 (未登录或请求失败): {load_err()}" }
                } else if !loaded() {
                    p { class: "text-sm text-zinc-500", "加载中…" }
                } else if shown_len == 0 {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "没有找到匹配的日志记录" }
                    }
                } else {
                    CardGrid { aria_label: "日志列表".to_string(),
                        for log in logs_list {
                            UsageCard {
                                entry: log,
                            }
                        }
                    }
                }
            }

            if !load_err().is_empty() {
                // 错误态不再提供「加载更多」
            } else if shown_len < total_len {
                div { class: "flex justify-center pt-4",
                    Button {
                        button_type: "button",
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Lg,
                        onclick: move |_| {},
                        "加载更多"
                    }
                }
            } else if shown_len > 0 {
                div { class: "text-center text-xs text-zinc-500 py-6",
                    "已显示全部日志"
                }
            }

            // 详情弹窗
            if let Some(log) = selected_log() {
                Dialog {
                    dialog_trigger: DialogTrigger::None,
                    open: detail_modal_open,
                    title: "日志详情".to_string(),
                    div { class: "space-y-3",
                        div { class: "flex justify-between",
                            span { class: "text-zinc-500", "模型" }
                            span { class: "font-mono text-zinc-200", "{log.model_name}" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-zinc-500", "时间" }
                            span { class: "font-mono text-zinc-200", "{log.created_at}" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-zinc-500", "密钥" }
                            span { class: "font-mono text-zinc-200", "{log.token_name}" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-zinc-500", "渠道" }
                            span { class: "font-mono text-zinc-200", "{log.channel_name}" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-zinc-500", "Tokens(提示/补全)" }
                            span { class: "font-mono text-zinc-200", "{fmt_num(log.prompt_tokens)} / {fmt_num(log.completion_tokens)}" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-zinc-500", "耗时" }
                            span { class: "font-mono text-zinc-200", "{if log.use_time_ms > 0 { format!("{:.1}s", log.use_time_ms as f64 / 1000.0) } else { "—".to_string() }}" }
                        }
                        div { class: "flex justify-between",
                            span { class: "text-zinc-500", "流式" }
                            span { class: "font-mono text-zinc-200", "{if log.is_stream { "是".to_string() } else { "否".to_string() }}" }
                        }
                    }
                }
            }
        }
    }
}