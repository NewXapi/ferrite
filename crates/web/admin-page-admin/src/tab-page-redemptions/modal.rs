//! 兑换码生成弹窗 + 生成成功后的一次性明文码展示。
//!
//! 纯展示组件:表单状态以 `Signal` 注入(Signal 是可拷贝的全局句柄),数量/面额
//! 直接写回页面级 signal;提交按钮只把事件抛给页面(`on_submit`),校验与
//! 网络写回(POST /api/redemption)在 `page` 的
//! `commit_generate` 里。两组件自身都不发网络请求。
//!
//! 边界:弹窗外壳复用 `tab-page-groups::Modal`;不含生成请求、不含明文码的保存
//! (仅展示与关闭)。

use dioxus::prelude::*;

use super::shared::{
    BTN_CANCEL, BTN_CLOSE_SAVED, BTN_SUBMIT_GENERATE, FIELD_COUNT, FIELD_QUOTA, LBL_BATCH,
    LBL_QUOTA_PRESETS, LBL_TOTAL_VALUE, MSG_CODES_WARNING, MSG_GENERATE_HINT, TTL_GENERATE,
    TTL_GENERATED_PREFIX, TTL_GENERATED_SUFFIX,
};
use crate::tab_page_groups::Modal;

/// 批量生成兑换码弹窗(后端仅支持 面额 + 数量;无活动名/有效期字段)。
///
/// 【是什么】生成兑换码的表单弹窗:数量 / 单张面额两个输入框(两列网格)、快捷面额预设
/// 按钮、实时测算卡片(生成总批次 + 发行总面值金额)、底部一次性提示与 [取消][立即批量生成]。
///
/// 【做什么】就地读写页面的 `count` / `quota` signal,并按当前值实时算出
/// `parsed_count` / `parsed_quota` / `total_value` 用于高亮预设与测算展示。
/// 不负责校验与请求(提交只抛 `on_submit`,真正的 clamp 与换算在页面 `commit_generate`)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 输入数量 / 面额 → `count.set(e.value())` / `quota.set(e.value())`(纯本地)。
/// - 点快捷面额按钮 → `quota.set(预设值)`;当前值命中某预设时会高亮该按钮
///   (`parsed_quota` 与预设值差 < 0.001)。
/// - 点「取消」或关闭 → `on_cancel` 抛回页面关弹窗。
/// - 点「立即批量生成」→ `on_submit` 抛回页面,页面 `commit_generate` 把 ¥ 换算成
///   后端计费单位后调生成 API,成功则切到明文码弹窗。
/// 数据交互:本组件自身**不发任何网络请求**。
///
/// 【样式】经 `Modal` 外壳(标题 `TTL_GENERATE`);内容区 `space-y-4 max-h-[70vh]
/// overflow-y-auto pr-1`;两个输入框同用 `w-full rounded-xl border border-zinc-700
/// bg-zinc-950 px-4 py-2.5 font-mono`(数量框带 `min: "1"` `max: "100"`);预设 chip
/// 选中态 `border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold`;测算卡片
/// `rounded-xl border border-zinc-800 bg-zinc-950`,发行总额用 `text-emerald-400 font-mono`;
/// 提示行 `text-[11px] text-zinc-600`;底部取消为描边按钮、提交为白底 `bg-white text-zinc-900`。
///
/// 【子组件组成】`Modal`(外壳);其余为原生 `div` / `input` / `button`。
///
/// 【数据流】
/// - 对内(入):`count` / `quota`(页面持有的表单 Signal,双向就地读写)、
///   `on_cancel` / `on_submit`。
/// - 对外(出):两个 Signal 的写回只影响本弹窗预览与页面提交时的读取;
///   `on_cancel` → 页面置 `RedModalState::Closed`;`on_submit` → 页面
///   `commit_generate`(换算 + 调 `generate_redemptions_api`,成功切 `Codes` 弹窗)。
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

    // 快捷面额预设:纯金额字面量(符号/数字,非文案),不抽常量。
    let preset_quotas = [
        ("¥10", "10"),
        ("¥20", "20"),
        ("¥50", "50"),
        ("¥100", "100"),
        ("¥200", "200"),
    ];

    rsx! {
        Modal { title: TTL_GENERATE.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4 max-h-[70vh] overflow-y-auto pr-1",
                div { class: "grid grid-cols-2 gap-3",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_COUNT}" }
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
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_QUOTA}" }
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
                    p { class: "text-[11px] text-zinc-500", "{LBL_QUOTA_PRESETS}" }
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
                        span { "{LBL_BATCH}" }
                        span { "{parsed_count} 张卡密" }
                    }
                    div { class: "flex justify-between font-medium",
                        span { class: "text-zinc-300", "{LBL_TOTAL_VALUE}" }
                        span { class: "{ui::STATE_SUCCESS_TEXT} font-mono text-sm",
                            "¥ {total_value:.2}"
                        }
                    }
                }

                p { class: "text-[11px] text-zinc-600",
                    "{MSG_GENERATE_HINT}"
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    "data-testid": "cancel-generate",
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "{BTN_CANCEL}"
                }
                button {
                    "data-testid": "submit-generate",
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                    onclick: move |_| on_submit.call(()),
                    "{BTN_SUBMIT_GENERATE}"
                }
            }
        }
    }
}

/// 生成成功后的一次性明文码展示(关闭后不再可见)。
///
/// 【是什么】生成成功后的明文卡密展示弹窗:红色警示语 + 可滚动的 `<pre>` mono 列表 +
/// 单个 [我已保存,关闭] 按钮。
///
/// 【做什么】把 `codes` 用换行拼成一整块文本展示,标题带张数。不负责复制到剪贴板
/// (靠 `select-all` 让用户手动选中)、不负责再次获取明文(后端只返回一次)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点「我已保存,关闭」或关闭按钮 → `on_close` 抛回页面,页面置 `RedModalState::Closed`。
/// 数据交互:纯展示 + 单关闭回调,本组件自身**不发任何网络请求**。
///
/// 【样式】经 `Modal` 外壳(标题 `生成成功 · {N} 张明文卡密(仅此一次)`);警示语
/// `text-xs text-amber-400`;码块为 `max-h-72 overflow-y-auto rounded-xl border
/// border-zinc-700 bg-zinc-950 p-3 font-mono text-xs text-zinc-200 select-all`
/// (方便整块选中复制);底部单按钮 `flex-1 rounded-xl bg-white text-zinc-900`。
///
/// 【子组件组成】`Modal`(外壳);其余为原生 `div` / `p` / `pre` / `button`。
///
/// 【数据流】
/// - 对内(入):`codes`(生成接口返回的明文码列表,页面从 `RedModalState::Codes` 取出)、
///   `on_close`。
/// - 对外(出):`on_close` → 页面关弹窗;明文码随后从内存状态中消失,不可再查看。
#[component]
pub fn GeneratedCodesModal(codes: Vec<String>, on_close: EventHandler<()>) -> Element {
    let joined = codes.join("\n");
    rsx! {
        Modal { title: format!("{TTL_GENERATED_PREFIX}{}{TTL_GENERATED_SUFFIX}", codes.len()), on_close: move |_| on_close.call(()),
            div { class: "space-y-3",
                p { class: "text-xs {ui::STATE_WARNING_TEXT}",
                    "{MSG_CODES_WARNING}"
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
                    "{BTN_CLOSE_SAVED}"
                }
            }
        }
    }
}
