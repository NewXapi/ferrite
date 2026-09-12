//! 四区 dock 布局渲染与拖拽逻辑。
//!
//! 结构：左列 / 中央 editor 槽 / 右列。左右列各上下二分（比例来自
//! [`crate::dock`] 的 `SplitRatio`），列间与列内分割线均可拖拽：
//! 水平分割线改上下比例，竖向列分割线改列宽（拖过 3% 阈值或单击即折叠/展开列）。
//!
//! 面板跨区拖拽事件链：面板标题栏 `mousedown` 记录被拖项与源区 →
//! 根容器 `mousemove` 置移动标记 → 根容器 `mouseup` 按指针坐标经
//! [`hit_zone`] 判定落入哪个区：跨区调 [`crate::dock::move_panel`]，
//! 同区调 [`crate::dock::reorder_zone`]。

use dioxus::prelude::*;
use tavern_state::dock::{Side, Zone};

use crate::dock;

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

/// 把 client 坐标映射为「(所在区, 区内序号)」：
/// - x 落在左列/右列内，再按中央区高度中点上下二分；
/// - 中央区不可停靠，返回 `None`。
fn hit_zone(
    rect: &ChatRect,
    col_left: f32,
    col_right: f32,
    cx: f64,
    cy: f64,
) -> Option<(Zone, usize)> {
    let lw = rect.width * col_left as f64;
    let rw = rect.width * col_right as f64;
    let in_left = col_left > 0.0 && cx < rect.x + lw;
    let in_right = col_right > 0.0 && cx > rect.x + rect.width - rw;
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

/// 渲染四区 dock 骨架：左列 / 中央 editor 槽 / 右列。
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
    let col_left = dock_layout().col_left.value();
    let col_right = dock_layout().col_right.value();

    // prop 名只对应默认停靠区：面板可跨区拖动，render_zone 按 DOCK 信号的
    // 实际落区从这四个元素里取用，槽位名只是传参通道，不锁定面板位置。
    let character_panel = left_top;
    let sessions_panel = left_bottom;
    let prompt_panel = right_top;
    let model_panel = right_bottom;

    let mut rect = use_signal(|| None::<ChatRect>);
    let mut panel_drag = use_signal(|| None::<PanelDrag>);
    let mut split_drag = use_signal(|| None::<(SplitSide, f64)>);
    let mut col_drag = use_signal(|| None::<(ColSide, f64)>);

    // 列分割线 mousedown：记录侧与起点。单击（无移动）折叠/展开在 mouseup 处理。
    let start_col = {
        let mut col_drag = col_drag;
        move |side: ColSide| {
            move |e: MouseEvent| {
                e.stop_propagation();
                let c = e.client_coordinates();
                col_drag.set(Some((side, c.x)));
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
                    SplitSide::Left => dock::DOCK.read().clone().split_left.value() as f64,
                    SplitSide::Right => dock::DOCK.read().clone().split_right.value() as f64,
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

            // 竖向列分割线拖拽：按整宽 x 比例改列宽，压到 <3% 即折叠该列
            if let Some((_side, sx)) = *col_drag.read() {
                let Some(r) = *rect.read() else {
                    return;
                };
                // 未超过 4px 视为点按，不进入拖拽改宽
                if (c.x - sx).abs() <= 4.0 {
                    return;
                }
                let (mut l, mut rr) = (
                    dock::DOCK.read().col_left.value(),
                    dock::DOCK.read().col_right.value(),
                );
                match _side {
                    ColSide::Left => l = ((c.x - r.x) / r.width.max(1.0)) as f32,
                    ColSide::Right => rr = ((r.x + r.width - c.x) / r.width.max(1.0)) as f32,
                }
                // 拖过 3% 阈值视为折叠意图，置 0
                if l < 0.03 {
                    l = 0.0;
                }
                if rr < 0.03 {
                    rr = 0.0;
                }
                dock::set_cols(l, rr);
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

            // 列分割线单击（无移动）：切换该列折叠/展开
            if let Some((side, sx)) = *col_drag.read() {
                let c = e.client_coordinates();
                if (c.x - sx).abs() <= 4.0 {
                    let cur = dock::DOCK.read();
                    let (l, r) = (cur.col_left.value(), cur.col_right.value());
                    drop(cur);
                    match side {
                        ColSide::Left => {
                            let nl = if l == 0.0 { 0.28 } else { 0.0 };
                            dock::set_cols(nl, r);
                        }
                        ColSide::Right => {
                            let nr = if r == 0.0 { 0.28 } else { 0.0 };
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
                (cur.col_left.value(), cur.col_right.value())
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
    let col_left_c = col_left;
    let col_right_c = col_right;

    rsx! {
        div {
            class: "flex h-full w-full min-h-0 min-w-0",
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

            // 左列：上下二分（列宽内联 style，动态来自 DOCK.col_left；0=折叠隐藏）
            { if col_left_c > 0.0 {
                rsx! {
                    div {
                        class: "flex h-full shrink-0 flex-col gap-1 py-1 pr-1",
                        style: "width: {col_left_c as f64 * 100.0}%",
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

            // 左竖向列分割线：拖动调宽，单击折叠/展开左列
            VColLine {
                side: ColSide::Left,
                collapsed: col_left_c == 0.0,
                on_mousedown: move |e: MouseEvent| start_col(ColSide::Left)(e),
            }

            // 中央 editor 槽
            div { class: "flex h-full min-h-0 min-w-0 flex-1 flex-col", { editor } }

            // 右竖向列分割线：拖动调宽，单击折叠/展开右列
            VColLine {
                side: ColSide::Right,
                collapsed: col_right_c == 0.0,
                on_mousedown: move |e: MouseEvent| start_col(ColSide::Right)(e),
            }

            // 右列：上下二分（列宽内联 style，动态来自 DOCK.col_right；0=折叠隐藏）
            { if col_right_c > 0.0 {
                rsx! {
                    div {
                        class: "flex h-full shrink-0 flex-col gap-1 py-1 pl-1",
                        style: "width: {col_right_c as f64 * 100.0}%",
                        role: "group",
                        aria_label: "右列面板",

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
                    }
                }
            } else { rsx! { div {} } } }
        }
    }
}

/// 竖向列分割线（左右分区的调宽/折叠条）：6px 宽、cursor-col-resize；
/// 折叠态显示醒目色提示可点击展开。
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
            class: if collapsed {
                // 折叠态：平时只有一条淡紫细线（不抢视觉），悬停整条亮起并可点击展开
                "group h-full w-2 shrink-0 cursor-col-resize bg-purple-500/25 hover:bg-purple-400 transition-colors"
            } else {
                "h-full w-1.5 shrink-0 cursor-col-resize rounded bg-zinc-800/70 hover:bg-purple-500/40 transition-colors"
            },
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

/// 4px 水平分割线（上下分区的 col-resize 拖动条）。
#[component]
fn SplitLine(
    /// 所属侧。
    side: SplitSide,
    /// 按下回调。
    on_mousedown: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: "my-0.5 h-1 w-full shrink-0 cursor-row-resize rounded bg-zinc-800/70 hover:bg-purple-500/40 transition-colors",
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
