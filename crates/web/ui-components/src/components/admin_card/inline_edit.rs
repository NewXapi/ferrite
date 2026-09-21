//! 卡牌内「原地编辑」原语：点值 → 值节点原地变成输入框，标签浮动缩小，底部横条标示编辑态。
//!
//! 设计来源：Quasar `QField` **standard 变体**（参考实现克隆在
//! `todo/web-ui-ref/quasar`，`ui/src/components/field/QField.sass`）——
//! 维护者描述的「点击后标签变小缩放到输入框头顶 + 底部出现横条」就是它：
//! - 浮动标签：`.q-field--float .q-field__label { transform: translateY(-40%)
//!   scale(.75); transform-origin: left top; transition: transform .36s }`
//!   （`QField.sass:198`）——标签缩到 75% 并浮到输入框上方；
//! - 底部横条：`.q-field__control:after { height: 2px; transform: scaleX(0);
//! background: currentColor; transition: transform }`，聚焦（--highlighted）时
//!   `scaleX(1)`（`QField.sass:289-323`）——2px 横条从中心展开标示编辑态；
//! - 输入框本体**无盒型边框**（standard 变体只有底栏，没有四周边框），深色主题
//!   标签色 `rgba(255,255,255,.7)`、聚焦提亮（`QField.sass:337-347`）。
//!
//! 注意：Quasar 的 `borderless` 变体是**连底栏一起去掉**的那个（`QField.sass:444`），
//! 与维护者要的交互正相反——本原语实现的是 standard 变体。
//!
//! 交互契约（v1，维护者 2026-09-21 确认）：
//! - 点行（label + 值整行可点）→ 值原地变输入框，标签浮动缩小，底部横条展开；
//! - Enter → 退出编辑态；`on_commit` 可选，未传时草稿仅留卡内（"先不提交"）；
//! - Escape → 退出并还原进入编辑前的值（stash 机制）；
//! - blur → 不提交不退出（Quasar 默认的「点开即保存」在卡牌网格里会误触，
//!   留到卡牌外保存按钮上线后再议）。

use dioxus::prelude::*;

use super::editable::EDITABLE_ROW_CLASS;

/// 原地编辑输入框 class：Quasar standard 变体——无盒型边框（四周边框全删），
/// 只有编辑态底部横条（独立元素做 scaleX 动画，见组件内 `bar`）。
pub const INLINE_INPUT_CLASS: &str =
    "w-full border-0 bg-transparent p-0 pb-1 pt-1.5 text-xs font-medium text-zinc-100 outline-none focus:ring-0";

/// 浮动后的标签 class：缩到 ~75%（text-[10px] vs 展示态 text-xs）、上浮到输入框
/// 头顶、左对齐、变暗（Quasar 深色主题 `rgba(255,255,255,.7)` 的 zinc 等价）。
pub const INLINE_LABEL_FLOAT_CLASS: &str =
    "block origin-left truncate text-[10px] leading-none text-zinc-500 transition-all duration-200";

/// 卡牌内的原地可编辑行：展示态是「label + 值」文本行，点击后值的位置变成
/// 输入框——标签浮动缩小到头顶，底部横条展开标示编辑态（Quasar standard 变体）。
///
/// 【是什么】「展示即可修改」的最小单元，替代「编辑按钮 → 弹窗」路径。
///
/// 【做什么】行内展示 `label` 与草稿；点击进入编辑态，输入逐键写草稿；
/// Enter 退出（可选抛 `on_commit`）；Escape 退出并还原。
///
/// 【交互逻辑】
/// - 点行 → stash 记下当前草稿 → 进入编辑态 → 聚焦并全选输入框；
/// - 进入编辑态后一帧 → 横条信号置真（scaleX 0→1 展开动画，Quasar `:after` 机制）；
/// - 输入 → 逐键写草稿 signal（不发任何事件）；
/// - Enter → `on_commit(草稿)`（若传了回调）+ 退出编辑态；
/// - Escape → 草稿还原为 stash + 退出编辑态；
/// - blur → 不做任何事（v1 不提交，见模块头）。
///
/// 【数据流】对内(入)：`label` / `value`（外部当前值，仅作草稿初值）/ 可选
/// `on_commit`。对外(出)：`on_commit(String)` 草稿原文；解析与失败提示归调用方。
// ponytail: 草稿与编辑态都留在组件内 signal；卡牌外「保存」按钮上线时提升为
// 调用方持有的 signal（on_commit 已留出口），届时组件改受控即可。
#[component]
pub fn InlineEdit(
    /// 行标签（展示态常显；编辑态浮动到输入框头顶，同时作输入框 `aria-label`）
    label: String,
    /// 外部当前值：仅作草稿初值（组件挂载时同步一次）
    value: String,
    /// 行的测试标识（`data-testid`；输入框为 `{testid}-input`，横条为 `{testid}-bar`）
    testid: String,
    /// 保存回调（Enter 时抛出草稿原文）；未传时编辑仅落卡内草稿。
    #[props(default)]
    on_commit: Option<EventHandler<String>>,
    /// 值为空时的占位提示（展示态与编辑态 input 共用）
    #[props(default)]
    placeholder: String,
) -> Element {
    // 卡内草稿：展示态显示它，编辑态输入写它；挂载时以外部值播种。
    let mut draft = use_signal(|| value.clone());
    // 受控编辑态 + 进入前 stash（Escape 还原用）。
    let mut editing = use_signal(|| false);
    let mut stash = use_signal(String::new);
    // 横条展开信号：挂载后一帧置真，让 scaleX 0→1 有过渡可走（Quasar :after 机制）。
    let mut bar_in = use_signal(|| false);
    use_effect(move || {
        if editing() {
            bar_in.set(true);
        } else {
            bar_in.set(false);
        }
    });

    // 进入编辑态：聚焦并全选（repo 既有 document::eval 模式，见 dropdown_menu）。
    let testid_for_focus = testid.clone();
    use_effect(move || {
        if editing() {
            let js = format!(
                r#"{{ const el = document.querySelector('[data-testid="{testid_for_focus}-input"]'); if (el) {{ el.focus(); el.select(); }} }}"#
            );
            spawn(async move {
                let _ = document::eval(&js).await;
            });
        }
    });

    let display = if draft().is_empty() {
        placeholder.clone()
    } else {
        draft()
    };

    // 横条 class：展开信号置真后 scaleX 0→1（在 rsx 外算好，避免属性串里嵌 if）。
    let bar_class = if bar_in() {
        "h-0.5 origin-center rounded-full bg-zinc-100 transition-transform duration-200 scale-x-100"
    } else {
        "h-0.5 origin-center rounded-full bg-zinc-100 transition-transform duration-200 scale-x-0"
    };

    rsx! {
        div { class: "relative",
            // Escape 收关：焦点在行内任意节点按键均可（冒泡到本容器）。
            onkeydown: move |e: KeyboardEvent| {
                if editing() && e.key() == Key::Escape {
                    draft.set(stash());
                    editing.set(false);
                }
            },
            if editing() {
                // 编辑态：标签浮动缩小到头顶 → 无盒型边框输入框 → 底部横条（展开动画）。
                div { class: "px-2 py-1.5",
                    span { class: INLINE_LABEL_FLOAT_CLASS, "{label}" }
                    input {
                        class: INLINE_INPUT_CLASS,
                        "data-testid": "{testid}-input",
                        "aria-label": "{label}",
                        value: "{draft}",
                        placeholder: "{placeholder}",
                        oninput: move |ev: FormEvent| draft.set(ev.value()),
                        onkeydown: move |e: KeyboardEvent| {
                            // Enter 提交（v1 仅退出 + 可选回调）；Escape 由容器处理。
                            if e.key() == Key::Enter {
                                e.prevent_default();
                                if let Some(cb) = &on_commit {
                                    cb.call(draft());
                                }
                                editing.set(false);
                            }
                        },
                    }
                    // 底部横条：2px，从中心展开（scaleX 0→1）；Quasar standard 的
                    // control:after（--highlighted 时 scaleX(1)）。
                    div {
                        class: "{bar_class}",
                        "data-testid": "{testid}-bar",
                        "aria-hidden": "true",
                    }
                }
            } else {
                // 展示态：整行可点，hover 底色提示可编辑。
                button {
                    class: EDITABLE_ROW_CLASS,
                    "data-testid": "{testid}",
                    onclick: move |_| {
                        stash.set(draft());
                        editing.set(true);
                    },
                    span { class: "text-zinc-400", "{label}" }
                    span { class: "font-medium text-zinc-200 truncate", "{display}" }
                }
            }
        }
    }
}
