//! 密钥卡片 — 数据来自 TokenDto (GET /api/token)。
//! 操作: 编辑 (改名) / 停用·启用 (status 1↔2) / 删除, 成功后由父面板刷新列表。
//! 已用额度走 $ 口径 (fmt_quota, 500_000 ≈ $1) + used_pct 进度条,
//! 无限额度显示「无限」徽标且不渲染进度条。

use contract::api::token::TokenDto;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

use crate::usage_support::{fmt_quota, used_pct};

/// 【是什么】单张密钥卡片，展示密钥名称、预览、状态、已用额度、进度条、创建时间及三个操作按钮。
///
/// 【做什么】负责渲染单个 TokenDto 的完整展示与交互区：名称+掩码预览、启用/停用 badge、已用额度 ($ 口径) 与进度条、创建时间、编辑/切换/删除三个按钮。不负责数据获取、列表刷新、弹窗弹出，仅把用户操作通过回调透传给父页面。
///
/// 【交互逻辑】
/// - 点击「编辑」：调用 on_edit(entry)，由父页面打开 EditKeyModal。
/// - 点击「启用/停用」：调用 on_toggle(entry)，由父页面发 PUT /api/token/{key} 切换 status。
/// - 点击「删除」：调用 on_delete(entry)，由父页面打开 DeleteKeyModal。
/// 所有回调均为 fire-and-forget，本组件不关心后续结果。
///
/// 【样式】圆角卡片 (rounded-xl border-zinc-800 bg-zinc-900/60 p-4)，hover 时边框变亮、背景加深；
/// 状态 badge 绿/黄对应启用/停用；进度条颜色分三档：≥90% 红、≥70% 黄、<70% 绿；
/// 底部分隔线 border-t-zinc-800，三个按钮等宽 flex-1，Ghost variant，删除按钮红色文案。
///
/// 【子组件组成】ui::components::button::Button × 3 (编辑/切换/删除)
///
/// 【数据流】
/// - 对内（入）：entry (TokenDto) 含全部展示字段；on_edit/on_toggle/on_delete (EventHandler<TokenDto>) 由父页面传入。
/// - 对外（出）：三个回调各自传回 entry，页面层决定后续动作 (打开弹窗/发 API/刷新列表)。
#[component]
pub fn KeyCard(
    entry: TokenDto,
    on_edit: EventHandler<TokenDto>,
    on_toggle: EventHandler<TokenDto>,
    on_delete: EventHandler<TokenDto>,
) -> Element {
    let enabled = entry.status == 1;
    let status_color = if enabled {
        "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-amber-500/20 text-amber-400 border-amber-500/30"
    };
    // 每个 handler 闭包各持一份 clone, 避免 3 个 move 闭包连环占用 entry
    let e_edit = entry.clone();
    let e_toggle = entry.clone();
    let e_del = entry.clone();
    // RFC3339 → 展示取日期段 (无数据时不渲染)
    let created: String = entry
        .created_at
        .chars()
        .take(10)
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    // 已用额度 $ 口径 (fmt_quota: 500_000 ≈ $1), 内部裸数对用户无意义;
    // 无限额度显示「无限」徽标 (参照 admin-page-users 面板处理, 跨 crate 只看不引)
    let unlimited = entry.unlimited_quota;
    // 进度条: 0..=100; quota <= 0 (无限/未设限额) 时 0, 超用 clamp 100。
    // 配色随用量升高转告警, 样式抄 admin-page-users panel.rs 的 bar_tone 风格
    let pct = used_pct(entry.quota, entry.used_quota);
    let bar_tone = if pct >= 90 {
        "bg-red-500"
    } else if pct >= 70 {
        "bg-amber-500"
    } else {
        "bg-emerald-500"
    };
    rsx! {
        div {
            class: "group rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "mb-3 flex items-start justify-between gap-2",
                div { class: "min-w-0",
                    h3 { class: "truncate text-sm font-medium text-zinc-100", "{entry.name}" }
                    // 掩码预览仅作展示 (完整明文不可再获取), 不提供复制 ——
                    // 复制到的是 `sk-ab****ef` 这类废串, 粘贴必失败。
                    p { class: "min-w-0 truncate font-mono text-[11px] text-zinc-500", "{entry.key_preview}" }
                }
                span {
                    class: "shrink-0 rounded-full border px-2.5 py-0.5 text-xs font-medium {status_color}",
                    if enabled { "启用" } else { "停用" }
                }
            }

            div { class: "space-y-2 text-xs",
                div { class: "flex items-center justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "已用额度" }
                    if unlimited {
                        span {
                            class: "whitespace-nowrap rounded-full border border-sky-500/30 bg-sky-500/20 px-2 py-0.5 text-[11px] font-medium text-sky-300",
                            "无限"
                        }
                    } else {
                        span { class: "whitespace-nowrap font-medium text-zinc-200",
                            "{fmt_quota(entry.used_quota)} / {fmt_quota(entry.quota)}"
                        }
                    }
                }
                // 用量进度条: 无限额度不渲染 (无分母, 百分比无意义)
                if !unlimited {
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone}", style: "width: {pct}%" }
                    }
                }
                if !created.is_empty() {
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 whitespace-nowrap text-zinc-400", "创建时间" }
                        span { class: "whitespace-nowrap font-mono text-zinc-400", "{created}" }
                    }
                }
            }

            div { class: "mt-4 flex items-center gap-2 border-t border-zinc-800 pt-3",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "flex-1 text-zinc-400",
                    onclick: move |_| on_edit.call(e_edit.clone()),
                    "编辑"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "flex-1 text-zinc-400",
                    onclick: move |_| on_toggle.call(e_toggle.clone()),
                    if enabled { "停用" } else { "启用" }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "flex-1 text-red-400",
                    onclick: move |_| on_delete.call(e_del.clone()),
                    "删除"
                }
            }
        }
    }
}
