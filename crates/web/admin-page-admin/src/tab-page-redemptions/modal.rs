//! 兑换码生成弹窗 + 生成成功后的一次性明文码展示。
//!
//! 纯展示组件:表单状态以 `Signal` 注入(Signal 是可拷贝的全局句柄),数量/面额
//! 直接写回页面级 signal;提交按钮只把事件抛给页面(`on_submit`),校验与
//! 网络写回(POST /api/redemption)在 `page` 的
//! `commit_generate` 里。

use dioxus::prelude::*;

use crate::tab_page_groups::Modal;

/// 批量生成兑换码弹窗(后端仅支持 面额 + 数量;无活动名/有效期字段)
#[component]
pub fn RedemptionGenerateModal(
    count: Signal<String>,
    quota: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let parsed_count = count().trim().parse::<u32>().unwrap_or(1).clamp(1, 100);
    let parsed_quota = quota().trim().parse::<f64>().unwrap_or(10.0).max(0.0);
    let total_value = (parsed_count as f64) * parsed_quota;

    let preset_quotas = [
        ("¥10", "10"),
        ("¥20", "20"),
        ("¥50", "50"),
        ("¥100", "100"),
        ("¥200", "200"),
    ];

    rsx! {
        Modal { title: "生成批量兑换码".to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4 max-h-[70vh] overflow-y-auto pr-1",
                div { class: "grid grid-cols-2 gap-3",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "生成数量 (1-100)" }
                    input {
                        "data-testid": "redemption-count",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        r#type: "number",
                        min: "1",
                        max: "100",
                        value: "{count}",
                        oninput: move |e| count.set(e.value()),
                    }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "单张面额 (元)" }
                    input {
                        "data-testid": "redemption-quota",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        placeholder: "50",
                        value: "{quota}",
                        oninput: move |e| quota.set(e.value()),
                    }
                    }
                }

                // 快捷面额按钮
                div { class: "space-y-1.5",
                    p { class: "text-[11px] text-zinc-500", "快捷面额预设" }
                    div { class: "flex flex-wrap gap-1.5",
                        for (lbl, val) in preset_quotas {
                            {
                                let is_active = (parsed_quota - val.parse::<f64>().unwrap_or(0.0)).abs() < 0.001;
                                let btn_tone = if is_active {
                                    "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                } else {
                                    "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                };
                                rsx! {
                                    button {
                                        class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {btn_tone}",
                                        onclick: move |_| quota.set(val.to_string()),
                                        "{lbl}"
                                    }
                                }
                            }
                        }
                    }
                }

                // 测算卡片
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs space-y-1.5",
                    div { class: "flex justify-between text-zinc-400",
                        span { "生成总批次" }
                        span { "{parsed_count} 张卡密" }
                    }
                    div { class: "flex justify-between font-medium",
                        span { class: "text-zinc-300", "发行总面值金额" }
                        span { class: "text-emerald-400 font-mono text-sm",
                            "¥ {total_value:.2}"
                        }
                    }
                }

                p { class: "text-[11px] text-zinc-600",
                    "提示: 明文卡密只在生成后显示一次,请及时复制保存;后端暂不支持活动名与有效期字段。"
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    "data-testid": "cancel-generate",
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    "data-testid": "submit-generate",
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                    onclick: move |_| on_submit.call(()),
                    "立即批量生成"
                }
            }
        }
    }
}

/// 生成成功后的一次性明文码展示(关闭后不再可见)
#[component]
pub fn GeneratedCodesModal(codes: Vec<String>, on_close: EventHandler<()>) -> Element {
    let joined = codes.join("\n");
    rsx! {
        Modal { title: format!("生成成功 · {} 张明文卡密(仅此一次)", codes.len()), on_close: move |_| on_close.call(()),
            div { class: "space-y-3",
                p { class: "text-xs text-amber-400",
                    "以下明文卡密关闭本窗口后无法再次查看,请立即复制保存。"
                }
                pre {
                    "data-testid": "generated-codes",
                    class: "max-h-72 overflow-y-auto rounded-xl border border-zinc-700 bg-zinc-950 p-3 font-mono text-xs text-zinc-200 select-all",
                    "{joined}"
                }
            }
            div { class: "mt-6 flex",
                button {
                    "data-testid": "close-codes",
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                    onclick: move |_| on_close.call(()),
                    "我已保存,关闭"
                }
            }
        }
    }
}
