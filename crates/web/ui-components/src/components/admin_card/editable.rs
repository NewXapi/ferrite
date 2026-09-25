//! 卡牌内「展示行 → 点击 Popover 逐字段编辑」原语。
//!
//! 对应 UI 决策记录（`todo/web-ui-decisions-2026-09-17.md` §2.2/§3.2）：
//! 单字段修改走 Click-triggered Popover，不走 Modal。交互契约按
//! WCAG 2.2 SC 1.4.13（Content on Hover or Focus）满足 Dismissible /
//! Hoverable / Persistent：
//! - Dismissible：Escape 或点击面板外（透明遮罩）收关，不丢焦点上下文；
//! - Hoverable：指针可移入浮层选择文本、点击按钮，浮层不消失；
//! - Persistent：浮层持续到用户主动收关或提交，不自动倒计时关闭。
//!
//! 定位用 `absolute`（相对行容器）+ `z-40` 遮罩 + `z-50` 浮层，不引第三方
//! 定位库；卡牌在网格里被裁剪（`overflow`）时改用调用方追加 class 调整。

use dioxus::prelude::*;

use crate::components::form::component::FormField;
use crate::components::overlay::component::Dialog;

/// 展示行 class：label 左、当前值右，整行可点。
pub const EDITABLE_ROW_CLASS: &str = "flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left text-xs transition-colors hover:bg-zinc-800/60";

/// 危险操作行 class（删除等破坏性入口，红色文本）。
pub const DANGER_ROW_CLASS: &str = "flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left text-xs text-red-300 transition-colors hover:bg-red-950/40";

/// Popover 浮层 class：右对齐、贴行下方、高于网格。
pub const EDIT_POPOVER_CLASS: &str = "absolute right-0 top-full z-50 mt-1 w-64 space-y-3 rounded-xl border border-zinc-700 bg-zinc-900 p-3 shadow-2xl shadow-black/50";

/// 卡牌内的可编辑展示行：点击行弹出 Popover，内含输入框与保存 / 取消。
///
/// 【是什么】「只读行 + 点击编辑」的最小单元，替代原先「编辑按钮 →  Modal」路径。
///
/// 【做什么】行内展示 `label` 与当前 `value`；点击后弹出浮层（`FormField` 输入框 +
/// 保存 / 取消）。保存时把草稿值经 `on_commit` 抛回调用方（由调用方校验与写回），
/// 并置 `open = false` 收关；取消只收关。打开时草稿以最新 `value` 重置。
///
/// 【交互逻辑】
/// - 点行 → `open.toggle()`（`aria-expanded` 同步，供结构化断言）。
/// - 浮层内输入 → 内部草稿 signal 就地更新（不发任何事件）。
/// - 点「保存」→ `on_commit(草稿)` → 调用方写回 → 本组件置 `open=false`。
/// - 点「取消」/ 遮罩 / Escape → 只收关，不提交。
///
/// 【样式】行 hover `bg-zinc-800/60`；浮层 `w-64` 右对齐，`z-50` 高于卡牌网格。
///
/// 【数据流】对内(入)：`label` / `value`（当前值，纯展示）/ `open`（调用方持有的
/// 受控开合 signal）/ `on_commit`。对外(出)：`on_commit(String)` 草稿原文；
/// 解析与失败提示归调用方。
#[component]
pub fn EditableRow(
    /// 字段名（行左侧标签，同时作浮层 aria-label）
    label: String,
    /// 当前值（打开浮层时作为草稿初值）
    value: String,
    /// 受控开合：调用方（卡牌）持有，保存后由本组件置 false
    open: Signal<bool>,
    /// 行的测试标识（`data-testid`）
    testid: String,
    /// 输入框 `name`/`data-testid`（默认取 `testid`）
    #[props(default)]
    input_name: String,
    /// 占位提示
    #[props(default)]
    placeholder: String,
    /// 保存回调：抛出草稿原文（调用方负责解析、校验与写回）
    on_commit: EventHandler<String>,
) -> Element {
    let mut draft = use_signal(|| value.clone());
    // 打开时以最新值重置草稿；value 变化（外部写回成功）也同步。
    // effect 闭包持克隆副本，`value` 本体留给 rsx 展示。
    let value_for_effect = value.clone();
    use_effect(move || {
        if open() {
            draft.set(value_for_effect.clone());
        }
    });

    let input_testid = if input_name.is_empty() {
        testid.clone()
    } else {
        input_name.clone()
    };

    rsx! {
        div { class: "relative",
            // Escape 收关：焦点在触发行或浮层内按键均可（事件冒泡到本容器）。
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Escape {
                    open.set(false);
                }
            },
            button {
                class: EDITABLE_ROW_CLASS,
                "data-testid": "{testid}",
                "aria-expanded": "{open()}",
                onclick: move |_| open.toggle(),
                span { class: "text-zinc-400", "{label}" }
                span { class: "font-medium text-zinc-200", "{value}" }
            }
            if open() {
                // 外点收关遮罩（WCAG dismissible）；浮层在其上，点击浮层不触发。
                div {
                    class: "fixed inset-0 z-40",
                    "aria-hidden": "true",
                    onclick: move |_| open.set(false),
                }
                div {
                    class: EDIT_POPOVER_CLASS,
                    role: "dialog",
                    "aria-label": "{label}",
                    div {
                        FormField {
                            label: label.clone(),
                            name: input_testid,
                            placeholder: placeholder,
                            value: draft(),
                            oninput: move |ev: FormEvent| draft.set(ev.value()),
                        }
                    }
                    div { class: "flex justify-end gap-2",
                        button {
                            class: "rounded-lg border border-zinc-700 px-2.5 py-1 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                            "data-testid": "{testid}-cancel",
                            onclick: move |_| open.set(false),
                            "取消"
                        }
                        button {
                            class: "rounded-lg bg-white px-2.5 py-1 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                            "data-testid": "{testid}-save",
                            onclick: move |_| {
                                on_commit.call(draft.peek().clone());
                                open.set(false);
                            },
                            "保存"
                        }
                    }
                }
            }
        }
    }
}

/// 卡牌内的危险操作行：点击后弹确认 Dialog，确认才触发 `on_confirm`。
///
/// 对应 UI 决策记录 §2.4：危险操作（删除）保留确认弹窗，不与单字段 Popover 混排。
/// 复用 `dialog::Dialog`（遮罩 + 居中卡 + 取消/确认），不另做确认壳。
#[component]
pub fn DangerActionRow(
    /// 行文案（如「删除别名」）
    label: String,
    /// 确认框标题
    confirm_title: String,
    /// 确认框正文（说明后果）
    confirm_detail: String,
    /// 行的测试标识（`data-testid`）
    testid: String,
    /// 确认回调
    on_confirm: EventHandler<()>,
) -> Element {
    let mut confirming = use_signal(|| false);

    rsx! {
        div { class: "relative",
            button {
                class: DANGER_ROW_CLASS,
                "data-testid": "{testid}",
                onclick: move |_| confirming.set(true),
                "{label}"
            }
            if confirming() {
                Dialog {
                    title: confirm_title,
                    open: true,
                    on_confirm: move |_| {
                        on_confirm.call(());
                        confirming.set(false);
                    },
                    on_cancel: move |_| confirming.set(false),
                    "{confirm_detail}"
                }
            }
        }
    }
}
