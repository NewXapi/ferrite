//! 单条会话卡片。`on_revoke` 由父面板发起 DELETE 请求并刷新列表。
//! 「当前设备」走 `on_request_current_revoke` 先弹确认 (吊销即本机立即登出),
//! 其他设备维持行内直接吊销。

use contract::api::user::SessionDto;
use dioxus::prelude::*;
use ui::button::{Button, ButtonSize, ButtonVariant};

use crate::usage_support::{fmt_time_minute, summarize_ua};

/// 【是什么】单条登录会话卡片，展示设备、IP、登录方式与活跃/到期时间，并提供吊销入口。
///
/// 【做什么】渲染一条会话：UA 归纳为「浏览器 · OS」短标签 (完整 UA 挂 title 悬停)、当前设备徽标、IP/登录方式/最后活跃/到期四项明细 (时间悬停显示原始 RFC3339)、右侧吊销按钮。不发请求。
///
/// 【交互逻辑】点吊销：当前设备走 on_request_current_revoke (父面板弹二次确认，因为吊销即本机登出)，其它设备直接走 on_revoke；busy 时按钮禁用防重入。
///
/// 【样式】卡片 rounded-xl bg-zinc-900/60 p-4 + hover:border-zinc-600；短标签 truncate；当前设备 emerald 圆角徽标；明细 12px 灰字两列网格；按钮 Ghost Xs 红字。
///
/// 【子组件组成】ui::button::Button (吊销) × 1
///
/// 【数据流】props 接收 session (SessionDto)、busy (bool)、on_revoke (EventHandler<String>)、on_request_current_revoke (EventHandler<String>)；输出为两个携带 sid 的回调。
#[component]
pub fn SessionRow(
    session: SessionDto,
    busy: bool,
    on_revoke: EventHandler<String>,
    on_request_current_revoke: EventHandler<String>,
) -> Element {
    // UA 归纳为「浏览器 · OS」短标签, 完整 UA 挂 title 悬停可见 (不截断丢信息)。
    let ua_label = summarize_ua(&session.user_agent);
    // 时间展示: 本地时区分钟精度 (含年份); 解析失败时 fmt_time_minute 原样返回,
    // 保证后端异常数据仍诚实可见而不是变成空串。
    let last_active = fmt_time_minute(&session.last_active);
    let expires_at = fmt_time_minute(&session.expires_at);

    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-colors hover:border-zinc-600",
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    div { class: "flex items-center gap-2",
                        p {
                            class: "truncate {ui::TYPE_BODY}",
                            title: "{session.user_agent}",
                            "{ua_label}"
                        }
                        if session.current {
                            span { class: "shrink-0 rounded-full bg-emerald-500/20 px-2 py-0.5 {ui::TYPE_LABEL} {ui::C_SUCCESS}", "当前设备" }
                        }
                    }
                    div { class: "mt-2 grid grid-cols-1 gap-1 {ui::TYPE_DESC} sm:grid-cols-2",
                        span { "IP: {session.ip}" }
                        span { "登录方式: {session.login_method}" }
                        // 时间悬停可见原始 RFC3339 (与上方 UA 短标签同款处理)
                        span { title: "{session.last_active}", "最后活跃: {last_active}" }
                        span { title: "{session.expires_at}", "到期: {expires_at}" }
                    }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "shrink-0 {ui::C_DANGER}",
                    disabled: busy,
                    "data-testid": "revoke-session-{session.sid}",
                    "aria-label": "吊销此会话",
                    onclick: move |_| {
                        if session.current {
                            on_request_current_revoke.call(session.sid.clone());
                        } else {
                            on_revoke.call(session.sid.clone());
                        }
                    },
                    "吊销"
                }
            }
        }
    }
}
