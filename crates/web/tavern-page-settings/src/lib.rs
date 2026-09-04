//! tavern-page-settings — 连接与采样设置。
//!
//! 对齐 SillyTavern 双抽屉:
//! 1. **API 连接** (`#rm_api_block`):
//!    - API 类型(Chat Completion / Kobold / Novel)
//!    - 基础端点 URL
//!    - 密钥输入 + 显隐切换
//!    - 连接状态指示灯 +「测试连接」按钮
//! 2. **AI 响应与采样参数** (`#left-nav-panel`):
//!    - 预设切换 (Default / Creative / Precise)
//!    - 核心滑杆组: Temperature / Top P / Top K / Min P / Repetition Penalty
//!    - Max Tokens 限制
//!    - 流式响应 (Streaming) 开关
//!
//! 当前为壳内效果页,本地保存,待 client/secrets 接线。

use dioxus::prelude::*;
use tavern_ui::{Field, SliderField};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveSection {
    Connection,
    Sampling,
}

#[component]
pub fn SettingsPage() -> Element {
    let mut tab = use_signal(|| ActiveSection::Connection);

    // 连接表单状态
    let mut api_type = use_signal(|| "openai_chat".to_string());
    let mut endpoint = use_signal(|| "http://localhost:3000/v1".to_string());
    let mut api_key = use_signal(String::new);
    let mut show_key = use_signal(|| false);
    let mut connected = use_signal(|| false);
    let mut testing = use_signal(|| false);

    // 采样滑杆状态
    let mut preset = use_signal(|| "Default".to_string());
    let mut temperature = use_signal(|| 0.8_f64);
    let mut top_p = use_signal(|| 0.95_f64);
    let mut top_k = use_signal(|| 40.0_f64);
    let mut min_p = use_signal(|| 0.05_f64);
    let mut rep_penalty = use_signal(|| 1.1_f64);
    let mut max_tokens = use_signal(|| 1024_u32);
    let mut streaming = use_signal(|| true);

    let mut saved_hint = use_signal(|| false);

    rsx! {
        div { class: "flex h-full flex-col gap-4 overflow-hidden",
            // 顶部分段切换器 (对齐 ST 多抽屉概念)
            div { class: "flex shrink-0 items-center justify-between border-b border-zinc-800 pb-3",
                div { class: "flex items-center gap-1 rounded-full border border-zinc-800 bg-zinc-900/60 p-0.5",
                    button {
                        class: if tab() == ActiveSection::Connection {
                            "rounded-full bg-zinc-100 px-3.5 py-1 text-xs font-semibold text-zinc-900 transition-all"
                        } else {
                            "rounded-full px-3.5 py-1 text-xs font-medium text-zinc-400 hover:text-zinc-200"
                        },
                        onclick: move |_| tab.set(ActiveSection::Connection),
                        "API 连接"
                    }
                    button {
                        class: if tab() == ActiveSection::Sampling {
                            "rounded-full bg-zinc-100 px-3.5 py-1 text-xs font-semibold text-zinc-900 transition-all"
                        } else {
                            "rounded-full px-3.5 py-1 text-xs font-medium text-zinc-400 hover:text-zinc-200"
                        },
                        onclick: move |_| tab.set(ActiveSection::Sampling),
                        "采样与模型"
                    }
                }

                div { class: "flex items-center gap-3",
                    if saved_hint() {
                        span { class: "text-xs text-emerald-400", "✓ 设置已暂存到本地信号" }
                    }
                    button {
                        class: "rounded-full bg-zinc-100 px-4 py-1.5 text-xs font-semibold text-zinc-900 hover:bg-zinc-300",
                        onclick: move |_| saved_hint.set(true),
                        "保存配置"
                    }
                }
            }

            // 主滚动区
            div { class: "min-h-0 flex-1 overflow-y-auto pr-1 text-xs",
                match tab() {
                    ActiveSection::Connection => rsx! {
                        div { class: "flex max-w-2xl flex-col gap-5",
                            // 状态卡片
                            div { class: "flex items-center justify-between rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4",
                                div { class: "flex items-center gap-3",
                                    div {
                                        class: if connected() {
                                            "h-3 w-3 rounded-full bg-emerald-500 shadow-lg shadow-emerald-500/50"
                                        } else {
                                            "h-3 w-3 rounded-full bg-amber-500/80"
                                        },
                                    }
                                    div { class: "flex flex-col",
                                        span { class: "text-xs font-semibold text-zinc-200",
                                            if connected() { "后端已就绪 (HTTP 200)" } else { "未连接或待测试" }
                                        }
                                        span { class: "text-[11px] text-zinc-500", "代理端点: {endpoint()}" }
                                    }
                                }
                                button {
                                    class: "rounded-full border border-zinc-700 bg-zinc-800 px-3 py-1 text-xs font-medium text-zinc-200 hover:bg-zinc-700 disabled:opacity-50",
                                    disabled: testing(),
                                    onclick: move |_| {
                                        testing.set(true);
                                        connected.set(true);
                                        testing.set(false);
                                    },
                                    if testing() { "检测中…" } else { "测试连通" }
                                }
                            }

                            // 接口表单
                            div { class: "flex flex-col gap-3 rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4",
                                Field { label: "API 接口类型",
                                    select {
                                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-200 outline-none focus:border-zinc-600",
                                        value: "{api_type()}",
                                        onchange: move |e| api_type.set(e.value()),
                                        option { value: "openai_chat", "OpenAI Chat Completion (/v1/chat/completions)" }
                                        option { value: "claude", "Anthropic Messages (/v1/messages)" }
                                        option { value: "kobold", "KoboldCpp / Local GGUF" }
                                    }
                                }

                                Field { label: "API 端点 Base URL",
                                    input {
                                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-200 outline-none focus:border-zinc-600",
                                        placeholder: "https://api.openai.com/v1 或 http://127.0.0.1:3000/v1",
                                        value: "{endpoint()}",
                                        oninput: move |e| endpoint.set(e.value()),
                                    }
                                }

                                Field { label: "API Key 访问密钥",
                                    div { class: "flex items-center gap-2",
                                        input {
                                            r#type: if show_key() { "text" } else { "password" },
                                            class: "flex-1 rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-200 outline-none focus:border-zinc-600",
                                            placeholder: "sk-… (存入后端安全目录)",
                                            value: "{api_key()}",
                                            oninput: move |e| api_key.set(e.value()),
                                        }
                                        button {
                                            class: "rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-zinc-400 hover:text-zinc-200",
                                            onclick: move |_| show_key.set(!show_key()),
                                            if show_key() { "隐藏" } else { "查看" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    ActiveSection::Sampling => rsx! {
                        div { class: "flex max-w-2xl flex-col gap-5",
                            // 预设选择器
                            div { class: "flex items-center justify-between rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4",
                                div { class: "flex flex-col gap-0.5",
                                    span { class: "text-xs font-semibold text-zinc-200", "采样预设 (Preset)" }
                                    span { class: "text-[11px] text-zinc-500", "快速套用常用的温度与惩罚参数组合" }
                                }
                                select {
                                    class: "rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-zinc-200 outline-none focus:border-zinc-600",
                                    value: "{preset()}",
                                    onchange: move |e| {
                                        let v = e.value();
                                        preset.set(v.clone());
                                        match v.as_str() {
                                            "Creative" => {
                                                temperature.set(1.1);
                                                top_p.set(0.98);
                                                rep_penalty.set(1.15);
                                            }
                                            "Precise" => {
                                                temperature.set(0.4);
                                                top_p.set(0.8);
                                                rep_penalty.set(1.05);
                                            }
                                            _ => {
                                                temperature.set(0.8);
                                                top_p.set(0.95);
                                                rep_penalty.set(1.1);
                                            }
                                        }
                                    },
                                    option { value: "Default", "Default (平衡)" }
                                    option { value: "Creative", "Creative (发散/故事)" }
                                    option { value: "Precise", "Precise (严谨/对话)" }
                                }
                            }

                            // 滑杆组
                            div { class: "flex flex-col gap-4 rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4",
                                SliderField {
                                    label: "Temperature",
                                    value: temperature(),
                                    min: 0.0,
                                    max: 2.0,
                                    step: 0.05,
                                    on_change: move |v| temperature.set(v),
                                }
                                SliderField {
                                    label: "Top P (Nucleus)",
                                    value: top_p(),
                                    min: 0.0,
                                    max: 1.0,
                                    step: 0.01,
                                    on_change: move |v| top_p.set(v),
                                }
                                SliderField {
                                    label: "Top K",
                                    value: top_k(),
                                    min: 0.0,
                                    max: 100.0,
                                    step: 1.0,
                                    on_change: move |v| top_k.set(v),
                                }
                                SliderField {
                                    label: "Min P",
                                    value: min_p(),
                                    min: 0.0,
                                    max: 1.0,
                                    step: 0.01,
                                    on_change: move |v| min_p.set(v),
                                }
                                SliderField {
                                    label: "Rep. Penalty",
                                    value: rep_penalty(),
                                    min: 1.0,
                                    max: 2.0,
                                    step: 0.05,
                                    on_change: move |v| rep_penalty.set(v),
                                }
                            }

                            // 长度与流式
                            div { class: "flex flex-col gap-3 rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4",
                                div { class: "flex items-center justify-between",
                                    div { class: "flex flex-col",
                                        span { class: "font-medium text-zinc-300", "流式生成 (Streaming)" }
                                        span { class: "text-[11px] text-zinc-500", "逐字渲染角色回复，降低等待首字延迟" }
                                    }
                                    input {
                                        r#type: "checkbox",
                                        class: "h-4 w-4 accent-zinc-100",
                                        checked: streaming(),
                                        oninput: move |e| streaming.set(e.value().parse().unwrap_or(true)),
                                    }
                                }
                                div { class: "flex items-center justify-between border-t border-zinc-800/60 pt-3",
                                    div { class: "flex flex-col",
                                        span { class: "font-medium text-zinc-300", "Max Tokens" }
                                        span { class: "text-[11px] text-zinc-500", "单轮回复最大生成 token 预算" }
                                    }
                                    input {
                                        r#type: "number",
                                        class: "w-28 rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-right text-zinc-200 outline-none focus:border-zinc-600",
                                        value: "{max_tokens()}",
                                        oninput: move |e| {
                                            if let Ok(v) = e.value().parse() {
                                                max_tokens.set(v);
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}
