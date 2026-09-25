//! 单条会话卡片。`on_revoke` 由父面板发起 DELETE 请求并刷新列表。
//! 「当前设备」走 `on_request_current_revoke` 先弹确认 (吊销即本机立即登出),
//! 其他设备维持行内直接吊销。

use contract::api::user::SessionDto;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

use crate::usage_support::{fmt_time_minute, summarize_ua};

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
                            class: "truncate text-sm text-zinc-200",
                            title: "{session.user_agent}",
                            "{ua_label}"
                        }
                        if session.current {
                            span { class: "shrink-0 rounded-full bg-emerald-500/20 px-2 py-0.5 text-[10px] font-medium text-emerald-400", "当前设备" }
                        }
                    }
                    div { class: "mt-2 grid grid-cols-1 gap-1 text-xs text-zinc-500 sm:grid-cols-2",
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
                    class: "shrink-0 text-red-400",
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
