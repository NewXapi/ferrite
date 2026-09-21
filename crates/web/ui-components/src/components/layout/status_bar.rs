//! StatusBar — 底部细状态条（无背景胶囊，整条一个字高）。
//!
//! 契约（维护者 2026-09-21 批注 + 口头需求）：左下角 = 用户名按钮（头像调大 +
//! 用户名文本），点击弹出名片面板（上拉）：第一行头像+名称（整行可点 → 跳转
//! 账户页，取代原「账户资料」菜单项），第二行余额·用量纯数值展示（无文字标签，
//! 余额绿 / 用量橙，$ 口径同账户页 fmt_quota：500_000 ≈ $1）；面板末尾保留
//! 「退出登录」。右下角=系统状态纯数字占位（CPU·MEM，含义走 title 悬停提示）。
//! 用户下拉复用 crate 的 DropdownMenu（含外部点击/Escape 关闭，选中即关对齐 Radix 默认）。
//! 名片数据读 localStorage `ferrite_current_user`（登录/账户页 /self 刷新时写入），
//! 每次 StatusBar 渲染时重读——额度在页面停留期间的服务端变化要等下一次导航才反映。

use contract::api::user::UserDto;
use dioxus::prelude::*;

use crate::components::dropdown_menu::{DropdownMenu, DropdownMenuItem, DropdownMenuSeparator};
use crate::session::get_storage_item;

/// 底部状态条目：占位名称 + 可选 hint（popover 接入前的静态说明）。
#[derive(Clone, PartialEq)]
pub struct StatusItem {
    /// 占位显示名（如「后端」「版本」）。
    pub label: String,
    /// 可选静态提示（不承载实时数据）。
    pub hint: Option<String>,
}

/// 名片面板大头像 class（32px 圆点 + 首字母，渐变底对齐账户页 UserBadge 观感）。
fn card_avatar_class() -> &'static str {
    "flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-sky-500 to-indigo-600 text-sm font-semibold text-white shadow-sm"
}

/// 内部额度单位 → $ 展示（500_000 ≈ $1，小数 1 位）——口径与账户页 fmt_quota 一致。
fn fmt_usd(quota: i64) -> String {
    format!("${:.1}", quota as f64 / 500_000.0)
}

/// 读 localStorage `ferrite_current_user`（登录/账户页写入的序列化 UserDto）。
/// 解析失败返回 None（此时名片只显示名称行，不渲染余额/用量行）。
fn read_user_card() -> Option<UserDto> {
    serde_json::from_str::<UserDto>(&get_storage_item("ferrite_current_user")?).ok()
}

/// 底部细状态条（无背景，单行文字高度）。
///
/// - `user_name`：登录用户名；Some 时显示用户名按钮+名片下拉，None 时占位「未登录」。
/// - `is_light` / `on_toggle_theme`：主题切换。
/// - `on_logout`：退出登录回调（名片面板用）。
#[component]
pub fn StatusBar(
    /// 登录用户名；None 时显示「未登录」占位。
    user_name: Option<String>,
    /// 当前是否浅色主题。
    is_light: bool,
    /// 切换主题回调。
    on_toggle_theme: EventHandler<()>,
    /// 退出登录回调。
    on_logout: EventHandler<()>,
    /// 状态条目（当前仅作为预留槽位，用不到时传空 vec）。
    #[props(default)]
    items: Vec<StatusItem>,
) -> Element {
    // 外部关闭请求信号（「面板应关闭？」默认 false）：点选中项置 true，DropdownMenu 收关
    let close_request = use_signal(|| false);
    let mut close_signal = close_request;
    rsx! {
        div {
            class: "flex w-full items-center justify-between py-0.5 text-[11px] text-zinc-500",
            // 左下角：用户名按钮 + 名片下拉
            div {
                class: "flex items-center gap-1.5",
                match user_name {
                    Some(name) => rsx! {
                        div {
                            class: "relative",
                            DropdownMenu {
                                trigger: rsx! {
                                    // 维护者批注: 头像调大 + 显示用户名（原先仅 16px 首字母圆点）
                                    button {
                                        class: "flex h-5 items-center gap-1.5 rounded-full pr-1.5 text-[11px] font-medium text-zinc-300 transition-colors hover:text-zinc-100",
                                        "data-testid": "status-user-menu-button",
                                        "aria-label": "用户菜单",
                                        span {
                                            class: "flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-sky-500 to-indigo-600 text-[10px] font-semibold text-white shadow-sm",
                                            "{name.chars().next().unwrap_or('?')}"
                                        }
                                        span { class: "max-w-[140px] truncate", "{name}" }
                                    }
                                },
                                content: rsx! {
                                    // 名片: 第一行 头像+名称, 整行可点 → 账户页
                                    // (维护者批注: 原「账户资料」菜单项删除, 入口改到这里;
                                    //  锚点在 item 内: 点击冒泡到 item 统一收关后跳转)
                                    DropdownMenuItem {
                                        "data-testid": "menu-account",
                                        onclick: move |_| close_signal.set(true),
                                        a {
                                            class: "flex items-center gap-2.5",
                                            href: "#account",
                                            span {
                                                class: "{card_avatar_class()}",
                                                "{name.chars().next().unwrap_or('?')}"
                                            }
                                            span { class: "min-w-0 truncate text-sm font-medium text-zinc-100", "{name}" }
                                        }
                                    }
                                    // 名片: 第二行 余额·用量 纯数值 (无文字标签, 颜色区分;
                                    // 口径同账户页: 余额=quota-used, 500_000 ≈ $1)
                                    if let Some(u) = read_user_card() {
                                        div {
                                            class: "flex items-center justify-between gap-3 px-2 py-1 font-mono text-xs",
                                            "data-testid": "status-user-card",
                                            span {
                                                class: "text-emerald-400",
                                                title: "余额",
                                                "data-testid": "status-user-balance",
                                                "{fmt_usd(u.quota - u.used_quota)}"
                                            }
                                            span { class: "text-zinc-600", "·" }
                                            span {
                                                class: "text-amber-400",
                                                title: "用量",
                                                "data-testid": "status-user-usage",
                                                "{fmt_usd(u.used_quota)}"
                                            }
                                        }
                                    }
                                    DropdownMenuSeparator {}
                                    DropdownMenuItem {
                                        onclick: move |_| {
                                            close_signal.set(true);
                                            on_logout.call(());
                                        },
                                        "data-testid": "logout",
                                        "data-variant": "destructive",
                                        "退出登录"
                                    }
                                },
                                content_class: Some("bottom-full left-0 mb-2 w-56".into()),
                                close_signal: Some(close_request),
                            }
                        }
                    },
                    None => rsx! {
                        span { class: "text-zinc-500", "未登录" }
                    },
                }
            }
            // 右下角：系统状态纯数字占位（CPU · MEM，含义走 title）+ 主题切换
            div {
                class: "flex items-center gap-2",
                span {
                    class: "text-zinc-500",
                    title: "CPU",
                    "data-testid": "status-cpu",
                    "12%"
                }
                span { class: "text-zinc-600", "·" }
                span {
                    class: "text-zinc-500",
                    title: "内存",
                    "data-testid": "status-mem",
                    "34%"
                }
                button {
                    class: "shrink-0 rounded px-1 text-[11px] text-zinc-500 transition-colors hover:text-zinc-300",
                    aria_label: "切换主题",
                    onclick: move |_| on_toggle_theme.call(()),
                    if is_light { "Dark" } else { "Light" }
                }
            }
        }
    }
}
