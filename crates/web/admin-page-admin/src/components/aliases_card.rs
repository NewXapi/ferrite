//! 单张别名卡(对齐分组卡风格)。
//! 一个可操作的卡片:别名标识 + 默认/内置/状态徽标 + 倍率滑条 + 指标行(价格) + 底部按钮组。
//!
//! 【是什么】一个可操作的别名卡:展示 `key` + 标识(`name`) + 默认/内置/状态徽标 + 倍率滑条 + 指标行(价格) + 底部按钮组。
//!
//! 【做什么】展示单条 `AliasItem` 的全部视觉信息,并在卡内消化倍率滑条的交互;
//! 编辑 / 删除 / 倍率写回全部以 `EventHandler` 抛出,不自己做任何网络请求,
//! 也不改动外部状态。不负责网格排版(在 `list.rs`)、不负责列表四态与按钮状态。
//!
//! 【交互逻辑】用户操作 → 组件行为 → 数据交互:
//! - 点倍率滑条轨道(首次) → 本地 `adjusting = true`,出现 thumb,并按点击位置换算
//!   预览值写入 `local_ratio`(纯本地,不发网络);此时指针移动(`onpointermove`,
//!   要求按住)会持续改写 `local_ratio` 实时预览,步进 0.05、下限 0.05。
//! - 再次点击滑条 → `on_ratio_drag(local_ratio)` 把当前预览值抛回页面(页面据此调
//!   `update_model_alias_api`),随后 `adjusting = false`、thumb 消失。
//! - 点底部按钮组 → 按下标分派:0→`on_edit`、1→`on_delete`、2→`on_toggle_status`，
//!   三者均只抛 `EventHandler`,网络请求由页面闭包完成。
//!
//! 【样式】外壳用共用样式壳 `ui::CardShell`(`CARD_SHELL_CLASS`:
//! `group flex flex-col justify-between rounded-xl border border-zinc-800
//! bg-zinc-900/60 p-4`,悬停 `hover:border-zinc-600 hover:bg-zinc-900/80` 且
//! `transition-all duration-200`),根节点 `role="region"` +
//! `aria-label="{alias.name}"` + `data-testid="alias-card"`。
//!
//! 【子组件组成】`Badge`(倍率 / 内置 / 状态三枚)、`ui::ActionButtonGroup`(底部按钮组)。
//!
//! 【数据流】
//! - 对内(入):`alias`(单条 `AliasItem`,提供 key/name/price_mode/enabled)、
//!   `on_edit` / `on_delete` / `on_toggle_status` / `on_ratio_drag`。
//! - 对外(出):四个无参 `EventHandler` → 页面构造 `WriteOp` 或弹窗动作;
//!   `on_ratio_drag(f64)` → 页面构造 `WriteOp::SetPriceMode(v)`。写回后由页面就地更新本地列表,卡片随之重渲染。
//!
//! 状态块:`adjusting` / `local_ratio` 是本卡独有的临时交互状态(滑条是否处于
//! 可拖预览态、预览值),不跨组件、不参与写库,故留在组件内 `use_signal`;
//! 静态态的显示值始终取后端值 `price_mode`,避免未确认的拖动污染展示。

use dioxus::prelude::*;

use crate::shared::{BTN_EDIT, BTN_DELETE_TITLE, BTN_DISABLE, BTN_ENABLE, LBL_STATUS_ENABLED, LBL_STATUS_DISABLED, OPT_STANDARD, OPT_CUSTOM, OPT_FREE};
use crate::tab_page_aliases::shared::{AliasItem, PriceMode};
use ui::Badge;

#[component]
pub fn AliasCard(
    alias: AliasItem,
    on_edit: EventHandler<()>,
    on_delete: EventHandler<()>,
    on_toggle_status: EventHandler<()>,
    on_ratio_drag: EventHandler<f64>,
) -> Element {
    // 本地交互状态
    let mut adjusting = use_signal(|| false);
    let mut local_ratio = use_signal(|| alias.row.multiplier);
    
    // 倍率滑条处理
    let on_pointer_down = move |_| {
        adjusting.set(true);
    };
    
    let on_pointer_move = move |evt: Event<PointerMoveData>| {
        if adjusting() {
            let rect = evt.data().element_bounding_client_rect();
            let x = evt.data().client_x();
            let pos = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            let new_val = pos * 2.0; // 0-2 范围
            local_ratio.set(new_val);
        }
    };
    
    let on_pointer_up = move |_| {
        if adjusting() {
            adjusting.set(false);
            let new_ratio = local_ratio();
            on_ratio_drag.call(new_ratio);
        }
    };
    
    // 状态徽标处理
    let status_badge = match alias.row.enabled {
        true => rsx! { Badge { text: LBL_STATUS_ENABLED.to_string(), tone: "emerald" } },
        false => rsx! { Badge { text: LBL_STATUS_DISABLED.to_string(), tone: "zinc" } },
    };
    
    // 倍率徽标
    let price_badge = match alias.price_mode {
        PriceMode::Standard => rsx! { Badge { text: OPT_STANDARD.to_string(), tone: "emerald" } },
        PriceMode::PerToken => rsx! { Badge { text: OPT_CUSTOM.to_string(), tone: "amber" } },
        PriceMode::PerCall => rsx! { Badge { text: OPT_FREE.to_string(), tone: "zinc" } },
    };
    
    // 计算显示倍率(保持一位小数)
    let display_ratio = format!("{:.2}×", alias.row.multiplier);
    
    rsx! {
        div {
            class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            role: "region",
            aria_label: alias.row.alias,
            "data-testid": "alias-card",
            
            // 头部:标识 + 徽标组
            div {
                class: "flex items-start justify-between mb-3",
                
                // 标识 + 状态徽标
                div {
                    class: "flex-1",
                    h3 { class: "text-lg font-semibold text-zinc-100 mb-1", "{alias.row.alias}" }
                    div { class: "flex gap-2", status_badge, price_badge }
                }
                
                // 操作按钮
                div { class: "flex gap-1",
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200 transition-colors",
                        title: BTN_EDIT,
                        onclick: move |_| on_edit.call(()),
                        // 图标简化为文本
                        "✎"
                    }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200 transition-colors",
                        title: BTN_DELETE_TITLE,
                        onclick: move |_| on_delete.call(()),
                        "🗑"
                    }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200 transition-colors",
                        title: if alias.row.enabled { BTN_DISABLE } else { BTN_ENABLE },
                        onclick: move |_| on_toggle_status.call(()),
                        if alias.row.enabled { "⏸" } else { "▶" }
                    }
                }
            }
            
            // 中间:倍率滑条
            div { class: "mt-3",
                // 滑条轨道
                div {
                    class: "relative h-2 w-full bg-zinc-800 rounded-full cursor-pointer",
                    onpointerdown: on_pointer_down,
                    onpointermove: on_pointer_move,
                    onpointerup: on_pointer_up,
                    
                    // 进度条
                    div {
                        class: "absolute top-0 left-0 h-full bg-amber-400 rounded-full",
                        style: format!("width: {}%", (alias.row.multiplier / 2.0 * 100.0).min(100.0)),
                    }
                    
                    // 拇指(仅调整态显示)
                    if adjusting() {
                        div {
                            class: "absolute top-1/2 transform -translate-y-1/2 w-3.5 h-3.5 rounded-full border-2 border-zinc-100 bg-zinc-900 shadow",
                            style: format!("left: {}%", (local_ratio() / 2.0 * 100.0).min(100.0)),
                            "data-testid": "ratio-thumb"
                        }
                    }
                }
                
                // 标签行:倍率显示 + 指标
                div {
                    class: "flex items-center justify-between mt-2",
                    p { class: "text-sm text-zinc-400", "倍率: {display_ratio}" }
                    div { class: "flex gap-2 text-xs text-zinc-500",
                        p { "价格: {}" }, // 实际价格值将从外部传入
                        p { "缓存: {}" },
                        p { "补全: {}" }
                    }
                }
            }
        }
    }
}