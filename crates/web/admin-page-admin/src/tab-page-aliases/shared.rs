//! 模型别名管理共享类型、判定与文案。page / card / modal 三处复用。

use contract::api::admin::GroupDto;
use dioxus::prelude::*;

use crate::state::AliasRow;
use crate::tab_page_groups::parse_whitelist;

pub const SEC_STATS: &str = "别名概览";
pub const SEC_FILTER: &str = "筛选与操作";
pub const SEC_LIST: &str = "别名列表";
/// 别名列表项:后端 ModelView 的 key(UUID) + 页面展示行 + per-card 定价模式。
/// key 不并入 AliasRow — AliasRow 被 entities.rs 结构体字面量构造,
/// 本页独立持有 key 以定位 PUT/DELETE 路径;price_mode 是 UI 层本地状态,
/// 后端 models 域无 pricing_mode 列,保存不写回。
#[derive(Clone, PartialEq)]
pub struct AliasItem {
    pub key: String,
    pub row: AliasRow,
    pub price_mode: PriceMode,
}

/// 弹窗状态(Edit 携带后端模型 UUID key)
#[derive(Clone, PartialEq)]
pub enum AliasModalState {
    Closed,
    New,
    Edit(String),
}

/// 计算「可用此别名的分组及其倍率」。
///
/// 后端 `model_whitelist` 是「分组内可用的模型名列表」;空白名单 = 该分组
/// 可用全部模型。因此「可用此别名」= 白名单为空(默认全可用)或显式包含
/// 该别名。原型新卡与旧卡片网格共用此判定,避免两处逻辑分叉。
pub fn usable_groups_for(alias: &str, groups: &[GroupDto]) -> Vec<(String, f64)> {
    groups
        .iter()
        .filter(|g| {
            let names = parse_whitelist(&g.model_whitelist);
            names.is_empty() || names.iter().any(|n| n.as_str() == alias)
        })
        .map(|g| (g.name.clone(), g.ratio))
        .collect()
}

// ============ 定价模式 toggle（共享组件,卡片与弹窗共用） ============

/// 定价模式:按量(Token 计费) / 按次(按调用次数计费)。
/// 后端 models 域暂无对应列,UI 层本地状态,保存路径见 AliasFormModal。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PriceMode {
    PerToken,
    PerCall,
}

/// 定价模式 toggle:两个分段胶囊,激活段浅色底。
///
/// - `compact`(默认 true):小号胶囊,用在卡片面板里(不占满,视觉克制)。
/// - 非 compact:全宽,用在编辑弹窗「基本」tab 里。
/// 后端 models 域暂无对应列,UI 层本地状态,保存路径见 AliasFormModal。
#[component]
pub fn PriceModeToggle(
    /// 当前激活模式
    active: PriceMode,
    /// 切换回调
    on_change: EventHandler<PriceMode>,
    /// 紧凑模式(卡片用);默认 true
    #[props(default = true)]
    compact: bool,
) -> Element {
    // 容器:略提亮 zinc-800/60 底;激活段用深色高对比底 + 白字(不依赖渐变对比,
    // 避免浅色字在亮底上看不清)。
    let container_cls = if compact {
        "inline-flex items-center rounded-full border border-zinc-700/60 bg-zinc-800/60 p-0.5 text-[11px] shadow-sm"
    } else {
        "flex w-full overflow-hidden rounded-lg border border-zinc-700/60 bg-zinc-800/60 p-0.5 text-xs shadow-sm"
    };
    let active_cls = if compact {
        "rounded-full bg-zinc-100 px-2.5 py-0.5 text-[11px] font-semibold text-zinc-950 shadow-sm transition-colors"
    } else {
        "flex-1 rounded-md bg-zinc-100 px-3 py-1.5 text-center font-semibold text-zinc-950 shadow-sm transition-colors"
    };
    let idle_cls = if compact {
        "rounded-full px-2.5 py-0.5 text-[11px] text-zinc-400 transition-colors hover:text-zinc-200"
    } else {
        "flex-1 rounded-md px-3 py-1.5 text-center text-zinc-400 transition-colors hover:text-zinc-200"
    };
    rsx! {
        div {
            class: "{container_cls}",
            role: "tablist",
            "aria-label": "定价模式",
            button {
                class: if active == PriceMode::PerToken {
                    "{active_cls}"
                } else {
                    "{idle_cls}"
                },
                role: "tab",
                aria_selected: "{active == PriceMode::PerToken}",
                "data-testid": "price-mode-token",
                onclick: move |_| on_change.call(PriceMode::PerToken),
                "按量"
            }
            button {
                class: if active == PriceMode::PerCall {
                    "{active_cls}"
                } else {
                    "{idle_cls}"
                },
                role: "tab",
                aria_selected: "{active == PriceMode::PerCall}",
                "data-testid": "price-mode-call",
                onclick: move |_| on_change.call(PriceMode::PerCall),
                "按次"
            }
        }
    }
}

