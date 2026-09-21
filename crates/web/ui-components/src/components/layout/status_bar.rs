//! StatusBar — 底部细状态条（无背景胶囊，整条一个字高）。
//!
//! 契约（维护者 2026-09-21 批注 + 口头需求）：左下角 = 用户名按钮（无头像，仅文本），
//! 点击弹出名片面板（上拉）：第一行头像+名称（整行可点 → 账户页密钥·资料），第二行
//! 余额·用量纯数值展示（默认文本色，无文字标签，$ 口径同账户页 fmt_quota：
//! 500_000 ≈ $1），第三行 = 账户页 tab 选项（横向 chips，由 `tabs` 传入，点击经
//! `on_open_tab` 回调导航，选中项由 `active_tab` 高亮）；面板末尾保留「退出登录」。
//! 右下角=系统状态纯数字占位（CPU·MEM，含义走 title 悬停提示）。
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

/// 名片面板横向 tab 选项 chip（私有小组件：rsx for 体内不能 let，拆组件最省事）。
#[component]
fn MenuTabChip(idx: usize, label: String, active: bool, onclick: EventHandler<usize>) -> Element {
    let tone = if active {
        "rounded bg-zinc-800 px-1.5 py-0.5 text-[11px] text-zinc-100"
    } else {
        "rounded px-1.5 py-0.5 text-[11px] text-zinc-400 transition-colors hover:bg-zinc-800/60 hover:text-zinc-200"
    };
    rsx! {
        button {
            key: "{idx}",
            class: "{tone}",
            "data-testid": "menu-tab-{idx}",
            onclick: move |_| onclick.call(idx),
            "{label}"
        }
    }
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
    /// 名片面板的横向 tab 选项（账户页 tab 标签，按顺序对应 tab 下标 0..n）。
    #[props(default)]
    tabs: Vec<String>,
    /// 当前选中的面板 tab 下标（信号驱动：chip 高亮直接读信号，始终反映当前 tab，
    /// 不依赖调用方重渲染传值；仅账户页时有意义，-1 = 无选中）。
    active_tab: ReadSignal<i8>,
    /// 面板 tab 选项点击回调（参数 = tab 下标；导航 + 收关由组件内部处理）。
    on_open_tab: EventHandler<u8>,
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
                                    // 维护者批注 2026-09-21: 触发器删掉头像圆点, 只留用户名文本
                                    button {
                                        class: "flex h-5 items-center rounded-full pr-1.5 text-[11px] font-medium text-zinc-300 transition-colors hover:text-zinc-100",
                                        "data-testid": "status-user-menu-button",
                                        "aria-label": "用户菜单",
                                        span { class: "max-w-[140px] truncate", "{name}" }
                                    }
                                },
                                content: rsx! {
                                    // 名片: 第一行 头像+名称, 整行可点 → 账户页密钥·资料
                                    // (2026-09-21: 锚点改 on_open_tab 回调——hash 已是目标值时
                                    // hashchange 不触发, 锚点导航会失效; 回调直接 set 状态必达)
                                    DropdownMenuItem {
                                        "data-testid": "menu-account",
                                        onclick: move |_| {
                                            close_signal.set(true);
                                            on_open_tab.call(0);
                                        },
                                        div { class: "flex items-center gap-2.5",
                                            span {
                                                class: "{card_avatar_class()}",
                                                "{name.chars().next().unwrap_or('?')}"
                                            }
                                            span { class: "min-w-0 truncate text-sm font-medium text-zinc-100", "{name}" }
                                        }
                                    }
                                    // 名片: 第二行 余额·用量 纯数值 (无文字标签, 默认文本色;
                                    // 口径同账户页: 余额=quota-used, 500_000 ≈ $1)
                                    if let Some(u) = read_user_card() {
                                        div {
                                            class: "flex items-center justify-between gap-3 px-2 py-1 font-mono text-xs",
                                            "data-testid": "status-user-card",
                                            span {
                                                title: "余额",
                                                "data-testid": "status-user-balance",
                                                "{fmt_usd(u.quota - u.used_quota)}"
                                            }
                                            span { class: "text-zinc-600", "·" }
                                            span {
                                                title: "用量",
                                                "data-testid": "status-user-usage",
                                                "{fmt_usd(u.used_quota)}"
                                            }
                                        }
                                    }
                                    // 名片: 第三行 账户页 tab 选项 (横向 chips, mark1 批注
                                    // 2026-09-21: 账号页面里面的 tab 选项做成横向选项加入菜单)
                                    if !tabs.is_empty() {
                                        div { class: "flex flex-wrap gap-1 px-2 py-1.5",
                                            for idx in 0..tabs.len() {
                                                MenuTabChip {
                                                    idx,
                                                    label: tabs[idx].clone(),
                                                    active: active_tab() == idx as i8,
                                                    onclick: move |i| {
                                                        close_signal.set(true);
                                                        on_open_tab.call(i as u8);
                                                    },
                                                }
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
