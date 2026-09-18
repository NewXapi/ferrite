//! 别名新建/编辑弹窗:基本 / 按量定价 / 按次定价 三 tab。
//!
//! 纯展示组件:表单状态以 `Signal` 注入(Signal 是可拷贝的全局句柄),定价
//! 模式 toggle 直接写回页面级 signal;提交按钮只把事件抛给页面(`on_submit`),
//! 校验与网络写回(PUT /api/models/{key})在 `page` 的
//! `submit_alias` 里。三 tab 内容统一固定高度,切换时弹窗不伸缩。

use dioxus::prelude::*;

use super::shared::{PriceMode, PriceModeToggle};
use crate::tab_page_groups::Modal;
/// 别名新建/编辑弹窗
#[component]
pub fn AliasFormModal(
    editing: bool,
    alias: Signal<String>,
    display: Signal<String>,
    input_rate: Signal<String>,
    output_rate: Signal<String>,
    multiplier: Signal<String>,
    price_mode: Signal<PriceMode>,
    active_tab: Signal<usize>,
    p_input: Signal<String>,
    p_output: Signal<String>,
    p_cache_read: Signal<String>,
    p_cache_write: Signal<String>,
    p_completion: Signal<String>,
    p_per_call: Signal<String>,
    c_output_on: Signal<bool>,
    c_cache_read_on: Signal<bool>,
    c_cache_write_on: Signal<bool>,
    c_completion_on: Signal<bool>,
    submitting: Signal<bool>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing {
        "编辑模型别名"
    } else {
        "新建模型别名"
    };
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建别名"
    };

    // 弹窗内 tab 信号:由页面持有(active_tab),弹窗内切换只影响弹窗本体
    let mut modal_tab = active_tab;
    // 定价模式 toggle 的写回调:更新页面级 f_price_mode
    let on_mode_change = {
        let mut pm = price_mode;
        move |mode: PriceMode| pm.set(mode)
    };

    // 提交:只把事件抛回页面,校验与网络写回(PUT /api/models/{key})在
    // 页面的 submit_alias 里(见 page)。
    let do_submit = move |_| {
        on_submit.call(());
    };

    // 弹窗按量 tab:紧凑单列排布 — 主通道(输入)+ 可关闭补充通道,
    // 每行 = 标题+开关 + 价格输入框(单行紧凑);开关控制通道是否启用。
    // 三 tab 内容统一固定高度,切换时弹窗不伸缩。
    let TAB_CONTENT_H: &str = "min-h-[300px]";
    let price_panel = |channel_title: String,
                       channel_desc: String,
                       mut enabled: Signal<bool>,
                       mut price: Signal<String>,
                       testid: String| {
        rsx! {
            div {
                class: "rounded-xl border border-zinc-800 bg-zinc-950/60 px-3 py-2.5",
                div { class: "flex items-center gap-3",
                    // 标题 + 悬停说明(title 属性,不占固定行高)
                    div {
                        class: "shrink-0 w-20",
                        title: "{channel_desc}",
                        p { class: "text-xs font-medium text-zinc-100 truncate", "{channel_title}" }
                    }
                    // 价格输入框
                    div { class: "flex-1 flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 px-2.5 py-1.5",
                        span { class: "text-[11px] text-zinc-500", "$" }
                        input {
                            class: "w-full bg-transparent font-mono text-xs text-zinc-100 focus:outline-none",
                            r#type: "text",
                            "data-testid": "{testid}",
                            value: "{price}",
                            oninput: move |e| price.set(e.value()),
                        }
                        span { class: "shrink-0 text-[10px] text-zinc-500", "USD" }
                    }
                    // 开关:点击切换启用状态
                    button {
                        class: "relative h-5 w-9 shrink-0 rounded-full transition-colors cursor-pointer",
                        style: if enabled() { "background-color: #525252" } else { "background-color: #3f3f46" },
                        "data-testid": "{testid}-toggle",
                        role: "switch",
                        aria_checked: "{enabled()}",
                        onclick: move |_| enabled.set(!enabled()),
                        div {
                            class: "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                            style: if enabled() { "left: 18px" } else { "left: 2px" },
                        }
                    }
                }
            }
        }
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                // 顶部 tab 栏:基本 / 按量定价 / 按次定价
                div {
                    class: "flex w-full overflow-hidden rounded-lg border border-zinc-800 bg-zinc-950 p-0.5 text-xs",
                    role: "tablist",
                    "aria-label": "别名编辑选项",
                    for (i, tab_label) in (["基本", "按量定价", "按次定价"]).into_iter().enumerate() {
                        button {
                            key: "{i}",
                            class: if i == active_tab() {
                                "flex-1 rounded-md bg-zinc-100 px-3 py-2 font-medium text-zinc-900 transition-colors"
                            } else {
                                "flex-1 rounded-md px-3 py-2 text-zinc-400 transition-colors hover:text-zinc-200"
                            },
                            role: "tab",
                            aria_selected: "{i == active_tab()}",
                            "data-testid": "alias-modal-tab-{i}",
                            onclick: move |_| modal_tab.set(i),
                            "{tab_label}"
                        }
                    }
                }

                // ---- tab 0:基本 — 标识 / 展示名 / 倍率 / 定价模式 toggle(与卡片同状态) ----
                if active_tab() == 0 {
                    div { class: "space-y-4 {TAB_CONTENT_H}",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "别名标识 (API 请求匹配名)" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                "data-testid": "alias-name",
                                placeholder: "例如: gpt-4o, claude-3-5-sonnet",
                                value: "{alias}",
                                oninput: move |e| alias.set(e.value()),
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "展示名称 (可选)" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                "data-testid": "alias-display",
                                placeholder: "例如: GPT-4o 旗舰模型",
                                value: "{display}",
                                oninput: move |e| display.set(e.value()),
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "计费倍率 (multiplier ≥ 0)" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                                "data-testid": "alias-multiplier",
                                placeholder: "1.0",
                                value: "{multiplier}",
                                oninput: move |e| multiplier.set(e.value()),
                            }
                        }

                        // 定价模式 toggle:与卡片面板同一状态(非 compact 全宽)
                        div { class: "space-y-1.5",
                            label { class: "block text-xs text-zinc-400", "定价模式 (启用哪种定价)" }
                            PriceModeToggle { active: price_mode(), on_change: on_mode_change, compact: false }
                            p { class: "mt-1 text-[11px] text-zinc-500", "切换到按量定价后,补充通道的启用开关在「按量定价」tab" }
                        }
                    }
                }

                // ---- tab 1:按量定价 — 输入/输出/缓存读取/缓存写入/补全 5 项($/1M) ----
                if active_tab() == 1 {
                    div { class: "space-y-3 {TAB_CONTENT_H}",
                        // 输入价格:主通道,固定开启,不带 toggle
                        div { class: "space-y-1.5",
                            div {
                                label { class: "block text-sm font-medium text-zinc-100", "输入价格" }
                                p { class: "mt-0.5 text-[11px] text-zinc-500", "每 100 万输入 token 的价格。" }
                            }
                            div { class: "flex items-center gap-3 rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-2",
                                span { class: "text-xs text-zinc-500", "$" }
                                input {
                                    class: "w-full bg-transparent font-mono text-sm text-zinc-100 focus:outline-none",
                                    r#type: "text",
                                    "data-testid": "alias-input-price",
                                    value: "{p_input}",
                                    oninput: move |e| p_input.set(e.value()),
                                }
                                span { class: "shrink-0 text-[11px] text-zinc-500", "$/1M" }
                            }
                        }

                        // 补充通道:紧凑单行面板(标题+悬停说明 / 价格框 / 开关)
                        {
                            price_panel(
                                "输出价格".to_string(),
                                "生成内容的输出 token 价格(悬停标题查看)".to_string(),
                                c_output_on,
                                p_output,
                                "alias-output-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                "缓存读取价格".to_string(),
                                "缓存读取 token 价格(悬停标题查看)".to_string(),
                                c_cache_read_on,
                                p_cache_read,
                                "alias-cache-read-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                "缓存写入价格".to_string(),
                                "缓存写入 token 价格(悬停标题查看)".to_string(),
                                c_cache_write_on,
                                p_cache_write,
                                "alias-cache-write-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                "补全价格".to_string(),
                                "补全(输出)调用的 token 价格(悬停标题查看)".to_string(),
                                c_completion_on,
                                p_completion,
                                "alias-completion-price".to_string(),
                            )
                        }
                    }
                }

                // ---- tab 2:按次定价 — 每次调用固定费用 ----
                if active_tab() == 2 {
                    div { class: "space-y-3 {TAB_CONTENT_H}",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "单次调用价格" }
                            p { class: "text-[11px] text-zinc-500", "每次调用(不论 token 数)固定扣费。" }
                        }
                        div {
                            class: "flex items-center gap-3 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5",
                            span { class: "text-xs text-zinc-500", "$" }
                            input {
                                class: "w-full bg-transparent font-mono text-sm text-zinc-100 focus:outline-none",
                                r#type: "text",
                                "data-testid": "alias-call-price",
                                placeholder: "例如: 0.05",
                                value: "{p_per_call}",
                                oninput: move |e| p_per_call.set(e.value()),
                            }
                            span { class: "shrink-0 text-[11px] text-zinc-500", "USD/次" }
                        }
                        p { class: "text-[11px] text-zinc-500", "后端落地前按次价格暂存于倍率字段,仅 UI 层生效。" }
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    "data-testid": "alias-cancel",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    "data-testid": "alias-submit",
                    disabled: submitting(),
                    onclick: do_submit,
                    "{submit_label}"
                }
            }
        }
    }
}

