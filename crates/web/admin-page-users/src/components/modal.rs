//! 模态弹窗外壳:遮罩 / 内容卡 / 标题栏 + 关闭按钮(`ui::CloseButton`)。
//!
//! 收敛两个弹窗(用户表单 / 充值)共用的壳结构与行为;壳不含表单字段——
//! 内容经 `children` 槽注入(`UserForm` 与 `TopUpForm` 各管各的)。

use dioxus::prelude::*;

use ui::CloseButton;

/// 弹窗内输入框的统一样式(用户表单与充值表单共用)。
pub const MODAL_INPUT: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none";

/// 模态弹窗外壳。
///
/// 【是什么】居中的模态遮罩 + 内容卡 + 标题栏(标题 + 圆形关闭按钮)。
///
/// 【做什么】只做壳:遮罩点击与关闭按钮触发 `on_close`;内容卡拦截冒泡防止
/// 误关;标题与内容经 props 注入,壳不含表单字段。
///
/// 【交互逻辑】
/// - 点击遮罩 → `on_close`。
/// - 点击关闭按钮 → `on_close`。
/// - 点击内容卡 → 仅拦截冒泡,不关闭。
///
/// 【样式】`fixed inset-0 z-50` 遮罩 + `max-w-md` 圆角内容卡(class 逐字保留)。
///
/// 【子组件组成】`ui::CloseButton`(标题栏关闭按钮)。
///
/// 【数据流】
/// - 对内(入):`title` / `children`。
/// - 对外(出):`on_close`(遮罩与按钮两处触发)。
#[component]
pub fn Modal(title: String, on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        // 遮罩层:点击空白处关闭弹窗
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            // 内容卡:拦截冒泡,卡内点击不触发遮罩的关闭
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),
                // 标题栏:标题 + 关闭按钮(复用 ui::CloseButton)
                // DONE: 关闭按钮不自建,收敛为 ui-components 复用组件 CloseButton(全仓统一 X 按钮) in=demo by=agent
                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "{title}" }
                    CloseButton { on_click: on_close }
                }
                {children}
            }
        }
    }
}
