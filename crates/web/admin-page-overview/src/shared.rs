//! 跨 tab 共用的展示常量、格式化纯函数与共享组件。
//!
//! 只放「多个 tab 目录都要用、且口径必须一致」的项:时间窗档位、模型调色板、
//! tokens 紧凑格式,以及总览趋势面板与排行榜共用的时间窗切换胶囊。tab 专属的
//! 常量与助手仍留在各自 `tab-page-*` 目录内。

use dioxus::prelude::*;

use ui::on_tab_wheel;

/// 时间窗档位:与 `api::window_start` 的窗口语义一一对应
/// (今天=24h 逐时 / 本周=7d 逐天 / 本月=30d 逐天 / 今年=12mo 逐月)。
///
/// 总览趋势面板与排行榜共用同一组档位,任一处改档位都会同时影响两处口径。
pub const TIMEFRAMES: [&str; 4] = ["今天", "本周", "本月", "今年"];

/// 模型调色板(内联 hex, 不走 Tailwind 扫描, 避免 @source 漏扫隐形)。
///
/// 总览趋势图的堆叠段与悬浮卡、排行榜条形色共用同一色板,按模型下标取模;
/// 图表卡(演示数据)同用。
pub const MODEL_COLORS: [&str; 10] = [
    "#3b82f6", "#c4b5fd", "#a78bfa", "#facc15", "#fb8500", "#34d399", "#22d3ee", "#f472b6",
    "#a3e635", "#a1a1aa",
];

/// 时间窗切换胶囊组:总览趋势面板与排行榜共用的四档分段按钮。
///
/// - 是什么:一轮重构前 `trend.rs` 与 `leaderboard/page.rs` 各写一份的切换器,
///   现收敛为单一定义(两处档位与激活态样式口径必须一致)。
/// - 负责什么:渲染四档胶囊,点击写回当前档位;不做取数、不做窗口计算。
/// - 交互逻辑:`timeframe` 由页面持有并以 Signal 双向绑定 —— 组件只就地写回,
///   页面 `use_effect` 依赖该 signal 重拉趋势 / 榜单;组件内部零 `use_signal`。
/// - 样式:zinc-950 底 + zinc-800 边框的胶囊容器,按钮 `rounded-md px-2.5 py-1
///   text-xs`;激活段 `bg-secondary text-foreground shadow-sm`,非激活
///   `text-muted-foreground hover:text-foreground`。
/// - 测试锚点:`testid_prefix` 为 `Some` 时按 `{prefix}-{档位}` 输出
///   `data-testid`(排行榜);为 `None` 时该属性整体不渲染(总览趋势面板原 DOM
///   无 testid,保持逐字不变)。
#[component]
pub fn TimeframeTabs(
    /// 当前档位(页面持有,供取数与窗口计算复用)
    timeframe: Signal<&'static str>,
    /// `data-testid` 前缀;`None` 表示不渲染该属性
    #[props(default)]
    testid_prefix: Option<&'static str>,
) -> Element {
    let tf = timeframe();
    rsx! {
        div { class: "flex items-center gap-1.5 rounded-lg border border-border bg-background p-1",
            // 滚轮竖向滚动 → 循环切换时间窗档位(总览与排行榜共用)
            onwheel: move |e: WheelEvent| {
                let cur = TIMEFRAMES.iter().position(|t| *t == tf).unwrap_or(0);
                on_tab_wheel(e, TIMEFRAMES.len(), cur, |i| timeframe.set(TIMEFRAMES[i]));
            },
            for t in TIMEFRAMES {
                button {
                    key: "{t}",
                    class: "rounded-md px-2.5 py-1 {ui::TYPE_DESC} transition-colors",
                    class: if tf == t { "bg-secondary text-foreground shadow-sm" } else { "text-muted-foreground hover:text-foreground" },
                    onclick: move |_| timeframe.set(t),
                    "data-testid": testid_prefix.map(|prefix| format!("{prefix}-{t}")),
                    "{t}"
                }
            }
        }
    }
}

/// 原始 tokens → 显示串 (K/M/B)。
///
/// 总览趋势面板与排行榜三口径榜共用(同一数字纪律:千分位以上切紧凑单位)。
pub fn fmt_raw(n: i64) -> String {
    let v = n as f64;
    if v.abs() >= 1_000_000_000.0 {
        format!("{:.1}B", v / 1_000_000_000.0)
    } else if v.abs() >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v.abs() >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else {
        n.to_string()
    }
}
