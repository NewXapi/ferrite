//! 四区 dock 布局渲染与拖拽逻辑。
//!
//! 结构：左列 / 中央 editor 槽 / 右 dock。左右列宽为像素定宽（0 = 折叠，
//! 非 0 由纯函数 clamp 到 180..=480px）；列内上下二分（比例来自
//! [`crate::dock`] 的 `SplitRatio`）。列间与列内分割线均为 tolaria 式
//! 「透明悬停显形」条：平时几乎不可见（`bg-transparent`），悬停整条亮起；
//! 拖动调整宽度/比例，单击（位移 ≤4px）折叠或展开该列。
//!
//! 右 dock 有两种呈现模式（[`tavern_state::dock::DockMode`]）：
//! - `Side`：侧挂，占 flex 位（行为与早期一致）；
//! - `Floating`：不占位，渲染为贴右缘的绝对定位浮层盖在聊天区上，
//!   右上角小钮（`btn-dock-mode`）单击回到 Side 模式。
//!
//! 面板跨区拖拽事件链：面板标题栏 `mousedown` 记录被拖项与源区 →
//! 根容器 `mousemove` 置移动标记 → 根容器 `mouseup` 按指针坐标经
//! [`hit_zone`] 判定落入哪个区（浮层模式下右列区域按浮层右缘像素宽判定，
//! 同样可停靠）：跨区调 [`crate::dock::move_panel`]，同区调
//! [`crate::dock::reorder_zone`]。

use dioxus::prelude::*;
use tavern_state::dock::{COL_DEFAULT_PX, DockMode, Side, Zone};
use tavern_ui::icons::IconDock;

use crate::dock;

/// 拖窄到此像素以下视为折叠意图（列宽 0）。
///
/// 低于 [`tavern_state::dock::COL_MIN_PX`] 的宽度本就会被纯函数 clamp 抬回
/// 下限，无法表达「更窄」，所以用这个介于 0 与下限之间的阈值把「拖到很窄」
/// 显式翻译成折叠，与纯函数层的约定（折叠要显式传 0）对齐。
const COL_COLLAPSE_PX: f64 = 60.0;

/// 中央区挂载时缓存的 client rect，拖拽命中测试用。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChatRect {
    /// 左上角 x。
    pub x: f64,
    /// 左上角 y。
    pub y: f64,
    /// 宽度。
    pub width: f64,
    /// 高度。
    pub height: f64,
}

/// 拖拽中的面板（标题栏 mousedown 写入，mouseup 提交）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PanelDrag {
    /// 被拖面板稳定 id。
    pub item_id: &'static str,
    /// 按下时 client x。
    pub sx: f64,
    /// 按下时 client y。
    pub sy: f64,
    /// 按下时所在区（跨区/同区判定的源区）。
    pub from_zone: Zone,
    /// 按下时区内启用项索引。
    pub from_index: usize,
    /// 是否真正移动过（区分点按与拖拽）。
    pub moved: bool,
}

/// 分割线拖拽侧。
#[derive(Clone, Copy, PartialEq, Debug)]
enum SplitSide {
    Left,
    Right,
}

/// 竖向列分割线侧（左列宽/右列宽）。
#[derive(Clone, Copy, PartialEq, Debug)]
enum ColSide {
    Left,
    Right,
}

/// 拖宽条基础样式：tolaria 式透明悬停显形（平时不可见，悬停整条亮起）。
///
/// `hover:bg-zinc-700` 是悬停显形的等价类：预生成的 tailwind.out.css 里没有
/// `hover:bg-border`/`hover:bg-zinc-500/40`，zinc-700 实心灰在深底上最接近
/// 规格描述的半透明显形效果。
const REVEAL_BAR: &str = "bg-transparent transition-colors hover:bg-zinc-700";

/// 把 client 坐标映射为「(所在区, 区内序号)」：
/// - x 落在左列/右列的像素宽度内，再按根容器高度中点上下二分；
/// - 列折叠（宽 0）时该侧不可停靠；中央区不可停靠，返回 `None`。
///
/// 右 dock 浮层模式的浮层贴根容器右缘（`right-0`），停靠区域同样是
/// 「右缘向左 col_right 像素」，判定公式与侧挂模式一致。
fn hit_zone(
    rect: &ChatRect,
    col_left_px: u16,
    col_right_px: u16,
    cx: f64,
    cy: f64,
) -> Option<(Zone, usize)> {
    let in_left = col_left_px > 0 && cx < rect.x + col_left_px as f64;
    let in_right = col_right_px > 0 && cx > rect.x + rect.width - col_right_px as f64;
    let top = cy < rect.y + rect.height * 0.5;

    if in_left {
        return Some((if top { Zone::LeftTop } else { Zone::LeftBottom }, 0));
    }
    if in_right {
        return Some((
            if top {
                Zone::RightTop
            } else {
                Zone::RightBottom
            },
            0,
        ));
    }
    None
}

/// 区名（testid/aria 用）。
fn zone_label(zone: Zone) -> &'static str {
    match zone {
        Zone::LeftTop => "left-top",
        Zone::LeftBottom => "left-bottom",
        Zone::RightTop => "right-top",
        Zone::RightBottom => "right-bottom",
    }
}

/// 渲染某区的启用面板堆（区内 order 升序），每项标题栏可按下发起拖拽。
fn render_zone(
    zone: Zone,
    character_panel: &Element,
    sessions_panel: &Element,
    prompt_panel: &Element,
    model_panel: &Element,
    panel_drag: &mut Signal<Option<PanelDrag>>,
) -> Element {
    let ids = dock::zone_item_ids(zone);
    let zone_c = zone;
    let mut children: Vec<Element> = Vec::new();
    for id in ids {
        let item_name: &'static str = match id.as_str() {
            "character" => "character",
            "sessions" => "sessions",
            "prompt" => "prompt",
            "model" => "model",
            _ => "unknown",
        };
        let el = match item_name {
            "character" => character_panel.clone(),
            "sessions" => sessions_panel.clone(),
            "prompt" => prompt_panel.clone(),
            "model" => model_panel.clone(),
            _ => rsx! {
                div {
                    class: "flex min-h-0 flex-1 flex-col items-center justify-center rounded-lg border border-zinc-800/40 bg-zinc-950/40 text-[10px] text-zinc-600",
                    "（空面板）"
                }
            },
        };
        let item_origin = dock::item_origin(item_name).unwrap_or((zone_c, 0));
        let mut drag_c = *panel_drag;

        let el_c = el.clone();
        let item_key = item_name.to_string();
        children.push(rsx! {
            div {
                key: "dock-item-{item_key}",
                class: "flex min-h-0 flex-1 flex-col",
                "data-testid": format!("dock-item-{item_name}"),
                onmousedown: move |e: MouseEvent| {
                    e.stop_propagation();
                    let c = e.client_coordinates();
                    drag_c.set(Some(PanelDrag {
                        item_id: item_name,
                        sx: c.x,
                        sy: c.y,
                        from_zone: item_origin.0,
                        from_index: item_origin.1,
                        moved: false,
                    }));
                },
                { el_c.clone() }
            }
        });
    }

    let children_v = children.clone();
    let zone_label_c = zone_label(zone_c);
    rsx! {
        div {
            // h-full：撑满上下分割容器，否则分区塌成内容高
            class: "flex h-full min-h-0 flex-col gap-1",
            "data-testid": "dock-zone-{zone_label_c}",
            { children_v.iter() }
        }
    }
}

/// 渲染四区 dock 骨架：左列 / 中央 editor 槽 / 右 dock（侧挂列或浮层）。
///
/// `editor` 是中央区内容（工具栏 + 消息流 + composer，行为不变）；
/// 四个面板元素按 `DOCK` 信号所在区落位，区内按 order 升序堆叠。
#[component]
pub fn DockFrame(
    /// 中央区 editor 槽。
    editor: Element,
    /// 角色卡面板元素（默认停靠左上区，实际落区由 `DOCK` 信号决定）。
    left_top: Element,
    /// 会话时间线面板元素（默认停靠左下区，实际落区由 `DOCK` 信号决定）。
    left_bottom: Element,
    /// 模型面板元素（默认停靠右下区，实际落区由 `DOCK` 信号决定）。
    right_bottom: Element,
    /// prompt 导航面板元素（默认停靠右上区，实际落区由 `DOCK` 信号决定）。
    right_top: Element,
) -> Element {
    dock::use_dock_state();

    let dock_layout = use_memo(move || dock::DOCK.read().clone());
    let split_left = dock_layout().split_left.value();
    let split_right = dock_layout().split_right.value();
    // 列宽：像素定宽，0 = 该列折叠（非 0 已由纯函数 clamp 到 180..=480）。
    let col_left = dock_layout().col_left;
    let col_right = dock_layout().col_right;
    let floating = dock_layout().mode == DockMode::Floating;

    // prop 名只对应默认停靠区：面板可跨区拖动，render_zone 按 DOCK 信号的
    // 实际落区取用这四个元素，槽位名只是传参通道，不锁定面板位置。
    let character_panel = left_top;
    let sessions_panel = left_bottom;
    let prompt_panel = right_top;
    let model_panel = right_bottom;

    let mut rect = use_signal(|| None::<ChatRect>);
    let mut panel_drag = use_signal(|| None::<PanelDrag>);
    let mut split_drag = use_signal(|| None::<(SplitSide, f64)>);
    // (侧, 按下 x, 按下时该侧列宽 px)：拖拽按位移增量算新宽，避免逐帧读回
    // 已 clamp 的值造成抖动。
    let mut col_drag = use_signal(|| None::<(ColSide, f64, u16)>);

    // 列分割线 mousedown：记录侧、起点 x 与该侧当前像素宽。
    // 单击（无移动）折叠/展开在 mouseup 处理。
    let start_col = {
        let mut col_drag = col_drag;
        move |side: ColSide| {
            move |e: MouseEvent| {
                e.stop_propagation();
                let c = e.client_coordinates();
                let start_px = match side {
                    ColSide::Left => dock::DOCK.read().col_left,
                    ColSide::Right => dock::DOCK.read().col_right,
                };
                col_drag.set(Some((side, c.x, start_px)));
            }
        }
    };

    // 分割线 mousedown：记录起点与初始比例，mousemove 持续改 split。
    let start_split = {
        let mut split_drag = split_drag;
        move |side: SplitSide| {
            move |e: MouseEvent| {
                e.stop_propagation();
                let c = e.client_coordinates();
                let _ = c;
                let ratio = match side {
                    SplitSide::Left => dock::DOCK.read().split_left.value() as f64,
                    SplitSide::Right => dock::DOCK.read().split_right.value() as f64,
                };
                split_drag.set(Some((side, ratio)));
            }
        }
    };

    let on_root_move = {
        // panel_drag 在闭包里经 with_mut 写入需要 mut 重绑定；
        // col_drag 只读、split_drag/rect 只读，直接捕获外层绑定即可。
        let mut panel_drag = panel_drag;
        let col_drag = col_drag;
        move |e: MouseEvent| {
            let c = e.client_coordinates();

            // 竖向列分割线拖拽：按下时的像素宽 + 水平位移增量算新宽，
            // 拖到 <60px 视为折叠（传 0）。
            if let Some((side, sx, start_px)) = *col_drag.read() {
                // 未超过 4px 视为点按，不进入拖拽改宽
                if (c.x - sx).abs() <= 4.0 {
                    return;
                }
                let dx = c.x - sx;
                let new_px = match side {
                    ColSide::Left => start_px as f64 + dx,
                    ColSide::Right => start_px as f64 - dx,
                };
                let new_px = if new_px < COL_COLLAPSE_PX {
                    0
                } else {
                    new_px.round().clamp(0.0, u16::MAX as f64) as u16
                };
                let (cl, cr) = {
                    let l = dock::DOCK.read();
                    (l.col_left, l.col_right)
                };
                match side {
                    ColSide::Left => dock::set_cols(new_px, cr),
                    ColSide::Right => dock::set_cols(cl, new_px),
                }
                return;
            }

            // 分割线拖拽中：按列内 y 比例改 split
            if let Some((side, _start)) = *split_drag.read() {
                let Some(r) = *rect.read() else {
                    return;
                };
                let col_h = r.height;
                let ratio = ((c.y - r.y) / col_h.max(1.0)).clamp(0.05, 0.95);
                dock::set_side_split(side.into(), ratio as f32);
                return;
            }

            // 面板拖拽：只置移动标记，落区在 mouseup 时按坐标判定
            panel_drag.with_mut(|v| {
                if let Some(d) = v {
                    d.moved = (c.x - d.sx).abs() + (c.y - d.sy).abs() > 4.0;
                }
            });
        }
    };

    let on_root_up = {
        // split_drag/col_drag 在闭包里 set 需要 mut 重绑定；
        // panel_drag/rect 只读，直接捕获外层绑定即可。
        let mut split_drag = split_drag;
        let mut col_drag = col_drag;
        move |e: MouseEvent| {
            split_drag.set(None);

            // 列分割线单击（无移动）：切换该列折叠/展开（展开恢复默认像素宽）
            if let Some((side, sx, _start_px)) = *col_drag.read() {
                let c = e.client_coordinates();
                if (c.x - sx).abs() <= 4.0 {
                    let (l, r) = {
                        let cur = dock::DOCK.read();
                        (cur.col_left, cur.col_right)
                    };
                    match side {
                        ColSide::Left => {
                            let nl = if l == 0 { COL_DEFAULT_PX } else { 0 };
                            dock::set_cols(nl, r);
                        }
                        ColSide::Right => {
                            let nr = if r == 0 { COL_DEFAULT_PX } else { 0 };
                            dock::set_cols(l, nr);
                        }
                    }
                }
            }
            col_drag.set(None);

            let pd = *panel_drag.read();
            let Some(pd) = pd else {
                return;
            };
            if !pd.moved {
                return;
            }

            let c = e.client_coordinates();
            let Some(r) = *rect.read() else {
                return;
            };
            let (cl, cr) = {
                let cur = dock::DOCK.read();
                (cur.col_left, cur.col_right)
            };
            if let Some((target, target_index)) = hit_zone(&r, cl, cr, c.x, c.y) {
                if target == pd.from_zone {
                    // 同区：区内重排（追加到目标索引处）
                    dock::reorder_zone(target, pd.from_index, target_index + 1);
                } else {
                    // 跨区：move_item 追加到目标区末尾
                    dock::move_panel(pd.item_id, target);
                }
            }
        }
    };

    let left_top_stack = render_zone(
        Zone::LeftTop,
        &character_panel,
        &sessions_panel,
        &prompt_panel,
        &model_panel,
        &mut panel_drag,
    );
    let left_bottom_stack = render_zone(
        Zone::LeftBottom,
        &character_panel,
        &sessions_panel,
        &prompt_panel,
        &model_panel,
        &mut panel_drag,
    );
    let right_top_stack = render_zone(
        Zone::RightTop,
        &character_panel,
        &sessions_panel,
        &prompt_panel,
        &model_panel,
        &mut panel_drag,
    );
    let right_bottom_stack = render_zone(
        Zone::RightBottom,
        &character_panel,
        &sessions_panel,
        &prompt_panel,
        &model_panel,
        &mut panel_drag,
    );

    let split_left_c = split_left;
    let split_right_c = split_right;

    // 右 dock 的面板栈（上下二分 + 分割线）：侧挂列与浮层共用同一份内容。
    let right_stack = rsx! {
        div {
            class: "flex min-h-0 flex-col",
            style: "height: {split_right_c as f64 * 100.0}%",
            { right_top_stack }
        }

        SplitLine {
            side: SplitSide::Right,
            on_mousedown: move |e: MouseEvent| start_split(SplitSide::Right)(e),
        }

        div {
            class: "flex min-h-0 flex-1 flex-col",
            { right_bottom_stack }
        }
    };

    rsx! {
        div {
            // relative：Floating 模式下右 dock 浮层的定位参照
            class: "relative flex h-full w-full min-h-0 min-w-0",
            role: "presentation",
            aria_label: "dock 布局",
            onmounted: move |e| async move {
                if let Ok(r) = e.get_client_rect().await {
                    rect.set(Some(ChatRect {
                        x: r.origin.x,
                        y: r.origin.y,
                        width: r.size.width,
                        height: r.size.height,
                    }));
                }
            },
            onmousemove: on_root_move,
            onmouseup: on_root_up,
            onmouseleave: move |_| {
                panel_drag.set(None);
                split_drag.set(None);
                col_drag.set(None);
            },

            // 左列：上下二分（列宽像素内联 style，来自 DOCK.col_left；0=折叠隐藏）
            { if col_left > 0 {
                rsx! {
                    div {
                        class: "flex h-full shrink-0 flex-col gap-1 py-1 pr-1",
                        style: "width: {col_left}px",
                        role: "group",
                        aria_label: "左列面板",

                        div {
                            class: "flex min-h-0 flex-col",
                            style: "height: {split_left_c as f64 * 100.0}%",
                            { left_top_stack }
                        }

                        SplitLine {
                            side: SplitSide::Left,
                            on_mousedown: move |e: MouseEvent| start_split(SplitSide::Left)(e),
                        }

                        div {
                            class: "flex min-h-0 flex-1 flex-col",
                            { left_bottom_stack }
                        }
                    }
                }
            } else { rsx! { div {} } } }

            // 左竖向列分割线：拖动调宽，单击折叠/展开左列（折叠态为悬停显形恢复条）
            VColLine {
                side: ColSide::Left,
                collapsed: col_left == 0,
                on_mousedown: move |e: MouseEvent| start_col(ColSide::Left)(e),
            }

            // 中央 editor 槽
            div { class: "flex h-full min-h-0 min-w-0 flex-1 flex-col", { editor } }

            // 右 dock：Side = 侧挂（分割线 + 占位列）；Floating = 贴右缘绝对
            // 定位浮层（不占 flex 位，盖在聊天区上）。浮层内面板的拖拽事件沿
            // 冒泡到根容器处理，停靠命中按右缘 col_right 像素宽判定。
            { if floating {
                if col_right > 0 {
                    rsx! {
                        div {
                            class: "absolute bottom-0 right-0 top-0 z-30 flex flex-col gap-1 border-l border-zinc-800/60 bg-zinc-950/95 pl-1 py-1 shadow-2xl backdrop-blur-xl",
                            style: "width: {col_right}px",
                            role: "group",
                            aria_label: "右 dock 浮层",
                            "data-testid": "dock-floating",

                            { right_stack }

                            // 右上角回停小钮：单击切回 Side 模式
                            button {
                                class: "absolute right-2 top-2 z-40 flex h-6 w-6 items-center justify-center rounded-md border border-zinc-800/60 bg-zinc-900/80 text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                                title: "停靠回侧栏",
                                aria_label: "停靠回侧栏",
                                name: "btn-dock-mode",
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    dock::toggle_mode();
                                },
                                IconDock { size: 14 }
                            }
                        }
                    }
                } else {
                    // 浮层折叠态：贴右缘的悬停显形恢复条，单击展开
                    rsx! {
                        div { class: "absolute bottom-0 right-0 top-0",
                            VColLine {
                                side: ColSide::Right,
                                collapsed: true,
                                on_mousedown: move |e: MouseEvent| start_col(ColSide::Right)(e),
                            }
                        }
                    }
                }
            } else {
                rsx! {
                    // 右竖向列分割线：拖动调宽，单击折叠/展开右列
                    VColLine {
                        side: ColSide::Right,
                        collapsed: col_right == 0,
                        on_mousedown: move |e: MouseEvent| start_col(ColSide::Right)(e),
                    }

                    // 右列：上下二分（列宽像素内联 style，来自 DOCK.col_right；0=折叠隐藏）
                    { if col_right > 0 {
                        rsx! {
                            div {
                                class: "flex h-full shrink-0 flex-col gap-1 py-1 pl-1",
                                style: "width: {col_right}px",
                                role: "group",
                                aria_label: "右列面板",

                                { right_stack }
                            }
                        }
                    } else { rsx! { div {} } } }
                }
            } }
        }
    }
}

/// 竖向列分割线 / 折叠态恢复条（左右分区的调宽/折叠条）。
///
/// tolaria 式透明悬停显形：`bg-transparent` 平时几乎不可见，悬停整条亮起
/// （`hover:bg-zinc-700`，预生成 css 中 `hover:bg-border` 不存在，取等价类）。
/// 展开态可拖动调宽、单击折叠；折叠态只剩这条细条，单击恢复默认宽度。
#[component]
fn VColLine(
    /// 所属侧。
    side: ColSide,
    /// 对应列是否折叠。
    collapsed: bool,
    /// 按下回调。
    on_mousedown: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: format!("h-full w-1.5 shrink-0 cursor-col-resize {REVEAL_BAR}"),
            role: "separator",
            aria_label: if side == ColSide::Left {
                if collapsed { "展开左列" } else { "折叠或拖宽左列" }
            } else if collapsed {
                "展开右列"
            } else {
                "折叠或拖宽右列"
            },
            "data-testid": if side == ColSide::Left { "col-split-left" } else { "col-split-right" },
            title: if collapsed { "单击展开" } else { "拖动调宽；单击折叠" },
            onmousedown: move |e: MouseEvent| on_mousedown.call(e),
        }
    }
}

/// 水平分割线（上下分区的 row-resize 拖动条），同样透明悬停显形。
#[component]
fn SplitLine(
    /// 所属侧。
    side: SplitSide,
    /// 按下回调。
    on_mousedown: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: format!("h-1 w-full shrink-0 cursor-row-resize {REVEAL_BAR}"),
            role: "separator",
            aria_label: if side == SplitSide::Left {
                "左列上下分割线"
            } else {
                "右列上下分割线"
            },
            "data-testid": if side == SplitSide::Left {
                "split-left"
            } else {
                "split-right"
            },
            onmousedown: move |e: MouseEvent| on_mousedown.call(e),
        }
    }
}

impl From<SplitSide> for Side {
    fn from(s: SplitSide) -> Self {
        match s {
            SplitSide::Left => Side::Left,
            SplitSide::Right => Side::Right,
        }
    }
}
