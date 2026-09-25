//! 别名弹窗组件:新建/编辑表单(基本 / 按量 / 按次 三 tab)。
//!
//! 【是什么】别名的新建/编辑表单弹窗,内含三个页签:0「基本」(别名 ID / 展示名 / 倍率 / 定价模式),
//! 1「按量定价」(输入/输出/缓存/补全价格),2「按次定价」(单次调用价格)。
//!
//! 【做什么】渲染表单、就地预览倍率、并在提交时以 `EventHandler` 抛出提交意图;
//! **不自己发网络请求**——网络写回由页面层完成。不负责开关弹窗(由页面的 `modal_state`
//! 控制)、不负责回填(页面 `open_edit` 已把值写进传入的 Signal)、不负责列表刷新
//! (只回调 `on_submit`)。
//!
//! 【交互逻辑】用户操作 → 组件行为 → 数据交互:
//! - 输入框 / 白名单 / 倍率 / 别名 → 直接 `set` 对应的页面级 Signal(纯本地)。
//! - 点倍率预设按钮 → `ratio.set(预设值)`(纯本地)。
//! - 点页签胶囊 → 本地 `active_tab.set(i)` 切换两个页签的显隐。
//! - 点「取消」/ `Modal` 的关闭 → `on_cancel` 抛回页面关弹窗。
//! - 点提交 → `do_submit`:验证字段,置 `submitting = true`,回调 `on_submit` 通知页面,
//!   页面据此发起 `update_model_alias_api` / `delete_model_alias_api` / 新建拒绝。
//!
//! 【样式】经 `Modal` 外壳(`max-w-md`);输入框统一 `MODAL_INPUT`
//! (`w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5`,
//! 聚焦 `focus:border-zinc-500`);倍率输入额外加 `font-mono`;预设/别名 chip 选中态
//! `border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold`;计费预览块
//! `rounded-xl border border-zinc-800 bg-zinc-950`,数值按倍率着色(低于 1
//! 为 `text-emerald-400`,高于 1 为 `text-amber-400`);底部两按钮
//! `flex-1 rounded-xl`,提交为白底 `bg-white text-zinc-900`,禁用时 `disabled:opacity-40`。
//!
//! 【子组件组成】`Modal`(外壳)、`SegmentedCapsule`(三页签胶囊)。
//!
//! 【数据流】
//! - 对内(入):`mode`(新建/编辑)、`alias`(编辑时的原始数据)、`on_cancel` / `on_submit`。
//! - 对外(出):表单 Signal 的写回停留本地表单(仅提交时读取);`on_cancel` → 页面
//!   置 `ModalState::Closed`;`on_submit` → 页面处理网络请求与列表刷新。

use dioxus::prelude::*;

use crate::shared::{
    BTN_CANCEL, BTN_CREATE_ALIAS, BTN_SAVE_CHANGES,
    FIELD_ALIAS_ID, FIELD_DISPLAY, FIELD_MULTIPLIER, FIELD_PRICE_MODE,
    FIELD_INPUT_PRICE, FIELD_OUTPUT_PRICE, FIELD_CACHE_READ_PRICE,
    FIELD_CACHE_WRITE_PRICE, FIELD_PER_CALL_PRICE,
    LBL_INPUT_PRICE_DESC, LBL_OUTPUT_PRICE_DESC, LBL_CACHE_READ_DESC,
    LBL_CACHE_WRITE_DESC, LBL_COMPLETION_DESC, LBL_PER_CALL_DESC,
    MSG_PH_ALIAS_ID, MSG_PH_DISPLAY, MSG_PH_PER_CALL,
    SEC_MODE_NOTE, SEC_PER_CALL_NOTE,
    TAB_BASIC, TAB_PER_TOKEN, TAB_PER_CALL, TTL_NEW,
};
use crate::tab_page_aliases::shared::{AliasItem, PriceMode};
use ui::{Modal, SegmentedCapsule};

#[derive(Clone, Copy, PartialEq)]
pub enum ModalMode {
    New,
    Edit(String),
}

#[component]
pub fn AliasModal(
    mode: ModalMode,
    on_close: EventHandler<()>,
    on_submit: EventHandler<()>,,
) -> Element {
    let is_new = matches!(mode, ModalMode::New);
    
    // 表单状态 - 由页面持有或这里临时持有
    let mut alias_id = use_signal(String::new);
    let mut display_name = use_signal(String::new);
    let mut multiplier = use_signal(String::new);
    let mut price_mode = use_signal(|| PriceMode::Standard);
    let mut input_price = use_signal(String::new);
    let mut output_price = use_signal(String::new);
    let mut cache_read_price = use_signal(String::new);
    let mut cache_write_price = use_signal(String::new);
    let mut completion_price = use_signal(String::new);
    let mut per_call_price = use_signal(String::new);
    let mut active_tab = use_signal(|| 0usize);
    let mut submitting = use_signal(|| false);
    
    // 标题与提交文案
    let title = if is_new { TTL_NEW } else { "编辑模型别名" };
    let submit_label = if is_new { BTN_CREATE_ALIAS } else { BTN_SAVE_CHANGES };
    
    rsx! {
        Modal {
            title: title.to_string(),
            on_close: move |_| on_close.call(()),
            div { class: "flex flex-col gap-4",
                // 页签
                SegmentedCapsule {
                    options: vec![TAB_BASIC.to_string(), TAB_PER_TOKEN.to_string(), TAB_PER_CALL.to_string()],
                    selected: active_tab(),
                    on_select: move |idx| active_tab.set(idx),
                }
                
                // 基本页签
                if active_tab() == 0 {
                    div { class: "flex flex-col gap-4",
                        // 别名标识
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_ALIAS_ID}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                placeholder: MSG_PH_ALIAS_ID,
                                value: alias_id(),
                                oninput: move |evt| alias_id.set(evt.value()),
                            }
                        }
                        // 展示名称
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_DISPLAY}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                placeholder: MSG_PH_DISPLAY,
                                value: display_name(),
                                oninput: move |evt| display_name.set(evt.value()),
                            }
                        }
                        // 计费倍率
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_MULTIPLIER}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                                placeholder: "1.0",
                                value: multiplier(),
                                oninput: move |evt| multiplier.set(evt.value()),
                            }
                        }
                        // 定价模式
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_PRICE_MODE}" }
                            SegmentedCapsule {
                                options: vec!["标准 1.0×".to_string(), "按量定价".to_string(), "按次定价".to_string()],
                                selected: match price_mode() {
                                    PriceMode::Standard => 0,
                                    PriceMode::PerToken => 1,
                                    PriceMode::PerCall => 2,
                                },
                                on_select: move |idx| price_mode.set(match idx {
                                    0 => PriceMode::Standard,
                                    1 => PriceMode::PerToken,
                                    2 => PriceMode::PerCall,
                                    _ => PriceMode::Standard,
                                }),
                            }
                        }
                    }
                }
                
                // 按量定价页签
                else if active_tab() == 1 {
                    div { class: "flex flex-col gap-4",
                        p { class: "text-xs text-zinc-500", "{SEC_MODE_NOTE}" }
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_INPUT_PRICE}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                placeholder: "0.00",
                                value: input_price(),
                                oninput: move |evt| input_price.set(evt.value()),
                            }
                            p { class: "mt-1 text-xs text-zinc-500", "{LBL_INPUT_PRICE_DESC}" }
                        }
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_OUTPUT_PRICE}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                placeholder: "0.00",
                                value: output_price(),
                                oninput: move |evt| output_price.set(evt.value()),
                            }
                            p { class: "mt-1 text-xs text-zinc-500", "{LBL_OUTPUT_PRICE_DESC}" }
                        }
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_CACHE_READ_PRICE}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                placeholder: "0.00",
                                value: cache_read_price(),
                                oninput: move |evt| cache_read_price.set(evt.value()),
                            }
                            p { class: "mt-1 text-xs text-zinc-500", "{LBL_CACHE_READ_DESC}" }
                        }
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_CACHE_WRITE_PRICE}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                placeholder: "0.00",
                                value: cache_write_price(),
                                oninput: move |evt| cache_write_price.set(evt.value()),
                            }
                            p { class: "mt-1 text-xs text-zinc-500", "{LBL_CACHE_WRITE_DESC}" }
                        }
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_COMPLETION_PRICE}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                placeholder: "0.00",
                                value: completion_price(),
                                oninput: move |evt| completion_price.set(evt.value()),
                            }
                            p { class: "mt-1 text-xs text-zinc-500", "{LBL_COMPLETION_DESC}" }
                        }
                    }
                }
                
                // 按次定价页签
                else {
                    div { class: "flex flex-col gap-4",
                        p { class: "text-xs text-zinc-500", "{SEC_PER_CALL_NOTE}" }
                        div {
                            label { class: "block text-sm text-zinc-300 mb-1", "{FIELD_PER_CALL_PRICE}" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                placeholder: MSG_PH_PER_CALL,
                                value: per_call_price(),
                                oninput: move |evt| per_call_price.set(evt.value()),
                            }
                            p { class: "mt-1 text-xs text-zinc-500", "{LBL_PER_CALL_DESC}" }
                        }
                    }
                }
                
                // 底部按钮
                div { class: "flex gap-3 pt-2",
                    button {
                        class: "flex-1 rounded-xl border border-zinc-700 bg-zinc-950 py-2.5 text-sm text-zinc-300 hover:border-zinc-500 transition-colors",
                        onclick: move |_| on_close.call(()),
                        disabled: submitting(),
                        "{BTN_CANCEL}"
                    }
                    button {
                        class: "flex-1 rounded-xl bg-white text-zinc-900 py-2.5 text-sm font-medium hover:bg-zinc-200 transition-colors disabled:opacity-40",
                        onclick: move |_| {
                            if !submitting() {
                                submitting.set(true);
                                on_submit.call(());
                            }
                        },
                        disabled: submitting(),
                        "{submit_label}"
                    }
                }
            }
        }
    }
}