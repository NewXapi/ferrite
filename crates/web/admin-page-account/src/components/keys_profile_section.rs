//! keys 面板个人资料区段组件:用户名 / 角色头部 + 4 个横向资料项。
//!
//! 纯展示组件——数据由页面层经 props 传入,本组件不发请求、不改状态。

use dioxus::prelude::*;

use contract::api::user::UserDto;

use crate::components::ProfileItem;
use crate::usage_support::short_key;

/// 区段标题文案。
const SEC_PROFILE: &str = "个人资料";

/// 【是什么】个人资料区段组件，展示用户名、角色及横向资料项。
///
/// 【做什么】负责渲染「个人资料」区：用户名/角色头部 + 4 个横向 ProfileItem (显示名/邮箱/用户ID/注册时间)。不负责数据获取，仅做纯展示与加载/错误态分发。
///
/// 【交互逻辑】纯展示，无交互。
///
/// 【样式】外层圆角卡片 (rounded-xl border-zinc-800 bg-zinc-900/60 p-6)；内部 flex-wrap 横向流式布局 (gap-x-14 gap-y-4)。
///
/// 【子组件组成】ProfileItem × 4
///
/// 【数据流】通过 props 接收：user (Option<UserDto>)、self_err (String)、pending (String)。无输出。
#[component]
pub fn KeysProfileSection(user: Option<UserDto>, self_err: String, pending: String) -> Element {
    rsx! {
        section {
            id: "keys-sec-profile",
            class: "scroll-mt-8 space-y-3",
            h2 { class: "{ui::TYPE_TITLE}", "{SEC_PROFILE}" }
            div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6 transition-colors hover:border-zinc-600",
                div { class: "flex items-start justify-between gap-4",
                    div { class: "min-w-0",
                        if let Some(user) = user {
                            div { class: "mb-4 flex items-center gap-2",
                                span { class: "truncate {ui::TYPE_CARD_TITLE}", "{user.username}" }
                                span { class: "shrink-0 rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] font-medium text-zinc-400", "{contract::api::user::role_label(user.role)}" }
                            }
                            div { class: "flex flex-wrap items-baseline gap-x-14 gap-y-4 text-sm",
                                ProfileItem { label: "显示名", value: user.display_name.clone(), copyable: false }
                                ProfileItem { label: "邮箱", value: if user.email.is_empty() { "—".to_string() } else { user.email.clone() }, copyable: false }
                                ProfileItem { label: "用户ID", value: short_key(&user.key), copy_value: Some(user.key.clone()), copyable: true }
                                ProfileItem { label: "注册时间", value: user.created_at.chars().take(10).collect::<String>(), copyable: false }
                            }
                        } else if !self_err.is_empty() {
                            p { class: "text-sm {ui::C_WARNING}", "无法加载用户信息 (未登录或请求失败): {self_err}" }
                        } else {
                            p { class: "text-sm text-zinc-500", "{pending}" }
                        }
                    }
                }
            }
        }
    }
}
