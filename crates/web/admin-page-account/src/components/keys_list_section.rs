//! keys 面板密钥列表区段组件:标题 + 计数 + 新建按钮 + 四态分发 + KeyCard 循环。
//!
//! 一对多的「多」= `KeyCard` 组件(本文件只负责循环实例化与四态分发)；
//! 状态与回调全部由页面层经 props 传入,本组件不发请求、不改状态。

use dioxus::prelude::*;
use ui::button::{Button, ButtonSize, ButtonVariant};

use contract::api::token::TokenDto;

use crate::components::KeyCard;

/// 区段标题文案。
const SEC_KEYS: &str = "我的密钥";

/// 【是什么】密钥列表区段组件，含标题、计数、新建按钮、四态分发与 KeyCard 循环。
///
/// 【做什么】负责渲染「我的密钥」区：标题+计数 badge+新建按钮；四态分发 (错误/加载/空/列表)；列表态下渲染 KeyCard 网格。不负责数据获取与状态写入，仅做展示与回调透传。
///
/// 【交互逻辑】点击新建按钮触发 on_new；KeyCard 内部的编辑/切换/删除通过 on_edit/on_toggle/on_delete 回调透传给父页面。
///
/// 【样式】网格布局：移动端 1 列、平板 3 列、桌面 5 列，间距 gap-3。标题行 flex 双端对齐，计数 badge 灰底圆角。
///
/// 【子组件组成】ui::Button (新建) + KeyCard × N
///
/// 【数据流】通过 props 接收：keys (Vec<TokenDto>)、keys_loaded (Signal<bool>)、keys_err (Signal<String>)、on_new/on_edit/on_toggle/on_delete (EventHandler)。无输出。
#[component]
pub fn KeysListSection(
    keys: Vec<TokenDto>,
    keys_loaded: Signal<bool>,
    keys_err: Signal<String>,
    on_new: EventHandler<()>,
    on_edit: EventHandler<TokenDto>,
    on_delete: EventHandler<TokenDto>,
    on_toggle: EventHandler<TokenDto>,
) -> Element {
    rsx! {
        section {
            id: "keys-sec-keys",
            class: "scroll-mt-8",
            div { class: "space-y-4",
                div { class: "flex items-center justify-between gap-3",
                    div { class: "flex items-center gap-2",
                        h2 { class: "{ui::TYPE_TITLE}", "{SEC_KEYS}" }
                        span { class: "text-xs px-3 py-1 rounded-full bg-zinc-800 text-zinc-400",
                            if keys_loaded() { "{keys.len()} 个" } else { "…" }
                        }
                    }
                    Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        onclick: move |_| on_new.call(()),
                        "✚ 新建密钥"
                    }
                }

                if !keys_err().is_empty() {
                    p { class: "text-sm text-amber-400", "无法加载密钥 (未登录或请求失败): {keys_err()}" }
                } else if !keys_loaded() {
                    p { class: "text-sm text-zinc-500", "加载中…" }
                } else if keys.is_empty() {
                    p { class: "text-sm text-zinc-500", "还没有密钥,点「✚ 新建密钥」签发第一个" }
                } else {
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for t in keys {
                            KeyCard {
                                entry: t,
                                on_edit,
                                on_toggle,
                                on_delete,
                            }
                        }
                    }
                }
            }
        }
    }
}
