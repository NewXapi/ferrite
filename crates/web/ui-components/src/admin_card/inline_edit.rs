//! 卡牌内「原地编辑」原语：点行/标题 → 值原地变输入框（标签留原地不浮动），
//! 底部横条展开标示编辑态。编辑块绝对定位覆盖展示行（展示行 invisible 保高），
//! **模式切换零布局位移（CLS=0）**——维护者批注（2026-09-21 f57ad76a /
//! 214aa194）：编辑不得改变卡牌宽高，横条不得覆盖文字。
//!
//! 视觉源自 Quasar `QField` **standard 变体**（参考实现克隆在
//! `todo/web-ui-ref/quasar`，`ui/src/components/field/QField.sass`）：
//! - 底部横条：`.q-field__control:after { height: 2px; transform: scaleX(0);
//! background: currentColor; transition: transform }`，聚焦（--highlighted）时
//!   `scaleX(1)`（`QField.sass:289-323`）——2px 横条从中心展开标示编辑态；
//! - 输入框本体**无盒型边框**（standard 变体只有底栏，没有四周边框）。
//!
//! 注意：Quasar 的「浮动标签」（缩到 75% 浮到输入框上方，`QField.sass:198`）在
//! 卡牌场景会让行高跳变（批注明令禁止），已弃用——标签保持原地不动。
//!
//! 交互契约（v1，维护者 2026-09-21 确认）：
//! - 点行（label + 值整行可点，标题模式整条标题可点）→ 值原地变输入框，底部
//!   横条展开；点行外任意处（遮罩）→ 退出且草稿保留（同 Enter）；
//! - Enter → 退出编辑态；`on_commit` 可选，未传时草稿仅留卡内（"先不提交"）；
//! - Escape → 退出并还原进入编辑前的值（stash 机制）；
//! - blur → 不提交不退出（Quasar 默认的「点开即保存」在卡牌网格里会误触，
//!   留到卡牌外保存按钮上线后再议）。

use super::card::CARD_TITLE_CLASS;
use dioxus::prelude::*;

/// 展示态行 class：与卡内其他只读行（角色 / 状态 / 分组）逐字对齐——无
/// padding、无圆角、无 hover 底色（批注：用户名/邮箱行与后面的列表对不齐）。
/// 可编辑性由 `cursor-pointer` 暗示； Enter 提交语义不变。
pub const INLINE_ROW_CLASS: &str =
    "flex w-full cursor-pointer items-center justify-between gap-2 text-left text-xs";

/// 行模式编辑态输入框：右侧对齐（与展示态值同位），`leading-4 + pb-1` 与标题
/// 模式同款间距——文字抬离底部横条 2px（批注 91631469：两种形态样式要一致，
/// 横线不贴文字底）。
pub const INLINE_ROW_INPUT_CLASS: &str = "min-w-0 flex-1 border-0 bg-transparent p-0 pb-1 text-right text-xs font-medium leading-4 text-zinc-100 outline-none focus:ring-0";

/// 标题模式展示态：整条标题可点，字号/字重/截断与卡牌静态标题逐字同款
/// （`CARD_TITLE_CLASS`），行高固定 20px（`h-5`）——编辑态不撑卡。
pub const INLINE_TITLE_ROW_CLASS: &str = "flex h-5 w-full cursor-pointer items-center text-left";

/// 标题模式输入框：与展示态同字号（text-sm，`leading-4 + pb-1` 凑足 20px 零高度
/// 差、文字抬离横条），仅允许轻微缩小（scale-[0.98]，批注：可以出现一点缩小）。
pub const INLINE_TITLE_INPUT_CLASS: &str = "w-full border-0 bg-transparent p-0 pb-1 text-sm font-medium leading-4 text-zinc-100 outline-none focus:ring-0 scale-[0.98] origin-left transition-transform";

/// 卡牌内的原地可编辑行：展示态是「label + 值」文本行，点击后值的位置变成
/// 输入框——标签留在原地（不浮动、零位移），底部横条展开标示编辑态。
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
    /// 行标签（展示态常显、编辑态留原地不浮动，同时作输入框 `aria-label`）
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
    /// 标题模式：整条标题即编辑入口——无标签、固定 20px 行高（编辑不撑卡）、
    /// 输入框同字号并轻微缩小，底部横条标示编辑态。用于卡牌标题（用户名）。
    #[props(default)]
    title_mode: bool,
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

    // 横条 class：绝对定位贴容器底边（不占布局），展开信号置真后 scaleX 0→1
    // （Quasar :after 机制）。标题/行两模式同款。
    let bar_class = if bar_in() {
        "absolute inset-x-0 bottom-0 h-0.5 origin-center rounded-full bg-zinc-100 transition-transform duration-200 scale-x-100"
    } else {
        "absolute inset-x-0 bottom-0 h-0.5 origin-center rounded-full bg-zinc-100 transition-transform duration-200 scale-x-0"
    };
    // 编辑块：绝对定位、固定 20px 高（与标题模式同款，行模式下探 4px 入栈间隙，
    // 不占布局 → 卡牌宽高零变化），入场淡入不位移（批注 f57ad76a）。
    let edit_class = if bar_in() {
        "absolute inset-x-0 bottom-0 h-5 z-50 flex items-center gap-2 transition-opacity duration-200 opacity-100"
    } else {
        "absolute inset-x-0 bottom-0 h-5 z-50 flex items-center gap-2 transition-opacity duration-200 opacity-0"
    };
    // 展示行编辑时隐身保高（同 AdminCard 面板叠加的 invisible 约定）。
    let display_suffix = if editing() { " invisible" } else { "" };
    // 容器：标题模式固定 20px 行高；行模式随内容。
    let root_class = if title_mode {
        "relative flex h-5 items-center"
    } else {
        "relative"
    };

    rsx! {
        div { class: "{root_class}", id: "{testid}-row",
            // Escape 收关：焦点在行内任意节点按键均可（冒泡到本容器）。
            onkeydown: move |e: KeyboardEvent| {
                if editing() && e.key() == Key::Escape {
                    draft.set(stash());
                    editing.set(false);
                }
            },
            // 展示态（常驻渲染保高；编辑时 invisible，宽高零变化）。
            if title_mode {
                // 标题模式：整条标题即编辑入口（与静态标题同字号同款）。
                button {
                    class: "{INLINE_TITLE_ROW_CLASS} {CARD_TITLE_CLASS}{display_suffix}",
                    "data-testid": "{testid}",
                    "aria-label": "{label}",
                    onclick: move |_| {
                        stash.set(draft());
                        editing.set(true);
                    },
                    span { class: "truncate", "{display}" }
                }
            } else {
                // 行模式：整行可点；行 class 与卡内其他只读行逐字对齐（批注：对不齐）。
                button {
                    class: "{INLINE_ROW_CLASS}{display_suffix}",
                    "data-testid": "{testid}",
                    onclick: move |_| {
                        stash.set(draft());
                        editing.set(true);
                    },
                    span { class: "{crate::C_MUTED}", "{label}" }
                    span { class: "font-medium text-zinc-200 truncate", "{display}" }
                }
            }
            if editing() {
                // 外点收关遮罩（EditableRow 同款，WCAG dismissible）：点行外任意处
                // 退出编辑、草稿保留（同 Enter 语义；批注 214aa194：点其他范围要能退出）。
                div {
                    class: "fixed inset-0 z-40",
                    "aria-hidden": "true",
                    onclick: move |_| editing.set(false),
                }
                // 编辑态：标签留在原地（行模式，不浮动）→ 无盒型边框输入框 → 底部横条。
                div { class: "{edit_class}",
                    if !title_mode {
                        span { class: "shrink-0 {crate::C_MUTED}", "{label}" }
                    }
                    input {
                        class: if title_mode { INLINE_TITLE_INPUT_CLASS } else { INLINE_ROW_INPUT_CLASS },
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
                    // 底部横条：2px，从中心展开（scaleX 0→1）；输入框留了底部间距，
                    // 横条不覆盖文字（批注 214aa194）。
                    div {
                        class: "{bar_class}",
                        "data-testid": "{testid}-bar",
                        "aria-hidden": "true",
                    }
                }
            }
        }
    }
}
