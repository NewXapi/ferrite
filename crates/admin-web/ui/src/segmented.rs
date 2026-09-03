//! 胶囊分段选择器(SegmentedCapsule)
//!
//! 外观:一个大胶囊容器拆成若干分段,两端半圆、中间是矩形段。
//! 布局约定:
//! - 手机端每行最多 3 段,超出自动换行(不横向滚动)
//! - 平板/桌面按内容自然排列
//!
//! 交互:悬停在容器上滚鼠标滚轮,可循环切换到相邻分段。

use dioxus::prelude::*;

#[component]
pub fn SegmentedCapsule(
    /// 分段文本,顺序即切换顺序
    items: Vec<String>,
    /// 当前选中下标
    active: usize,
    /// 选择回调
    on_select: EventHandler<usize>,
) -> Element {
    let n = items.len();
    rsx! {
        div {
            class: "flex w-full flex-wrap overflow-hidden rounded-full border border-zinc-700 bg-zinc-950 text-xs sm:w-fit",
            onwheel: move |e: WheelEvent| {
                if n == 0 {
                    return;
                }
                use dioxus::html::geometry::WheelDelta;
                e.prevent_default();
                let dy = match e.delta() {
                    WheelDelta::Pixels(v) => v.y,
                    WheelDelta::Lines(v) => v.y,
                    WheelDelta::Pages(v) => v.y,
                };
                let dir = if dy > 0.0 { 1i64 } else { -1i64 };
                let next = (active as i64 + dir).rem_euclid(n as i64) as usize;
                on_select.call(next);
            },
            for (i, item) in items.iter().enumerate() {
                button {
                    key: "{item}",
                    class: if i == active {
                        "min-w-[30%] flex-1 truncate border-r border-zinc-800 bg-zinc-100 px-3 py-1.5 text-center text-xs font-medium text-zinc-900 last:border-r-0 sm:min-w-0 sm:flex-none"
                    } else {
                        "min-w-[30%] flex-1 truncate border-r border-zinc-800 px-3 py-1.5 text-center text-xs text-zinc-400 transition-colors last:border-r-0 hover:bg-zinc-800 hover:text-zinc-200 sm:min-w-0 sm:flex-none"
                    },
                    onclick: move |_| on_select.call(i),
                    "{item}"
                }
            }
        }
    }
}
