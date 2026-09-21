//! 卡牌内「原地编辑」原语：点值 → 值节点原地变成无边框输入框，草稿留在卡内。
//!
//! 设计来源：Quasar `QInput` 的两个 prop 组合（参考实现克隆在
//! `todo/web-ui-ref/quasar`，`ui/src/components/input/QInput.sfc.vue`）：
//! - `borderless`——不画边框、不改变背景色，输入框无缝融入所在行；
//! - `stack-label`——label 常显，不用 placeholder 充当标签。
//! 与 [`editable`] 的 `EditableRow` 同族（同样零网络、保存抛回页面），差别是
//! **不弹浮层**：编辑态就是行内那个 input 本身。
//!
//! 交互契约（v1，维护者 2026-09-21 确认）：
//! - 点行（label + 值整行可点）→ 值原地变输入框，自动聚焦并全选；
//! - Enter → 退出编辑态；`on_commit` 可选，未传时草稿仅留卡内（"先不提交"）；
//! - Escape → 退出并还原进入编辑前的值（stash 机制）；
//! - blur → 不提交不退出（Quasar 默认的「点开即保存」在卡牌网格里会误触，
//!   留到卡牌外保存按钮上线后再议）。

use dioxus::prelude::*;

use super::editable::EDITABLE_ROW_CLASS;

/// 原地编辑输入框 class：Quasar `borderless` 的等价物——无边框、透明底、无聚焦
/// 环，字号字重与展示态逐字一致，切换时行不抖不跳。
pub const INLINE_INPUT_CLASS: &str =
    "w-full border-none bg-transparent p-0 text-xs font-medium text-zinc-100 outline-none focus:ring-0";

/// 卡牌内的原地可编辑行：展示态是「label + 值」文本行，点击后值的位置变成
/// 无边框输入框（Quasar `borderless` + `stack-label` 的 Dioxus 等价物）。
///
/// 【是什么】「展示即可修改」的最小单元，替代「编辑按钮 → 弹窗」路径。
///
/// 【做什么】行内展示 `label` 与草稿；点击进入编辑态，输入逐键写草稿；
/// Enter 退出（可选抛 `on_commit`）；Escape 退出并还原。
///
/// 【交互逻辑】
/// - 点行 → stash 记下当前草稿 → 进入编辑态 → 聚焦并全选输入框；
/// - 输入 → 逐键写草稿 signal（不发任何事件）；
/// - Enter → `on_commit(草稿)`（若传了回调）+ 退出编辑态；
/// - Escape → 草稿还原为 stash + 退出编辑态；
/// - blur → 不做任何事（v1 不提交，见模块头）。
///
/// 【样式】展示态复用 `EDITABLE_ROW_CLASS`（hover 底色提示可编辑）；编辑态同行
/// 布局，输入框无边框透明底。
///
/// 【数据流】对内(入)：`label` / `value`（外部当前值，仅作草稿初值）/ 可选
/// `on_commit`。对外(出)：`on_commit(String)` 草稿原文；解析与失败提示归调用方。
// ponytail: 草稿与编辑态都留在组件内 signal；卡牌外「保存」按钮上线时提升为
// 调用方持有的 signal（on_commit 已留出口），届时组件改受控即可。
#[component]
pub fn InlineEdit(
    /// 行标签（常显；同时作输入框 `aria-label`）
    label: String,
    /// 外部当前值：仅作草稿初值（组件挂载时同步一次）
    value: String,
    /// 行的测试标识（`data-testid`；输入框为 `{testid}-input`）
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
                // 编辑态：label 常显在左，无边框输入框占满右侧（stack-label 的
                // 卡片行式变体——不把 label 挪到上方，保持行高与展示态一致）。
                div { class: "flex items-center gap-2 rounded-lg px-2 py-1.5",
                    span { class: "shrink-0 text-xs text-zinc-400", "{label}" }
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
                }
            } else {
                // 展示态：整行可点（cursor 由 button 默认提供），hover 底色提示可编辑。
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
