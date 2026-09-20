//! 别名新建/编辑弹窗:基本 / 按量定价 / 按次定价 三 tab。
//!
//! 纯展示组件:表单状态以 `Signal` 注入(Signal 是可拷贝的全局句柄),定价
//! 模式 toggle 直接写回页面级 signal;提交按钮只把事件抛给页面(`on_submit`),
//! 校验与网络写回(PUT /api/models/{key})在 `page` 的
//! `submit_alias` 里。三 tab 内容统一固定高度,切换时弹窗不伸缩。
//!
//! 边界:本文件**不做**输入校验(空别名直接由页面 `submit_alias` 提前 return)、
//! **不做**网络请求、**不决定**提交按钮的启用语义(`submitting` 由页面的写回
//! 闭包控制);弹窗外壳与遮罩行为复用 `tab_page_groups::Modal`。

use dioxus::prelude::*;

use super::shared::{
    BTN_CANCEL, BTN_CREATE_ALIAS, FIELD_ALIAS_ID, FIELD_DISPLAY, FIELD_INPUT_PRICE,
    FIELD_MULTIPLIER, FIELD_PER_CALL_PRICE, FIELD_PRICE_MODE, LBL_CH_CACHE_READ,
    LBL_CH_CACHE_READ_DESC, LBL_CH_CACHE_WRITE, LBL_CH_CACHE_WRITE_DESC, LBL_CH_COMPLETION,
    LBL_CH_COMPLETION_DESC, LBL_CH_OUTPUT, LBL_CH_OUTPUT_DESC, LBL_INPUT_PRICE_DESC,
    LBL_MODAL_TABLIST, LBL_PER_CALL_DESC, MSG_PH_ALIAS_ID, MSG_PH_DISPLAY, MSG_PH_PER_CALL,
    PriceMode, PriceModeToggle, SEC_MODE_NOTE, SEC_PER_CALL_NOTE, TAB_BASIC, TAB_PER_CALL,
    TAB_PER_TOKEN, TTL_NEW,
};
use crate::tab_page_groups::Modal;
/// 别名新建/编辑弹窗
///
/// 【是什么】别名 tab 的新建/编辑弹窗,内含「基本 / 按量定价 / 按次定价」三个页签。
///
/// 【做什么】渲染表单、页签切换与底部取消/提交两个按钮,并把表单值写回页面注入的
/// Signal;不负责提交前的校验与网络写回(在页面 `submit_alias`)、不负责弹窗
/// 的开关(`modal_state` 归页面)、不负责决定标题/按钮文案之外的业务逻辑。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点顶部页签 → 写页面注入的 `active_tab`(仅影响弹窗本体,不关弹窗)。
/// - 在标识/展示名/倍率/价格输入框输入 → `oninput` 写对应 Signal(纯本地,不发网络)。
/// - 切换定价模式 toggle → `PriceModeToggle::on_change` 写页面级 `price_mode`。
/// - 按各补充通道开关 → 写对应的 `c_*_on` bool(启用/停用该通道行)。
/// - 点「取消」→ `on_cancel`(页面关弹窗,`modal_state = Closed`)。
/// - 点「保存修改 / 创建别名」→ `on_submit`,校验与 PUT `/api/models/{key}` 由页面执行;
///   按钮在 `submitting()` 为 true 时 `disabled:opacity-40`。
///
/// 【样式】外壳复用 `Modal`(标题栏 + 关闭按钮);内容区 `div.space-y-4`。三页签用
/// `flex w-full overflow-hidden rounded-lg border border-zinc-800 bg-zinc-950 p-0.5`,
/// 选中项 `bg-zinc-100 text-zinc-900`、未选中 `text-zinc-400 hover:text-zinc-200`;
/// 三 tab 内容统一 `min-h-[300px]`(`TAB_CONTENT_H`)固定高度,**切换时弹窗不伸缩**;
/// 底部按钮区 `mt-6 flex gap-3`,取消为描边 `border-zinc-700`,提交为白底 `bg-white`。
///
/// 【子组件组成】`Modal`(弹窗外壳)、`PriceModeToggle`(定价模式开关);
/// 按量 tab 的补充通道行由函数内闭包 `price_panel` 生成(标题 + 悬停说明 / 价格框 /
/// 开关),不是独立的 `#[component]`。
///
/// 【数据流】
/// - 对内(入):表单 Signal —— `alias` / `display` / `input_rate` / `output_rate` / `multiplier` /
///   `price_mode` / `active_tab`;价格 Signal —— `p_input` / `p_output` /
///   `p_cache_read` / `p_cache_write` / `p_completion` / `p_per_call`;通道开关 ——
///   `c_output_on` / `c_cache_read_on` / `c_cache_write_on` / `c_completion_on`;
///   `submitting`。全部由 `page.rs` 持有并在打开弹窗时重置。
/// - 对外(出):上述 Signal 就地写回页面状态(输入即改,不发网络);`on_cancel` 关弹窗;
///   `on_submit` 抛回页面,由页面走 PUT 或「新建被拒」提示。
#[component]
pub fn AliasFormModal(
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
    let title = TTL_NEW;
    let submit_label = BTN_CREATE_ALIAS;

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
                    "aria-label": "{LBL_MODAL_TABLIST}",
                    for (i, tab_label) in ([TAB_BASIC, TAB_PER_TOKEN, TAB_PER_CALL]).into_iter().enumerate() {
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
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_ALIAS_ID}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                "data-testid": "alias-name",
                                placeholder: "{MSG_PH_ALIAS_ID}",
                                value: "{alias}",
                                oninput: move |e| alias.set(e.value()),
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_DISPLAY}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                "data-testid": "alias-display",
                                placeholder: "{MSG_PH_DISPLAY}",
                                value: "{display}",
                                oninput: move |e| display.set(e.value()),
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_MULTIPLIER}" }
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
                            label { class: "block text-xs text-zinc-400", "{FIELD_PRICE_MODE}" }
                            PriceModeToggle { active: price_mode(), on_change: on_mode_change, compact: false }
                            p { class: "mt-1 text-[11px] text-zinc-500", "{SEC_MODE_NOTE}" }
                        }
                    }
                }

                // ---- tab 1:按量定价 — 输入/输出/缓存读取/缓存写入/补全 5 项($/1M) ----
                if active_tab() == 1 {
                    div { class: "space-y-3 {TAB_CONTENT_H}",
                        // 输入价格:主通道,固定开启,不带 toggle
                        div { class: "space-y-1.5",
                            div {
                                label { class: "block text-sm font-medium text-zinc-100", "{FIELD_INPUT_PRICE}" }
                                p { class: "mt-0.5 text-[11px] text-zinc-500", "{LBL_INPUT_PRICE_DESC}" }
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
                                LBL_CH_OUTPUT.to_string(),
                                LBL_CH_OUTPUT_DESC.to_string(),
                                c_output_on,
                                p_output,
                                "alias-output-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                LBL_CH_CACHE_READ.to_string(),
                                LBL_CH_CACHE_READ_DESC.to_string(),
                                c_cache_read_on,
                                p_cache_read,
                                "alias-cache-read-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                LBL_CH_CACHE_WRITE.to_string(),
                                LBL_CH_CACHE_WRITE_DESC.to_string(),
                                c_cache_write_on,
                                p_cache_write,
                                "alias-cache-write-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                LBL_CH_COMPLETION.to_string(),
                                LBL_CH_COMPLETION_DESC.to_string(),
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
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_PER_CALL_PRICE}" }
                            p { class: "text-[11px] text-zinc-500", "{LBL_PER_CALL_DESC}" }
                        }
                        div {
                            class: "flex items-center gap-3 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5",
                            span { class: "text-xs text-zinc-500", "$" }
                            input {
                                class: "w-full bg-transparent font-mono text-sm text-zinc-100 focus:outline-none",
                                r#type: "text",
                                "data-testid": "alias-call-price",
                                placeholder: "{MSG_PH_PER_CALL}",
                                value: "{p_per_call}",
                                oninput: move |e| p_per_call.set(e.value()),
                            }
                            span { class: "shrink-0 text-[11px] text-zinc-500", "USD/次" }
                        }
                        p { class: "text-[11px] text-zinc-500", "{SEC_PER_CALL_NOTE}" }
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    "data-testid": "alias-cancel",
                    onclick: move |_| on_cancel.call(()),
                    "{BTN_CANCEL}"
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
