//! 别名定价模式（按量 / 按次）及其分段 toggle。
//!
//! 原先定义在 `admin-page-admin` 的 `tab-page-aliases/shared.rs`，卡片与弹窗
//! 共用；卡片改由本 crate 的 `AliasCard` 承载后，为避免 `ui-components`
//! 反向依赖页面 crate，把这两项上提到此处。
//!
//! 后端 models 域暂无 pricing_mode 列，本模式是 UI 层本地状态，保存路径见页面
//! 的 `AliasFormModal`。

use dioxus::prelude::*;

/// 定价模式：按量（Token 计费）/ 按次（按调用次数计费）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PriceMode {
    /// 按 Token 用量计费。
    PerToken,
    /// 按调用次数计费。
    PerCall,
}

/// 定价模式 toggle：两个分段胶囊，激活段浅色底。
///
/// - `compact`（默认 true）：小号胶囊，用在卡片面板里（不占满，视觉克制）。
/// - 非 compact：全宽，用在编辑弹窗「基本」tab 里。
#[component]
pub fn PriceModeToggle(
    /// 当前激活模式
    active: PriceMode,
    /// 切换回调
    on_change: EventHandler<PriceMode>,
    /// 紧凑模式（卡片用）；默认 true
    #[props(default = true)]
    compact: bool,
) -> Element {
    // 容器：略提亮 zinc-800/60 底；激活段用深色高对比底 + 白字（不依赖渐变
    // 对比，避免浅色字在亮底上看不清）。
    let container_cls = if compact {
        "inline-flex items-center rounded-full border border-border/60 bg-secondary/60 p-0.5 text-[11px] shadow-sm"
    } else {
        "flex w-full overflow-hidden rounded-lg border border-border/60 bg-secondary/60 p-0.5 text-xs shadow-sm"
    };
    let active_cls = if compact {
        "rounded-full bg-primary px-2.5 py-0.5 text-[11px] font-semibold text-primary-foreground shadow-sm transition-colors"
    } else {
        "flex-1 rounded-md bg-primary px-3 py-1.5 text-center font-semibold text-primary-foreground shadow-sm transition-colors"
    };
    let idle_cls = if compact {
        "rounded-full px-2.5 py-0.5 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
    } else {
        "flex-1 rounded-md px-3 py-1.5 text-center text-muted-foreground transition-colors hover:text-foreground"
    };
    rsx! {
        div {
            class: "{container_cls}",
            role: "tablist",
            "aria-label": "定价模式",
            button {
                class: if active == PriceMode::PerToken { "{active_cls}" } else { "{idle_cls}" },
                role: "tab",
                aria_selected: "{active == PriceMode::PerToken}",
                "data-testid": "price-mode-token",
                onclick: move |_| on_change.call(PriceMode::PerToken),
                "按量"
            }
            button {
                class: if active == PriceMode::PerCall { "{active_cls}" } else { "{idle_cls}" },
                role: "tab",
                aria_selected: "{active == PriceMode::PerCall}",
                "data-testid": "price-mode-call",
                onclick: move |_| on_change.call(PriceMode::PerCall),
                "按次"
            }
        }
    }
}
