//! 四区 dock 布局渲染与拖拽逻辑。
//!
//! 结构：左列 / 中央 editor 槽 / 右列。左右列各上下二分（比例来自
//! [`crate::dock`] 的 `SplitRatio`），中间 4px 分割线拖拽改比例，
//! 把同侧某区压到 <120px 阈值时自动 `set_zone_collapsed(true)`。
//!
//! 面板跨区拖拽事件链：面板标题栏 `mousedown` 记录被拖项与源区 →
//! 根容器 `mousemove` 置移动标记 → 根容器 `mouseup` 按指针坐标经
//! [`hit_zone`] 判定落入哪个区：跨区调 [`crate::dock::move_panel`]，
//! 同区调 [`crate::dock::reorder_zone`]。

use dioxus::prelude::*;
use tavern_state::dock::{Side, Zone};

use crate::dock;

/// 把同侧某区压到 <120px 时自动收起该区的阈值。
const COLLAPSE_THRESHOLD_PX: f64 = 120.0;

/// 左右列各占中央区总宽的比例（与 rsx 里的 `w-[28%]` 保持一致，
/// 命中测试需要同值换算）。
const COLUMN_WIDTH_RATIO: f64 = 0.28;

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

/// 把 client 坐标映射为「(所在区, 区内序号)」：
/// - x 落在左列/右列内，再按中央区高度中点上下二分；
/// - 中央区不可停靠，返回 `None`。
fn hit_zone(rect: &ChatRect, cx: f64, cy: f64) -> Option<(Zone, usize)> {
    let col_w = rect.width * COLUMN_WIDTH_RATIO;
    let in_left = cx < rect.x + col_w;
    let in_right = cx > rect.x + rect.width - col_w;
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
            class: "flex min-h-0 flex-col gap-1",
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

    // prop 名只对应默认停靠区：面板可跨区拖动，render_zone 按 DOCK 信号的
    // 实际落区从这四个元素里取用，槽位名只是传参通道，不锁定面板位置。
    let character_panel = left_top;
    let sessions_panel = left_bottom;
    let prompt_panel = right_top;
    let model_panel = right_bottom;

    let mut rect = use_signal(|| None::<ChatRect>);
    let mut panel_drag = use_signal(|| None::<PanelDrag>);
    let mut split_drag = use_signal(|| None::<(SplitSide, f64)>);

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
        // 仅 panel_drag 在闭包里经 with_mut 写入需要 mut 重绑定；
        // split_drag/rect 只读，直接捕获外层绑定即可。
        let mut panel_drag = panel_drag;
        move |e: MouseEvent| {
            let c = e.client_coordinates();

            // 分割线拖拽中：按列内 y 比例改 split，被压区 <120px 自动收起
            if let Some((side, _start)) = *split_drag.read() {
                let Some(r) = *rect.read() else {
                    return;
                };
                let col_h = r.height;
                let ratio = ((c.y - r.y) / col_h.max(1.0)).clamp(0.05, 0.95);
                dock::set_side_split(side.into(), ratio as f32);

                let top_h = ratio * col_h;
                let bottom_h = col_h - top_h;
                let (top_zone, bottom_zone) = match side {
                    SplitSide::Left => (Zone::LeftTop, Zone::LeftBottom),
                    SplitSide::Right => (Zone::RightTop, Zone::RightBottom),
                };
                dock::collapse_zone(top_zone, top_h < COLLAPSE_THRESHOLD_PX);
                dock::collapse_zone(bottom_zone, bottom_h < COLLAPSE_THRESHOLD_PX);
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
        // 仅 split_drag 在闭包里 set 需要 mut 重绑定；
        // panel_drag/rect 只读，直接捕获外层绑定即可。
        let mut split_drag = split_drag;
        move |e: MouseEvent| {
            split_drag.set(None);

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
            if let Some((target, target_index)) = hit_zone(&r, c.x, c.y) {
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
            },

            // 左列：上下二分
            div {
                class: "flex h-full shrink-0 flex-col gap-1 py-1 pr-1 w-[28%]",
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

            // 中央 editor 槽
            div { class: "flex h-full min-h-0 min-w-0 flex-1 flex-col", { editor } }

            // 右列：上下二分
            div {
                class: "flex h-full shrink-0 flex-col gap-1 py-1 pl-1 w-[28%]",
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
