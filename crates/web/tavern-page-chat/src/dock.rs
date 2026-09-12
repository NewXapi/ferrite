//! UI 侧 dock 状态封装：以 `GlobalSignal<DockLayout>` 驱动四区 dock 渲染。
//!
//! 所有布局变更都走 [`tavern_state::dock`] 的纯函数（`move_item` /
//! `reorder_in_zone` / `set_split` / `set_zone_collapsed`），本模块只负责
//! 把纯函数结果写进全局信号，以及经 `document::eval` 读写 localStorage
//! 做持久化。native 目标下 eval 静默返回失败，等价于「localStorage 不可用」
//! 时降级：读不到用默认布局，写失败忽略。

use dioxus::prelude::*;
use tavern_state::dock::{
    DockItem, DockLayout, Side, SplitRatio, Zone, deserialize, move_item, reorder_in_zone,
    serialize, set_split, set_zone_collapsed,
};

/// localStorage 键。
pub const STORAGE_KEY: &str = "tavern-dock-layout";

/// 全页共享的 dock 布局信号，任何面板/布局组件读它渲染。
pub static DOCK: GlobalSignal<DockLayout> = Signal::global(default_layout);

/// 默认布局：四个面板各自停靠目标区。
fn default_layout() -> DockLayout {
    DockLayout {
        items: vec![
            DockItem {
                id: "character".into(),
                zone: Zone::LeftTop,
                order: 0,
                enabled: true,
                title: "角色卡".into(),
            },
            DockItem {
                id: "sessions".into(),
                zone: Zone::LeftBottom,
                order: 0,
                enabled: true,
                title: "会话时间线".into(),
            },
            DockItem {
                id: "prompt".into(),
                zone: Zone::RightTop,
                order: 0,
                enabled: true,
                title: "prompt 导航".into(),
            },
            DockItem {
                id: "model".into(),
                zone: Zone::RightBottom,
                order: 0,
                enabled: true,
                title: "模型与轮次".into(),
            },
        ],
        // 左右列上下分割：角色/prompt 内容短，会话/模型区占比更高，避免顶部
        // 半区空旷、底部被撑太大。左 35/65，右 40/60。
        split_left: SplitRatio::new_unchecked(0.35),
        split_right: SplitRatio::new_unchecked(0.40),
        ..DockLayout::default()
    }
}

/// 把当前布局序列化写回 localStorage；写失败（无 localStorage / eval 不可用）
/// 静默忽略。
fn persist() {
    let json = serialize(&DOCK());
    let escaped = serde_json::to_string(&json).unwrap_or_else(|_| "null".into());
    let js = format!(
        r#"(() => {{ try {{ localStorage.setItem({key:?}, {escaped:?}); }} catch (e) {{}} }})()"#,
        key = STORAGE_KEY,
        escaped = escaped
    );
    let _ = dioxus::document::eval(&js);
}

/// 在组件内初始化 dock 状态：
///
/// - 首次渲染用默认布局，挂载后从 localStorage 读回覆盖（读不到/损坏保持默认，
///   静默降级）；
/// - 之后所有变更入口（`move_panel` / `reorder_zone` / `set_side_split` /
///   `collapse_zone`）内部统一调 [`persist`] 写回 localStorage。
pub fn use_dock_state() {
    use_hook({
        move || {
            spawn(async move {
                let js = format!(
                    r#"(() => {{ const s = localStorage.getItem({key:?}); return s ? s : null; }})()"#,
                    key = STORAGE_KEY
                );
                let Ok(value) = dioxus::document::eval(&js).await else {
                    return;
                };
                let v: serde_json::Value = value;
                let restored = deserialize(&v).unwrap_or_else(|_| default_layout());
                DOCK.with_mut(|l| *l = restored);
            });
        }
    });
}

/// 跨区移动某面板（同区调用为 no-op，区内重排走 [`reorder_zone`]）。
pub fn move_panel(item_id: &str, target: Zone) {
    DOCK.with_mut(|l| move_item(l, item_id, target));
    persist();
}

/// 区内按启用项索引换位。
pub fn reorder_zone(zone: Zone, from_index: usize, to_index: usize) {
    DOCK.with_mut(|l| reorder_in_zone(l, zone, from_index, to_index));
    persist();
}

/// 设置某侧列的上下分割比例（越界值由纯函数 clamp 到 0.05..=0.95）。
pub fn set_side_split(side: Side, ratio: f32) {
    DOCK.with_mut(|l| set_split(l, side, SplitRatio::new_unchecked(ratio)));
    persist();
}

/// 收起/展开某区。
pub fn collapse_zone(zone: Zone, collapsed: bool) {
    DOCK.with_mut(|l| set_zone_collapsed(l, zone, collapsed));
    persist();
}

/// 该区是否处于收起态（任一停靠项被禁用即视为收起）。
pub fn zone_collapsed(zone: Zone) -> bool {
    DOCK().items.iter().any(|i| i.zone == zone && !i.enabled)
}

/// 某面板当前停靠的区。
pub fn zone_of_item(item_id: &str) -> Option<Zone> {
    DOCK()
        .items
        .iter()
        .find(|i| i.id == item_id)
        .map(|i| i.zone)
}

/// 某区内启用项按 order 升序的 id 列表。
pub fn zone_item_ids(zone: Zone) -> Vec<String> {
    let l = DOCK();
    let mut ids: Vec<(u32, &String)> = l
        .items
        .iter()
        .filter(|i| i.zone == zone && i.enabled)
        .map(|i| (i.order, &i.id))
        .collect();
    ids.sort_by_key(|(o, _)| *o);
    ids.into_iter().map(|(_, id)| id.clone()).collect()
}

/// 某区被禁用的项（区收起态渲染图标轨用）。
pub fn zone_disabled_items(zone: Zone) -> Vec<DockItem> {
    DOCK()
        .items
        .iter()
        .filter(|i| i.zone == zone && !i.enabled)
        .cloned()
        .collect()
}

/// 某面板在区内启用项中的（区, 索引）；找不到返回 None。
pub fn item_origin(item_id: &str) -> Option<(Zone, usize)> {
    for zone in [
        Zone::LeftTop,
        Zone::LeftBottom,
        Zone::RightTop,
        Zone::RightBottom,
    ] {
        if let Some(idx) = zone_item_ids(zone).iter().position(|id| id == item_id) {
            return Some((zone, idx));
        }
    }
    None
}

/// 面板默认停靠区（布局里找不到项时兜底用）。
pub fn default_zone(item_id: &str) -> Zone {
    match item_id {
        "character" => Zone::LeftTop,
        "sessions" => Zone::LeftBottom,
        "model" => Zone::RightBottom,
        _ => Zone::RightTop,
    }
}
