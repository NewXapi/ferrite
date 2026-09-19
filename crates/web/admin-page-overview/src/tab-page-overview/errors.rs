//! 近 24 小时错误面板 — 数据来自真实 `/api/log/errors`（错误流水 log_type=5
//! 按模型聚合，count 降序，见 contract `UsageErrorStatPage`）。
//!
//! 独立信号独立拉取，不阻塞总览面板其它数据；卡形态与三态写法对齐
//! `health.rs` 的渠道健康卡（同宽同风格、诚实空态）。拉取失败走中性占位
//! （卡头合计显示 `—`、列表区一行 muted「暂无数据」），不上失败文案与重试按钮。

use dioxus::prelude::*;

use super::shared::{
    DASH, ERRORS_EMPTY, ERRORS_EMPTY_HINT, ERRORS_LOADING, ERRORS_TOTAL_LABEL, NEUTRAL_NO_DATA,
    SEC_ERRORS,
};
use crate::api;
use contract::api::usage::UsageErrorStatPage;
use ui::components::card::{Card, CardAction, CardContent, CardHeader, CardTitle};

/// 近 24 小时错误卡：卡头（标题 + 合计错误数大数字 + asOf 裸本地时间）+
/// 行列表（左侧 count 比例条 + 模型名 + 次数 + lastSeen 本地时间 HH:MM）。
#[component]
pub fn ErrorsPanel() -> Element {
    let mut page = use_signal(|| None::<UsageErrorStatPage>);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);

    // 进面板自动拉一次(use_effect 无信号依赖 → 仅挂载执行);失败时 err 只留
    // 在内存,不驱动任何 UI 重试(8090 预览反馈①②)。
    use_effect(move || {
        loading.set(true);
        err.set(None);
        spawn(async move {
            // 窗口与条数与后端默认口径一致（hours=24、limit=10，服务端 count 降序）
            match api::errors_api(24, 10).await {
                Ok(p) => {
                    page.set(Some(p));
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let data = page();
    let loading = loading();
    let err = err();
    let items = data.as_ref().map(|p| p.items.as_slice()).unwrap_or(&[]);
    // 合计错误数与最大 count（比例条分母）在 rsx! 之外整形，避免宏内 let
    let total_errors: i64 = items.iter().map(|r| r.count).sum();
    let max_count = items.iter().map(|r| r.count).max().unwrap_or(0);
    // asOf 本地时间裸值（复用 W2 的 as_of_local_time；维护者要求不写「数据截至」字样）
    let as_of_time = data.as_ref().and_then(|p| api::as_of_local_time(&p.as_of));
    // 大数字诚实三态：拉数中 / 失败时没有合计可亮，以 — 占位不亮假 0
    let total_text = if loading || err.is_some() {
        DASH.to_string()
    } else {
        total_errors.to_string()
    };

    rsx! {
        Card {
            hoverable: true,
            CardHeader {
                CardTitle { class: "text-lg text-foreground", "{SEC_ERRORS}" }
                CardAction {
                    div { class: "flex items-center gap-3",
                        // 合计错误数大数字 + asOf 本地时间（裸值，不写「数据截至」）
                        div { class: "text-right", "data-testid": "errors-total",
                            p { class: "text-xl font-semibold leading-none font-mono tabular-nums text-foreground", "{total_text}" }
                            p { class: "mt-0.5 text-[10px] text-muted-foreground", "{ERRORS_TOTAL_LABEL}" }
                        }
                        if let Some(t) = as_of_time {
                            span {
                                class: "text-xs font-mono tabular-nums text-muted-foreground",
                                "data-testid": "errors-as-of",
                                "{t}"
                            }
                        }
                    }
                }
            }
            CardContent {
                section { "data-testid": "errors-panel",
                    class: "space-y-3",
                    // 拉取失败 → 中性占位(8090 预览反馈①):卡头合计已显示 —,
                    // 列表区与空态同风格,一行 muted 小字;失败文案 / HTTP 状态码 /
                    // 重试按钮均不上 UI。重拉时机:本面板随 tab 卸载/重挂(use_effect
                    // 重新执行即重新拉取);err 仅留在内存不渲染。
                    if err.is_some() {
                        div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                            p { class: "text-sm text-zinc-500", "{NEUTRAL_NO_DATA}" }
                        }
                    } else if loading {
                        div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                            p { class: "text-muted-foreground", "{ERRORS_LOADING}" }
                        }
                    } else if items.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                            p { class: "text-muted-foreground", "{ERRORS_EMPTY}" }
                            p { class: "mt-1 text-xs text-muted-foreground/70", "{ERRORS_EMPTY_HINT}" }
                        }
                    } else {
                        for r in items {
                            div { class: "flex items-center gap-3",
                                // count 比例条：宽度 = 相对最大 count 的百分比（zinc 槽 + emerald 填充，手绘）
                                div { class: "h-2 w-14 shrink-0 overflow-hidden rounded-full bg-muted sm:w-20",
                                    if max_count > 0 {
                                        div {
                                            class: "h-full rounded-full bg-emerald-400/80",
                                            style: "width: {(r.count as f64 / max_count as f64 * 100.0):.1}%",
                                        }
                                    }
                                }
                                span { class: "min-w-0 flex-1 truncate text-sm text-foreground", "{r.model_name}" }
                                span { class: "shrink-0 text-xs font-mono tabular-nums text-muted-foreground", "{r.count}" }
                                span { class: "w-12 shrink-0 text-right text-xs font-mono tabular-nums text-muted-foreground/70",
                                    {api::last_seen_local_time(&r.last_seen_at).unwrap_or_else(|| DASH.into())}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
