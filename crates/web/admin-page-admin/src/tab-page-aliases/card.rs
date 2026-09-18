//! 别名卡片:单个别名的概览(分组倍率标签 + 定价行 + 定价模式 toggle)。
//!
//! 纯展示组件:数据以值传入(`AliasRow` + 全部分组),per-card 定价模式切换
//! 通过 `on_mode_change` 抛回页面(写回 `AliasItem::price_mode`);分组可用性
//! 判定见 `shared::usable_groups_for`。

use contract::api::admin::GroupDto;
use dioxus::prelude::*;

use super::shared::{PriceMode, PriceModeToggle, usable_groups_for};
use crate::state::AliasRow;
/// 别名卡片
#[component]
pub fn AliasCard(
    alias_key: String,
    alias: AliasRow,
    index: usize,
    /// 该 card 独立持有的定价模式(per-card,非共享)
    price_mode: PriceMode,
    /// 全部分组(用于展示「哪些分组可用此别名 + 各分组倍率」)
    groups: Vec<GroupDto>,
    /// 各补充通道的启用状态(「基本」tab 胶囊开关控制;未启用通道不出现在卡片)
    c_output_on: bool,
    c_cache_read_on: bool,
    c_cache_write_on: bool,
    c_completion_on: bool,
    on_mode_change: EventHandler<PriceMode>,
    on_edit: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    // 保留参数以维持组件签名,渲染处暂不使用
    let _ = (alias_key, on_edit, on_delete);

    let display_title = if alias.display.is_empty() {
        alias.alias.clone()
    } else {
        alias.display.clone()
    };

    // 分组倍率标签:展示可用此别名的分组及其倍率(白名单语义见 usable_groups_for)。
    let usable_groups = usable_groups_for(&alias.alias, &groups);
    // 卡片空间有限,最多展示 4 个分组标签,超出折叠
    let shown_groups = usable_groups.iter().take(4).collect::<Vec<_>>();
    let overflow_groups = usable_groups.len().saturating_sub(4);

    // 定价:按量 = 输入 + 启用中的补充通道($/1M),按次 = 单项。卡片上放静态默认值
    // 作展示,真实可编辑值在弹窗里(后端无 pricing 列,此处为 UI 占位)。
    // 未启用的通道(开关关闭)不出现在卡片上。
    let price_rows: Vec<(String, String)> = if price_mode == PriceMode::PerCall {
        vec![("单次调用".into(), "0.05".into())]
    } else {
        let mut rows = vec![("输入".into(), "3".into())];
        if c_output_on {
            rows.push(("输出".into(), "15".into()));
        }
        if c_cache_read_on {
            rows.push(("缓存读取".into(), "0.3".into()));
        }
        if c_cache_write_on {
            rows.push(("缓存写入".into(), "0.75".into()));
        }
        if c_completion_on {
            rows.push(("补全".into(), "2.5".into()));
        }
        rows
    };
    // 单位列:按量统一 $/1M,按次为 USD/次
    let shared_unit = if price_mode == PriceMode::PerCall {
        "USD/次".to_string()
    } else {
        "$/1M".to_string()
    };

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            "data-testid": "alias-card",
            div { class: "space-y-3.5",
                // 头部:别名 + 序号
                div { class: "flex items-center justify-between gap-2",
                    div { class: "min-w-0",
                        h3 { class: "truncate text-sm font-medium text-zinc-100", "{alias.alias}" }
                        if !display_title.is_empty() && display_title != alias.alias {
                            p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{display_title}" }
                        }
                    }
                    span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                        "#{index + 1}"
                    }
                }

                // 分组倍率标签:哪些分组可用此别名 + 各分组倍率(替代原计费倍率进度条)
                div { class: "space-y-1.5",
                    p { class: "text-[11px] text-zinc-400", "可用分组 × 倍率" }
                    div { class: "flex flex-wrap gap-1.5",
                        if shown_groups.is_empty() {
                            span { class: "text-[11px] text-zinc-500", "无分组引用" }
                        } else {
                            for (gname, gratio) in shown_groups {
                                {
                                    let ratio_str = format!("×{gratio:.1}");
                                    let tone = if (*gratio - 1.0).abs() < 0.001 {
                                        "border-zinc-700 bg-zinc-800/80 text-zinc-300"
                                    } else if *gratio < 1.0 {
                                        "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
                                    } else {
                                        "border-amber-500/30 bg-amber-500/10 text-amber-300"
                                    };
                                    rsx! {
                                        span { class: "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] {tone}",
                                            "{gname}"
                                            span { class: "text-[10px] font-mono opacity-70", "{ratio_str}" }
                                        }
                                    }
                                }
                            }
                            if overflow_groups > 0 {
                                span { class: "rounded-full border border-zinc-700 bg-zinc-800/60 px-2 py-0.5 text-[11px] text-zinc-400",
                                    "+{overflow_groups}"
                                }
                            }
                        }
                    }
                }

                // 定价:标题行(左) + 模式 toggle(右),同排;价格列表按启用通道渲染
                div { class: "space-y-2",
                    div { class: "flex items-center justify-between gap-2",
                        p { class: "text-[11px] font-medium text-zinc-400",
                            if price_mode == PriceMode::PerCall { "按次定价" } else { "按量定价" }
                        }
                        PriceModeToggle { active: price_mode, on_change: move |m: PriceMode| on_mode_change.call(m) }
                    }
                    if !price_rows.is_empty() {
                        div { class: "space-y-1",
                            for (label, value) in &price_rows {
                                div { class: "flex items-center justify-between gap-2 text-[11px]",
                                    span { class: "shrink-0 text-zinc-500", "{label}" }
                                    div { class: "flex items-baseline gap-1 font-mono",
                                        span { class: "text-zinc-200", "{value}" }
                                        span { class: "text-[10px] text-zinc-500", "{shared_unit}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

