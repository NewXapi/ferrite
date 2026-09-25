//! 密钥卡片 — 数据来自 TokenDto (GET /api/token)。
//! 布局 (维护者批注 2026-09-21): 标题行 (名称截断 + 状态徽标 + ⋯ 菜单) /
//! 密钥行 (掩码占满一行 + 会话明文的复制按钮) / 信息区
//! (额度 hover 明细 → 分组按钮+下拉切换 → 创建时间 → 过期时间)。
//! 编辑·删除收进标题栏 ⋯ 菜单 (原底部三按钮已删); 停用·启用走状态徽标。
//! 分组切换走 PUT /api/token/{key} (group 字段), 成功后由父面板刷新列表。
//! 已用额度走 $ 口径 (fmt_quota, 500_000 ≈ $1, 小数≤1位) + used_pct 进度条,
//! 无限额度显示「无限」徽标且不渲染进度条。
//! 额度 popover 只跟额度行 hover: 卡牌根不挂 group class (旧实现卡牌级 group
//! 让鼠标进卡牌任意区域就弹出), 行内 group 是唯一 hover 源。

use contract::api::token::TokenDto;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonSize, ButtonVariant};
use ui::components::dropdown_menu::{DropdownMenu, DropdownMenuItem, DropdownMenuLabel};

use crate::tab_page_keys::CopyPlaintextButton;
use crate::usage_support::{fmt_quota, used_pct};

#[component]
pub fn KeyCard(
    entry: TokenDto,
    /// 可选分组名 (GET /api/token/auto-groups); 空 = 分组只读展示, 不给切换菜单。
    groups: Vec<String>,
    /// 本次会话创建时拿到的一次性明文 (后端只存 sha256, 刷新即失);
    /// Some 才渲染复制按钮 —— 掩码预览不可复制 (复制了也是废串)。
    plain_key: Option<String>,
    on_edit: EventHandler<TokenDto>,
    on_toggle: EventHandler<TokenDto>,
    on_delete: EventHandler<TokenDto>,
    /// (目标密钥, 新分组名) — 切分组, 由父面板发 PUT 并刷新。
    on_group_change: EventHandler<(TokenDto, String)>,
) -> Element {
    let enabled = entry.status == 1;
    // 维护者批注: 徽标本身可点 (绿=启用 / 红=停用), 与 ⋯ 菜单同一出口语义
    let status_color = if enabled {
        "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-red-500/20 text-red-400 border-red-500/30"
    };
    // 两个下拉各自的「请求关闭」信号 (item 点击后置 true, DropdownMenu 收关)
    let menu_close = use_signal(|| false);
    let mut menu_close_s = menu_close;
    let group_close = use_signal(|| false);
    let mut group_close_s = group_close;
    // 每个 handler 闭包各持一份 clone, 避免多个 move 闭包连环占用 entry
    let e_edit = entry.clone();
    let e_badge = entry.clone();
    let e_del = entry.clone();
    let e_grp = entry.clone();
    // RFC3339 → 展示取日期段 (无数据时不渲染)
    let created: String = entry
        .created_at
        .chars()
        .take(10)
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    // 过期时间: None = 永不过期 (卡牌常驻该行, 空值显示「永不过期」)
    let expiry: Option<String> = entry.expires_at.as_ref().map(|s| {
        s.chars()
            .take(10)
            .filter(|c| c.is_ascii_digit() || *c == '-')
            .collect()
    });
    // 分组展示名: None (跟随用户默认分组) 显示「默认分组」
    let group_label = entry.group.clone().unwrap_or_else(|| "默认分组".into());
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
            // 注意: 卡牌根不挂 `group` class —— 旧实现卡牌级 group 让额度 popover
            // 在鼠标进入卡牌任意区域时就弹出 (维护者批注 2026-09-21:
            // 「悬停到卡牌也出现」)。现在唯一的 group 在额度行上。
            class: ui::CARD_SHELL_CLASS,
            // 标题行: 名称 (截断, 不遮徽标) + 状态徽标 + ⋯ 菜单
            div { class: "mb-3 flex items-center gap-2",
                h3 {
                    class: "min-w-0 flex-1 truncate text-sm font-medium text-zinc-100",
                    title: "{entry.name}",
                    "{entry.name}"
                }
                button {
                    class: "shrink-0 rounded-full border px-2.5 py-0.5 text-xs font-medium transition-[filter] {status_color} hover:brightness-125",
                    "data-testid": "key-status-toggle",
                    "aria-label": if enabled { "停用此密钥" } else { "启用此密钥" },
                    onclick: move |_| on_toggle.call(e_badge.clone()),
                    if enabled { "启用" } else { "停用" }
                }
                // ⋯ 菜单: 编辑 / 删除 (替原底部三按钮; 停用·启用在徽标)
                div { class: "relative shrink-0",
                    DropdownMenu {
                        trigger: rsx! {
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::IconXs,
                                class: "text-zinc-500",
                                "data-testid": "key-menu",
                                "aria-label": "密钥操作",
                                "⋯"
                            }
                        },
                        content: rsx! {
                            DropdownMenuItem {
                                "data-testid": "key-edit",
                                onclick: move |_| {
                                    menu_close_s.set(true);
                                    on_edit.call(e_edit.clone());
                                },
                                "编辑密钥"
                            }
                            DropdownMenuItem {
                                "data-testid": "key-delete",
                                "data-variant": "destructive",
                                onclick: move |_| {
                                    menu_close_s.set(true);
                                    on_delete.call(e_del.clone());
                                },
                                "删除密钥"
                            }
                        },
                        content_class: Some("top-full right-0 w-32".into()),
                        close_signal: Some(menu_close),
                    }
                }
            }

            // 密钥行: 掩码预览占满一行; 复制图标常驻行尾 (维护者批注 2026-09-21 15:44:
            // 「复制图标按钮放到这一行的最右边」)。仅会话内创建的密钥持有明文可复制,
            // 旧密钥明文已不可得 (后端只存 sha256, 掩码复制无意义) → 禁用态+悬停说明。
            div { class: "mb-3 flex items-center gap-2",
                p {
                    class: "min-w-0 flex-1 truncate font-mono text-[11px] text-zinc-500",
                    title: "{entry.key_preview}",
                    "{entry.key_preview}"
                }
                match plain_key {
                    Some(pk) => rsx! {
                        CopyPlaintextButton { text: pk, label: "复制完整密钥".to_string() }
                    },
                    None => rsx! {
                        CopyPlaintextButton {
                            text: String::new(),
                            label: String::new(),
                            disabled_reason: Some("明文仅创建时展示一次, 旧密钥无法复制".into()),
                        }
                    },
                }
            }

            div { class: "space-y-2 text-xs",
                // 额度行 (维护者批注): 删「已用额度」标题 (移进 popover), 两个金额
                // 左右分置 + truncate 折行兜底 —— 旧实现 nowrap 单行撑出卡牌边界。
                // popover 只跟额度行自身 hover: 卡牌根已不挂 group class (旧实现
                // 卡牌级 group 让鼠标进卡牌任意区域就弹出, 维护者批注 2026-09-21),
                // 行内 group 是唯一 hover 源。原点击钉住已删 (点了常驻不消,
                // 触屏场景由 aria-label 兜底)。
                div { class: "group relative flex items-center justify-between gap-2",
                    if unlimited {
                        span {
                            class: "rounded-full border border-sky-500/30 bg-sky-500/20 px-2 py-0.5 text-[11px] font-medium text-sky-300",
                            "无限"
                        }
                    } else {
                        button {
                            class: "flex min-w-0 flex-1 items-center justify-between gap-2 text-left",
                            "data-testid": "key-quota",
                            "aria-label": "已用额度 {fmt_quota(entry.used_quota)}, 总额度 {fmt_quota(entry.quota)}",
                            // 维护者批注 2026-09-21 15:45: 删掉「/」分隔符, 两金额左右分置
                            span { class: "min-w-0 truncate font-medium text-zinc-200", "{fmt_quota(entry.used_quota)}" }
                            span { class: "min-w-0 shrink-0 text-zinc-400", "{fmt_quota(entry.quota)}" }
                        }
                        div {
                            class: "pointer-events-none absolute left-0 top-full z-50 mt-1 rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1 text-[11px] text-zinc-200 opacity-0 shadow-lg transition-opacity duration-150 group-hover:opacity-100",
                            "已用额度 {fmt_quota(entry.used_quota)} · 总额度 {fmt_quota(entry.quota)}"
                        }
                    }
                }
                // 用量进度条: 无限额度不渲染 (无分母, 百分比无意义)
                if !unlimited {
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone}", style: "width: {pct}%" }
                    }
                }
                // 分组行 (维护者批注): 按钮 + 下拉菜单形式切换; 无分组数据时只读展示
                div { class: "flex items-center justify-between gap-2",
                    span { class: "shrink-0 text-zinc-400", "分组" }
                    if groups.is_empty() {
                        span { class: "min-w-0 truncate text-zinc-300", "{group_label}" }
                    } else {
                        div { class: "relative min-w-0",
                            DropdownMenu {
                                trigger: rsx! {
                                    // 维护者批注 2026-09-21 15:49: 值要贴右缘 (原 Ghost 按钮
                                    // 内边距让「default」离右边界有空隙), 换无内边距原生按钮
                                    button {
                                        class: "min-w-0 max-w-[140px] truncate text-xs text-zinc-300 transition-colors hover:text-zinc-100",
                                        "data-testid": "key-group",
                                        "aria-label": "切换密钥分组, 当前 {group_label}",
                                        span { class: "truncate", "{group_label}" }
                                    }
                                },
                                content: rsx! {
                                    DropdownMenuLabel { "切换分组" }
                                    for g in groups.iter().cloned() {
                                        {
                                            let g_text = g.clone();
                                            let e_item = e_grp.clone();
                                            rsx! {
                                                DropdownMenuItem {
                                                    "data-testid": "key-group-option",
                                                    onclick: move |_| {
                                                        group_close_s.set(true);
                                                        on_group_change.call((e_item.clone(), g.clone()));
                                                    },
                                                    "{g_text}"
                                                }
                                            }
                                        }
                                    }
                                },
                                content_class: Some("top-full right-0 w-40".into()),
                                close_signal: Some(group_close),
                            }
                        }
                    }
                }
                if !created.is_empty() {
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 whitespace-nowrap text-zinc-400", "创建时间" }
                        span { class: "whitespace-nowrap font-mono text-zinc-400", "{created}" }
                    }
                }
                // 过期时间行 (维护者批注: 创建时间下方); None = 永不过期
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "过期时间" }
                    span {
                        class: "whitespace-nowrap font-mono text-zinc-400",
                        if let Some(exp) = expiry { "{exp}" } else { "永不过期" }
                    }
                }
            }
        }
    }
}
