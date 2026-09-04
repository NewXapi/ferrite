//! 模型调用健康度分析面板。

use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub struct HealthStat {
    pub label: &'static str,
    pub value: &'static str,
    pub sub: &'static str,
}

/// 顶部多维度统计卡片
#[component]
pub fn HealthStats() -> Element {
    let stats = [
        HealthStat { label: "总调用数", value: "3.2M", sub: "近 24 小时" },
        HealthStat { label: "Token 总量", value: "4.6B", sub: "近 24 小时" },
        HealthStat { label: "平均 RPM", value: "1,204", sub: "当前服务" },
        HealthStat { label: "平均 TPM", value: "190.2M", sub: "当前服务" },
        HealthStat { label: "全局限流次数", value: "1,042", sub: "近 24 小时" },
        HealthStat { label: "服务成功率", value: "99.8%", sub: "近 24 小时" },
    ];

    rsx! {
        section { class: "grid grid-cols-2 gap-3 sm:grid-cols-3 md:gap-4",
            for s in stats {
                div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/80",
                    p { class: "text-[11px] font-medium uppercase tracking-wider text-zinc-500", "{s.label}" }
                    p { class: "mt-2 text-2xl font-bold tabular-nums tracking-tight text-zinc-100", "{s.value}" }
                    p { class: "mt-1 text-xs text-zinc-600", "{s.sub}" }
                }
            }
        }
    }
}

/// 模型健康度横幅进度条
#[component]
pub fn HealthMixer() -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4",
            div { class: "mb-3 flex items-center justify-between",
                h3 { class: "text-sm font-medium text-zinc-100", "各模型健康度状态月历" }
                span { class: "text-xs text-zinc-500", "近 42 天" }
            }
            div { class: "space-y-4",
                HealthRow { name: "gpt-5.6-sol", good: 40, warn: 2, bad: 0 }
                HealthRow { name: "claude-fable-5", good: 41, warn: 1, bad: 0 }
                HealthRow { name: "kimi-k3", good: 42, warn: 0, bad: 0 }
                HealthRow { name: "glm-5.2", good: 39, warn: 0, bad: 3 }
                HealthRow { name: "deepseek-v4", good: 41, warn: 1, bad: 0 }
            }
            div { class: "mt-4 flex flex-wrap items-center gap-4 text-[11px] text-zinc-500",
                div { class: "flex items-center gap-1.5", span { class: "h-2 w-2 rounded-full bg-emerald-500" } "运行正常" }
                div { class: "flex items-center gap-1.5", span { class: "h-2 w-2 rounded-full bg-amber-400" } "轻微抖动" }
                div { class: "flex items-center gap-1.5", span { class: "h-2 w-2 rounded-full bg-red-500" } "限流/故障告警" }
            }
        }
    }
}

#[component]
fn HealthRow(name: &'static str, good: u32, warn: u32, bad: u32) -> Element {
    let total = good + warn + bad;
    let mut cells = Vec::with_capacity(total as usize);
    for _ in 0..good { cells.push("bg-emerald-500/90 shadow-[0_0_8px_rgba(16,185,129,0.3)]"); }
    for _ in 0..warn { cells.push("bg-amber-400/90 shadow-[0_0_8px_rgba(251,191,36,0.3)]"); }
    for _ in 0..bad { cells.push("bg-red-500 ring-1 ring-red-400/50 shadow-[0_0_8px_rgba(239,68,68,0.4)]"); }

    rsx! {
        div { class: "flex items-center gap-3",
            span { class: "w-24 shrink-0 truncate text-xs font-medium text-zinc-300", "{name}" }
            div { class: "flex flex-1 gap-0.5",
                for cls in cells {
                    span { class: "h-3.5 flex-1 rounded-[2px] {cls}" }
                }
            }
        }
    }
}
