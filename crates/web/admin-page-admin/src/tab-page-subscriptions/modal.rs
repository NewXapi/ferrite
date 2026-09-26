//! 订阅套餐编辑/新建弹窗:两 Tab(基本信息 / 规则与周期)+ 底部操作按钮。
//!
//! 表单字段即后端 `SubscriptionUpsertRequest` 的 8 个入参
//! (name/price/currency/durationDays/quota/upgradeGroup/maxPurchases/
//! enabled);后端表不存的字段(副标题、第三方支付 ID、重置周期等)不进
//! 表单——填了存不下、刷新就丢,是误导性 UI。
//!
//! 状态归属:所有表单 signal 由页面(`page::SubscriptionsPage`)持有并传入,
//! 弹窗就地读写;保存/关闭也是页面侧的闭包。弹窗自身不持业务状态、不发网络。

use dioxus::prelude::*;

use super::shared::{
    BTN_CLOSE, BTN_SAVE, FIELD_DURATION, FIELD_ENABLED, FIELD_GROUP, FIELD_LIMIT, FIELD_PLAN_TITLE,
    FIELD_PRICE, LBL_QUOTA, MSG_CURRENCY_HINT, MSG_DURATION_HINT, MSG_GROUP_HINT, MSG_LIMIT_HINT,
    MSG_PH_PLAN_TITLE, MSG_PRICE_HINT, MSG_QUOTA_HINT, MSG_TITLE_HINT, OPT_NO_UPGRADE, TAB_BASIC,
    TAB_RULES, TTL_EDIT, TTL_NEW, ToggleSwitch,
};

/// 订阅编辑/新建弹窗
///
/// 【是什么】模态弹窗:头部(标题随编辑/新建态切换)+ 写请求失败提示 +
/// 两 Tab 切换条 + Tab 体 + 底部关闭/保存按钮。
///
/// 【做什么】把页面持有的表单 signal 双向绑到 8 个后端入参字段;提交按钮
/// 触发 `on_commit`(页面侧组装 `SubscriptionUpsertRequest` 并调真实端点)。
///
/// 【交互逻辑】
/// - 点遮罩或「✕」/「关闭」→ `show_modal.set(false)` 关闭。
/// - 点 Tab 切换条 → `modal_tab.set(0|1)`。
/// - 各输入框 oninput/onchange → 直接写回对应 signal(弹窗与页面共享同一
///   signal 实例,页面侧 commit 读到的就是最新值)。
/// - 点「保存更改」→ `on_commit.call(())`;`saving` 期间按钮禁用并显示
///   「保存中…」。
///
/// 【样式】遮罩 `fixed inset-0 z-50 ... bg-black/70 backdrop-blur-sm`;
/// 弹窗体 `w-full max-w-2xl rounded-2xl border border-border bg-card
/// p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto`。
///
/// 【数据流】
/// - 对内(入):表单 signal(页面持有,见字段文档)+ `editing_idx`(区分
///   编辑/新建标题)+ `saving`/`action_err`(写反馈)+ `group_names`
///   (升级分组下拉的真实分组名)。
/// - 对外(出):`on_commit(())` → 页面 `commit`;signal 的写回经共享
///   signal 直接回流页面。
#[component]
pub fn SubscriptionFormModal(
    /// 弹窗开关(页面持有;遮罩/✕/关闭就地写 false)
    show_modal: Signal<bool>,
    /// 当前激活 Tab(0 = 基本信息,1 = 规则与周期)
    modal_tab: Signal<u8>,
    /// 编辑态行下标(None = 新建);决定标题与副标题文案
    editing_idx: Signal<Option<usize>>,
    /// 写请求在途(保存按钮禁用 + 文案切「保存中…」)
    saving: Signal<bool>,
    /// 写请求失败文案(名称必填 / 后端 400 校验失败)
    action_err: Signal<Option<String>>,
    /// 升级分组下拉候选项(真实分组名;拉取失败时退化为只有「不升级」)
    group_names: Signal<Vec<String>>,
    // ---- Tab 0 基本信息 ----
    f_title: Signal<String>,
    f_price: Signal<String>,
    f_currency: Signal<String>,
    f_quota: Signal<String>,
    // ---- Tab 1 规则与周期 ----
    f_duration: Signal<String>,
    f_group: Signal<String>,
    f_limit: Signal<String>,
    f_enabled: Signal<bool>,
    /// 保存按钮点击出口(页面侧组装请求体并调后端)
    on_commit: EventHandler<()>,
) -> Element {
    // 关闭态不渲染遮罩：否则固定层会永久盖住页面（main 的 if show_modal() 守卫，
    // 拆分时不能丢）
    if !show_modal() {
        return rsx! {};
    }
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm",
            onclick: move |_| show_modal.set(false),
            div {
                class: "w-full max-w-2xl rounded-2xl border border-border bg-card p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto scroll-subtle",
                onclick: move |e| e.stop_propagation(),

                // 弹窗头部
                div { class: "flex items-start justify-between",
                    div {
                        h3 { class: "{ui::TYPE_TITLE}",
                            if editing_idx().is_some() { "{TTL_EDIT}" } else { "{TTL_NEW}" }
                        }
                        p { class: "mt-0.5 {ui::TYPE_DESC}",
                            if editing_idx().is_some() {
                                "保存即按名称更新现有套餐"
                            } else {
                                "新建后可在列表中启停、编辑或删除"
                            }
                        }
                    }
                    button {
                        class: "rounded-lg p-1.5 {ui::C_MUTED} hover:bg-secondary hover:text-foreground transition-colors",
                        onclick: move |_| show_modal.set(false),
                        "✕"
                    }
                }

                // 写请求失败提示(名称必填 / 后端 400 校验失败)
                if let Some(e) = action_err() {
                    div { class: "rounded-lg border border-destructive bg-destructive p-3 text-sm {ui::C_DANGER}",
                        "data-testid": "subscriptions-form-error",
                        "{e}"
                    }
                }

                // 弹窗内部 Tab 切换条(两个 Tab:字段即后端入参,见组件文档)
                div { class: "flex items-center gap-2 border-b border-border pb-2 {ui::TYPE_DESC}",
                    button {
                        class: if modal_tab() == 0 { "rounded-lg bg-secondary px-3 py-1.5 font-semibold text-foreground" } else { "rounded-lg px-3 py-1.5 text-muted-foreground hover:text-foreground" },
                        onclick: move |_| modal_tab.set(0),
                        "{TAB_BASIC}"
                    }
                    button {
                        class: if modal_tab() == 1 { "rounded-lg bg-secondary px-3 py-1.5 font-semibold text-foreground" } else { "rounded-lg px-3 py-1.5 text-muted-foreground hover:text-foreground" },
                        onclick: move |_| modal_tab.set(1),
                        "{TAB_RULES}"
                    }
                }

                // ---- Tab 0: 基本信息 ----
                if modal_tab() == 0 {
                    div { class: "space-y-4 pt-1",
                        label { class: "block space-y-1",
                            span { class: "{ui::TYPE_DESC}", "{FIELD_PLAN_TITLE}" }
                            input {
                                class: "w-full rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_BODY} focus:border-border outline-none",
                                "data-testid": "subscriptions-form-title",
                                value: "{f_title()}",
                                placeholder: MSG_PH_PLAN_TITLE,
                                oninput: move |e| f_title.set(e.value()),
                            }
                            p { class: "{ui::TYPE_LABEL}", "{MSG_TITLE_HINT}" }
                        }
                        div { class: "grid grid-cols-1 sm:grid-cols-3 gap-4",
                            label { class: "block space-y-1",
                                span { class: "{ui::TYPE_DESC}", "{FIELD_PRICE}" }
                                input {
                                    r#type: "number",
                                    step: "0.01",
                                    class: "w-full rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_BODY} focus:border-border outline-none",
                                    "data-testid": "subscriptions-form-price",
                                    value: "{f_price()}",
                                    oninput: move |e| f_price.set(e.value()),
                                }
                                p { class: "{ui::TYPE_LABEL}", "{MSG_PRICE_HINT}" }
                            }
                            label { class: "block space-y-1",
                                span { class: "{ui::TYPE_DESC}", "计价币种" }
                                select {
                                    class: "w-full rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_BODY} focus:border-border outline-none",
                                    "data-testid": "subscriptions-form-currency",
                                    value: "{f_currency()}",
                                    onchange: move |e| f_currency.set(e.value()),
                                    option { value: "CNY", "CNY（¥）" }
                                    option { value: "USD", "USD（$）" }
                                }
                                p { class: "{ui::TYPE_LABEL}", "{MSG_CURRENCY_HINT}" }
                            }
                            label { class: "block space-y-1",
                                span { class: "{ui::TYPE_DESC}", "{LBL_QUOTA}" }
                                input {
                                    r#type: "number",
                                    step: "0.01",
                                    class: "w-full rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_BODY} focus:border-border outline-none",
                                    "data-testid": "subscriptions-form-quota",
                                    value: "{f_quota()}",
                                    oninput: move |e| f_quota.set(e.value()),
                                }
                                p { class: "{ui::TYPE_LABEL}", "{MSG_QUOTA_HINT}" }
                            }
                        }
                    }
                }

                // ---- Tab 1: 规则与周期 ----
                if modal_tab() == 1 {
                    div { class: "space-y-4 pt-1",
                        div { class: "flex items-center justify-between py-2 border-b border-border/80",
                            span { class: "{ui::TYPE_CARD_TITLE}", "{FIELD_ENABLED}" }
                            ToggleSwitch { on: f_enabled(), on_toggle: move |_| f_enabled.set(!f_enabled()) }
                        }
                        div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4 pt-2",
                            label { class: "block space-y-1",
                                span { class: "{ui::TYPE_DESC}", "{FIELD_DURATION}" }
                                input {
                                    r#type: "number",
                                    class: "w-full rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_BODY} focus:border-border outline-none",
                                    "data-testid": "subscriptions-form-duration",
                                    value: "{f_duration()}",
                                    oninput: move |e| f_duration.set(e.value()),
                                }
                                p { class: "{ui::TYPE_LABEL}", "{MSG_DURATION_HINT}" }
                            }
                            label { class: "block space-y-1",
                                span { class: "{ui::TYPE_DESC}", "{FIELD_LIMIT}" }
                                input {
                                    r#type: "number",
                                    class: "w-full rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_BODY} focus:border-border outline-none",
                                    "data-testid": "subscriptions-form-limit",
                                    value: "{f_limit()}",
                                    oninput: move |e| f_limit.set(e.value()),
                                }
                                p { class: "{ui::TYPE_LABEL}", "{MSG_LIMIT_HINT}" }
                            }
                        }
                        label { class: "block space-y-1",
                            span { class: "{ui::TYPE_DESC}", "{FIELD_GROUP}" }
                            select {
                                class: "w-full rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_BODY} focus:border-border outline-none",
                                "data-testid": "subscriptions-form-group",
                                value: "{f_group()}",
                                onchange: move |e| f_group.set(e.value()),
                                option { value: "{OPT_NO_UPGRADE}", "{OPT_NO_UPGRADE}" }
                                for g in group_names.read().iter() {
                                    option { value: "{g}", "{g}" }
                                }
                            }
                            p { class: "{ui::TYPE_LABEL}", "{MSG_GROUP_HINT}" }
                        }
                    }
                }

                // 弹窗底部操作按钮
                div { class: "flex items-center justify-end gap-3 pt-3 border-t border-border",
                    button {
                        class: "rounded-xl border border-border px-4 py-2 {ui::TYPE_DESC} hover:bg-secondary hover:text-foreground transition-colors",
                        onclick: move |_| show_modal.set(false),
                        "{BTN_CLOSE}"
                    }
                    button {
                        class: "rounded-xl bg-warning px-5 py-2 {ui::TYPE_DESC} hover:bg-warning transition-colors shadow-lg shadow-amber-500/10 disabled:opacity-50 disabled:cursor-not-allowed",
                        "data-testid": "subscriptions-form-submit",
                        disabled: saving(),
                        onclick: move |_| on_commit.call(()),
                        if saving() { "保存中…" } else { "{BTN_SAVE}" }
                    }
                }
            }
        }
    }
}
