//! 滚轮切换 tab 的通用交互原语。
//!
//! 复用点：左侧 `SectionRail`、顶部 `TopNavBar`、时间窗 `TimeframeTabs`、
//! 分段胶囊 `SegmentedCapsule`、模型卡 `StatTabsCard` —— 一切「一组互斥 tab」
//! 都挂同一个 [`on_tab_wheel`]，行为口径统一：竖向滚轮 → 循环切换
//! （末项 → 首项），并阻止默认滚动，避免连带翻页面。
//!
//! 横/竖向 tab 都用它：方向由 tab 集的排布决定，与滚轮方向无关（滚轮只有
//! 竖向 `deltaY`，向下 = 下一项，向上 = 上一项）。

use dioxus::prelude::*;

/// 循环下标：`dir` 为 +1（下一项）/ -1（上一项），越界时绕到另一端。
///
/// `len == 0` 返回 0（空 tab 集的防御，调用方不应传入空集）。
#[must_use]
pub fn cycle_index(len: usize, current: usize, dir: i32) -> usize {
    if len == 0 {
        return 0;
    }
    (current as i64 + dir as i64).rem_euclid(len as i64) as usize
}

/// 从滚轮事件取竖向滚动量；三种 delta 模式（像素 / 行 / 页）统一成同一标量，
/// 只比较符号，不比较大小。
fn delta_y(e: &WheelEvent) -> f64 {
    use dioxus::html::geometry::WheelDelta;
    match e.delta() {
        WheelDelta::Pixels(v) => v.y,
        WheelDelta::Lines(v) => v.y,
        WheelDelta::Pages(v) => v.y,
    }
}

/// tab 容器的 `onwheel` 处理器：竖向滚轮 → 循环切换 tab。
///
/// - `len`：tab 总数。
/// - `current`：当前激活下标。
/// - `on_select`：接收切换后的新下标（点选与滚轮共用同一出口）。
///
/// 阻止默认行为，否则页面会跟着滚。纯横向滚轮（`deltaY == 0`，触控板横滑）
/// 不触发切换。
// ponytail: 不做时间节流——滚轮一格一个事件即可用；若触控板惯性导致一次
// 手势跨多项，再在调用方加时间窗（需要 use_hook 存上次时间戳）。
pub fn on_tab_wheel(e: WheelEvent, len: usize, current: usize, on_select: impl FnOnce(usize)) {
    if len == 0 {
        return;
    }
    let dy = delta_y(&e);
    if dy == 0.0 {
        return;
    }
    e.prevent_default();
    on_select.call(cycle_index(len, current, if dy > 0.0 { 1 } else { -1 }));
}
