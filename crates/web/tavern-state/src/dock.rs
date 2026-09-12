//! dock 布局纯函数层（Flint 风格四区 dock：left-top / left-bottom / right-top / right-bottom）。
//!
//! 职责：只定义数据结构与纯函数，不做渲染、不做拖拽——UI 层（后续任务）
//! 依据这里的状态与函数驱动界面：
//! - [`Zone`]：四个 dock 区。
//! - [`DockItem`]：一个 dock 面板项（id / 区 / 区内序 / 启用 / 标题）。
//! - [`DockLayout`]：整个 dock 布局状态（项集合、各区激活项、左右列上下分割比例）。
//! - [`move_item`] / [`reorder_in_zone`] / [`dock_tab`] / [`set_split`] /
//!   [`set_zone_collapsed`] / [`serialize`] / [`deserialize`]：布局变更的纯函数，
//!   无全局状态，便于单测。

use std::collections::HashMap;

/// 四个 dock 区。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    /// 左上区。
    LeftTop,
    /// 左下区。
    LeftBottom,
    /// 右上区。
    RightTop,
    /// 右下区。
    RightBottom,
}

impl Zone {
    /// 四个区的稳定遍历序（渲染顺序用）。
    pub const ALL: [Zone; 4] = [
        Self::LeftTop,
        Self::LeftBottom,
        Self::RightTop,
        Self::RightBottom,
    ];

    /// 对侧同列区（LeftTop ↔ RightTop，LeftBottom ↔ RightBottom）。
    pub fn opposite(self) -> Zone {
        match self {
            Self::LeftTop => Self::RightTop,
            Self::LeftBottom => Self::RightBottom,
            Self::RightTop => Self::LeftTop,
            Self::RightBottom => Self::LeftBottom,
        }
    }
}

/// 一个 dock 面板项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockItem {
    /// 面板稳定标识（UI 层据此渲染内容，如 "sessions" / "model"）。
    pub id: String,
    /// 所在区。
    pub zone: Zone,
    /// 区内排序（越小越靠上）。
    pub order: u32,
    /// 是否启用（禁用时区槽位保留但隐藏内容）。
    pub enabled: bool,
    /// 显示标题。
    pub title: String,
}

/// 区上下分割比例：left/right 列各一份，取值 0.05..=0.95。
///
/// 内部保留原始 f32；经 [`set_split`] 写入时统一 clamp 到合法范围。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitRatio(f32);

impl SplitRatio {
    /// 默认比例 0.5。
    pub const DEFAULT: SplitRatio = SplitRatio(0.5);

    /// 以原始值构造比例（不做 clamp，clamp 由 [`set_split`] 负责）。
    pub const fn new_unchecked(value: f32) -> Self {
        Self(value)
    }

    /// 读取比例原始值。
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl Default for SplitRatio {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// 整个 dock 布局状态。
#[derive(Debug, Clone, PartialEq)]
pub struct DockLayout {
    /// 全部面板项。
    pub items: Vec<DockItem>,
    /// 各区当前激活项 id（`None` = 无激活）。全四区键恒存在（total map）：
    /// 读取无需处理缺键，跨区移动/折叠的写入语义也不依赖键是否已存在。
    pub active_by_zone: HashMap<Zone, Option<String>>,
    /// 左列上下分割比例。
    pub split_left: SplitRatio,
    /// 右列上下分割比例。
    pub split_right: SplitRatio,
    /// 左列宽度占总宽比例（0 = 该列折叠；上限 0.45）。
    pub col_left: SplitRatio,
    /// 右列宽度占总宽比例（0 = 该列折叠；上限 0.45）。
    pub col_right: SplitRatio,
}

impl Default for DockLayout {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            active_by_zone: Zone::ALL.map(|z| (z, None::<String>)).into_iter().collect(),
            split_left: SplitRatio::DEFAULT,
            split_right: SplitRatio::DEFAULT,
            col_left: SplitRatio::new_unchecked(0.28),
            col_right: SplitRatio::new_unchecked(0.28),
        }
    }
}

/// 左右列（分割比例所属侧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// 左列。
    Left,
    /// 右列。
    Right,
}

/// 把某项 order 追加到目标区末尾（目标区当前最大 order+1；目标区为空则 0）。
///
/// 仅修改该 item 的 order 与 zone。
fn append_to_zone_tail(layout: &mut DockLayout, item_id: &str, target: Zone) {
    let next_order = layout
        .items
        .iter()
        .filter(|i| i.zone == target)
        .map(|i| i.order)
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);
    if let Some(item) = layout.items.iter_mut().find(|i| i.id == item_id) {
        item.zone = target;
        item.order = next_order;
    }
}

/// 跨区/区内移动面板项。
///
/// - 跨区：改 zone，`enabled = true`，order 追加到目标区末尾（目标区当前最大
///   order + 1）；若源区当前激活的是该项，激活切到源区剩余启用项中 order 最大
///   的那个，没有则置 `None`；目标区激活不变。
/// - 同区：无操作（区内重排由 [`reorder_in_zone`] 负责）。
/// - 找不到 item：无操作，不 panic。
pub fn move_item(layout: &mut DockLayout, item_id: &str, target: Zone) {
    let Some(idx) = layout.items.iter().position(|i| i.id == item_id) else {
        return;
    };
    let from = layout.items[idx].zone;
    if from == target {
        return;
    }

    // 源区激活切换：先看源区是否激活了该项。
    let active_was_moved = layout.active_by_zone.get(&from) == Some(&Some(item_id.to_string()));

    layout.items[idx].enabled = true;
    append_to_zone_tail(layout, item_id, target);

    if active_was_moved {
        // 剩余启用项（已不含被移走的项）中 order 最大者成为新激活，没有则 None。
        let successor = layout
            .items
            .iter()
            .filter(|i| i.zone == from && i.enabled && i.id != item_id)
            .max_by(|a, b| a.order.cmp(&b.order))
            .map(|i| i.id.clone());
        layout.active_by_zone.insert(from, successor);
    }
}

/// 区内启用项（按 order 升序）稳定换位，并重写该区所有项 order 为 0..=n-1。
///
/// - `from_index` 越界：无操作。
/// - `to_index` 越界：clamp 到 `[0, n]`（n = 区内启用项数）。
/// - 重写只对区内启用项生效（禁用项保持原位不动）。
pub fn reorder_in_zone(layout: &mut DockLayout, zone: Zone, from_index: usize, to_index: usize) {
    // 区内启用项按 order 升序的 id 序列。
    let mut ids: Vec<(u32, String)> = layout
        .items
        .iter()
        .filter(|i| i.zone == zone && i.enabled)
        .map(|i| (i.order, i.id.clone()))
        .collect();
    ids.sort_by_key(|(o, _)| *o);

    let n = ids.len();
    if n == 0 || from_index >= n {
        return;
    }
    let to = to_index.min(n - 1);

    let (removed_order, removed_id) = ids.remove(from_index);
    ids.insert(to, (removed_order, removed_id));

    // 重写该区启用项 order 为 0..=n-1（禁用项不动）。
    for (new_order, (_, id)) in ids.iter().enumerate() {
        if let Some(item) = layout
            .items
            .iter_mut()
            .find(|i| i.id == id.as_str() && i.zone == zone)
        {
            item.order = new_order as u32;
        }
    }
}

/// 普通 tab 拖进某区即停靠的入口。
///
/// - 同 id 已存在：把已有项 zone 更新为 `item.zone`（等价跨区移动），保留已有
///   order；若新 zone 下 order 与目标区现有项冲突，则顺延到目标区末尾。
/// - 不存在：按 item 的 zone/order 直接插入。
pub fn dock_tab(layout: &mut DockLayout, item: DockItem) {
    let idx = match layout.items.iter().position(|i| i.id == item.id) {
        Some(idx) => idx,
        None => {
            layout.items.push(item);
            return;
        }
    };
    let existing = layout.items[idx].clone();
    layout.items[idx].zone = item.zone;
    // order 冲突（目标区已有其他项占同一 order）时顺延到末尾。
    let conflicts = layout
        .items
        .iter()
        .any(|i| i.id != item.id && i.zone == item.zone && i.order == existing.order);
    if conflicts {
        let next_order = layout
            .items
            .iter()
            .filter(|i| i.zone == item.zone && i.id != item.id)
            .map(|i| i.order)
            .max()
            .map(|m| m + 1)
            .unwrap_or(0);
        layout.items[idx].order = next_order;
    }
}

/// 设置某侧列的上下分割比例。
///
/// 越界比例 clamp 到 0.05..=0.95（NaN 归为 0.5）。
pub fn set_split(layout: &mut DockLayout, side: Side, ratio: SplitRatio) {
    let value = if ratio.value().is_finite() {
        ratio.value().clamp(0.05, 0.95)
    } else {
        0.5
    };
    let split = SplitRatio::new_unchecked(value);
    match side {
        Side::Left => layout.split_left = split,
        Side::Right => layout.split_right = split,
    }
}

/// 设置左右列宽度占总宽的比例（列间水平分割线用）。
///
/// - 取值 clamp 到 0.0..=0.45：0 表示该列折叠；NaN 归为 0.28。
/// - 左右之和超过 0.9 时按比例压缩，保证中央区至少 10%。
pub fn set_col_widths(layout: &mut DockLayout, left: SplitRatio, right: SplitRatio) {
    let l = if left.value().is_finite() {
        left.value().clamp(0.0, 0.45)
    } else {
        0.28
    };
    let r = if right.value().is_finite() {
        right.value().clamp(0.0, 0.45)
    } else {
        0.28
    };
    let (l, r) = if l + r > 0.9 {
        let k = 0.9 / (l + r);
        (l * k, r * k)
    } else {
        (l, r)
    };
    layout.col_left = SplitRatio::new_unchecked(l);
    layout.col_right = SplitRatio::new_unchecked(r);
}

/// 设置某区收起/展开。
///
/// - `collapsed = true`：该区所有项 `enabled = false`，激活项置 `None`。
/// - `collapsed = false`：该区所有项 `enabled = true`（激活项不变）。
pub fn set_zone_collapsed(layout: &mut DockLayout, zone: Zone, collapsed: bool) {
    for item in layout.items.iter_mut() {
        if item.zone == zone {
            item.enabled = !collapsed;
        }
    }
    if collapsed {
        layout.active_by_zone.insert(zone, None);
    }
}

/// 把布局序列化为 JSON（持久化用，与 [`deserialize`] 对称）。
pub fn serialize(layout: &DockLayout) -> serde_json::Value {
    let items: Vec<serde_json::Value> = layout
        .items
        .iter()
        .map(|i| {
            serde_json::json!({
                "id": i.id,
                "zone": zone_name(i.zone),
                "order": i.order,
                "enabled": i.enabled,
                "title": i.title,
            })
        })
        .collect();
    let active: HashMap<&str, Option<String>> = layout
        .active_by_zone
        .iter()
        .map(|(z, v)| (zone_name(*z), v.clone()))
        .collect();
    serde_json::json!({
        "items": items,
        "active_by_zone": active,
        "split_left": layout.split_left.value(),
        "split_right": layout.split_right.value(),
        "col_left": layout.col_left.value(),
        "col_right": layout.col_right.value(),
    })
}

/// 从 JSON 反序列化布局（与 [`serialize`] 对称）。
///
/// 未知字段容忍；缺少必需字段或损坏输入（items 非数组、zone 非法等）返回
/// `Err`，不 panic。
pub fn deserialize(v: &serde_json::Value) -> Result<DockLayout, String> {
    let mut layout = DockLayout::default();

    let items = v
        .get("items")
        .ok_or_else(|| "missing field `items`".to_string())?
        .as_array()
        .ok_or_else(|| "field `items` is not an array".to_string())?;
    for item in items {
        let id = item
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or_else(|| "item missing `id`".to_string())?
            .to_string();
        let zone = item
            .get("zone")
            .and_then(|x| x.as_str())
            .ok_or_else(|| "item missing `zone`".to_string())
            .and_then(|name| {
                zone_from_name(name).ok_or_else(|| format!("unknown zone `{name}`"))
            })?;
        let order = item.get("order").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        let enabled = item
            .get("enabled")
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        let title = item
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        layout.items.push(DockItem {
            id,
            zone,
            order,
            enabled,
            title,
        });
    }

    if let Some(active) = v.get("active_by_zone") {
        for (name, val) in active
            .as_object()
            .ok_or_else(|| "field `active_by_zone` is not an object".to_string())?
        {
            let zone =
                zone_from_name(name.as_str()).ok_or_else(|| format!("unknown zone `{name}`"))?;
            let id = if val.is_null() {
                None
            } else {
                Some(
                    val.as_str()
                        .map(String::from)
                        .ok_or_else(|| "active id must be null or a string".to_string())?,
                )
            };
            layout.active_by_zone.insert(zone, id);
        }
    }

    layout.split_left = parse_split(v.get("split_left"))?;
    layout.split_right = parse_split(v.get("split_right"))?;
    // 列宽容忍旧数据缺字段（缺省 0.28），越界 clamp 由 set_col_widths 语义负责
    let cl = v.get("col_left").and_then(|x| x.as_f64()).unwrap_or(0.28) as f32;
    let cr = v.get("col_right").and_then(|x| x.as_f64()).unwrap_or(0.28) as f32;
    set_col_widths(
        &mut layout,
        SplitRatio::new_unchecked(cl),
        SplitRatio::new_unchecked(cr),
    );
    Ok(layout)
}

/// 把 JSON 值解析为 SplitRatio（缺失时默认 0.5；非数字、非有限值报 Err，越界 clamp）。
fn parse_split(v: Option<&serde_json::Value>) -> Result<SplitRatio, String> {
    match v {
        None | Some(serde_json::Value::Null) => Ok(SplitRatio::DEFAULT),
        Some(x) => {
            let value = x
                .as_f64()
                .ok_or_else(|| "field `split_*` is not a number".to_string())?;
            let value = value as f32;
            if !value.is_finite() {
                return Err("field `split_*` is not finite".to_string());
            }
            Ok(SplitRatio::new_unchecked(value.clamp(0.05, 0.95)))
        }
    }
}

fn zone_name(zone: Zone) -> &'static str {
    match zone {
        Zone::LeftTop => "left-top",
        Zone::LeftBottom => "left-bottom",
        Zone::RightTop => "right-top",
        Zone::RightBottom => "right-bottom",
    }
}

fn zone_from_name(name: &str) -> Option<Zone> {
    match name {
        "left-top" => Some(Zone::LeftTop),
        "left-bottom" => Some(Zone::LeftBottom),
        "right-top" => Some(Zone::RightTop),
        "right-bottom" => Some(Zone::RightBottom),
        _ => None,
    }
}
