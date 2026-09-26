//! 被邀人列表 — GET /api/affiliate/invitees (真实端点)

use dioxus::prelude::*;

use crate::api::InviteeView;
use crate::usage_support::{fmt_num, fmt_time};

use crate::components::ErrCard;

/// 【是什么】被邀人列表区段组件，展示经邀请链接注册的用户及其累计贡献奖励。
///
/// 【做什么】渲染「被邀请用户」区：标题 + 人数徽标 + 四态分发 (错误 / 骨架 / 空态 / 列表)；每行含首字母头像、昵称 (空则显示「(未命名用户)」)、注册时间与贡献奖励额。不发请求。
///
/// 【交互逻辑】纯展示；行 hover 边框转琥珀色，无点击动作。
///
/// 【样式】卡片 rounded-xl p-6 + hover:border-zinc-600；行 rounded-2xl bg-zinc-950 p-5 gap-4；头像琥珀渐变方块；奖励额 emerald-400 tabular-nums 右对齐。
///
/// 【子组件组成】ErrCard × 0 或 1
///
/// 【数据流】props 接收 invitees (Signal<Option<Vec<InviteeView>>>)、invitees_loaded (Signal<bool>)、invitees_err (Signal<String>)。无输出回调。
#[component]
pub fn InviteesSection(
    invitees: Signal<Option<Vec<InviteeView>>>,
    invitees_loaded: Signal<bool>,
    invitees_err: Signal<String>,
) -> Element {
    rsx! {
        // 被邀人列表
        section { id: "rewards-sec-list", class: "scroll-mt-8 rounded-xl border {ui::T_border_zinc_800} {ui::T_bg_zinc_900} p-6 transition-colors hover:{ui::T_border_zinc_600}",
            div { class: "mb-2 flex items-center justify-between",
                h3 { class: "{ui::T_text_sm} {ui::T_font_medium} {ui::T_text_zinc_200}", "被邀请用户" }
                div { class: "rounded-full {ui::T_bg_zinc_800} px-3 py-1 {ui::T_text_xs} {ui::T_text_zinc_400}",
                    "data-testid": "invitee-count",
                    "{invitees().map_or(0, |v| v.len())} 人"
                }
            }
            if !invitees_err().is_empty() {
                ErrCard { testid: "invitee-error", what: "被邀人", msg: invitees_err() }
            } else if !invitees_loaded() {
                div { class: "space-y-3", "data-testid": "invitee-skeleton",
                    div { class: "h-16 w-full animate-pulse rounded-2xl {ui::T_bg_zinc_800}" }
                    div { class: "h-16 w-full animate-pulse rounded-2xl bg-zinc-800/70" }
                }
            } else if invitees().is_none_or(|v| v.is_empty()) {
                div {
                    class: "rounded-2xl border border-dashed {ui::T_border_zinc_700} bg-zinc-950/40 py-8 text-center",
                    "data-testid": "invitee-empty",
                    p { class: "{ui::T_text_sm} {ui::T_text_zinc_500}", "暂无被邀请用户 (通过链接注册后在此显示)" }
                }
            } else if let Some(rows) = invitees() {
                div { class: "space-y-3",
                    for i in &rows {
                        div { class: "group flex items-center gap-4 rounded-2xl border {ui::T_border_zinc_800} {ui::T_bg_zinc_950} p-5 hover:border-amber-900",
                            "data-testid": format!("invitee-row-{}", i.user_key),
                            div { class: "flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-amber-900 to-zinc-700 {ui::T_text_xl} {ui::T_font_semibold} text-amber-200",
                                "{i.name.chars().next().unwrap_or_default()}"
                            }
                            div { class: "min-w-0 flex-1",
                                div { class: "{ui::T_font_medium} {ui::T_text_zinc_100} group-hover:text-amber-100",
                                    if i.name.is_empty() { "(未命名用户)" } else { "{i.name}" }
                                }
                                div { class: "mt-0.5 {ui::TYPE_DESC}",
                                    "注册时间:{fmt_time(&i.joined_at)}"
                                }
                            }
                            div { class: "text-right",
                                div { class: "{ui::T_font_semibold} {ui::STATE_SUCCESS_TEXT} tabular-nums",
                                    "{fmt_num(i.reward)}"
                                }
                                div { class: "mt-px {ui::T_text_10px} {ui::T_text_zinc_500}", "贡献奖励" }
                            }
                        }
                    }
                }
            }
        }
    }
}
