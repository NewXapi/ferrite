use dioxus::prelude::*;

/// 渲染用于切换卡片内容的圆点按钮组。
///
/// `tabs` 为每颗圆点提供可访问名称，`active` 是当前按下的圆点索引，
/// `on_change` 在用户以 Tab 聚焦后按 Enter 或 Space，或直接点击圆点时接收
/// 所选索引。圆点是普通按钮，不采用不完整的 ARIA tab 语义。
///
/// 在页签容器上滚动鼠标滚轮也会在页签间循环切换：向上滚切换前一个
/// 页签（首个时回到末尾），向下滚切换后一个页签（末尾时回到首个），
/// 并阻止事件默认行为以免驱动页面滚动；`tabs` 为空时不做任何事。
///
/// 当 `active` 超出范围时不会产生错误：不标记任何按钮为按下。
///
/// 例如，可将此组件放入 `AdminCard` 标题栏以切换实体摘要内容。
#[component]
pub fn DotTabBar(
    tabs: Vec<&'static str>,
    active: usize,
    on_change: EventHandler<usize>,
) -> Element {
    let n = tabs.len();
    rsx! {
        div {
            class: "flex items-center gap-1.5",
            role: "group",
            "aria-label": "内容页签",
            // 滚轮循环切页签：deltaY < 0 前一个，deltaY > 0 后一个，越界取模回绕。
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
                if dy == 0.0 {
                    return;
                }
                let dir = if dy > 0.0 { 1i64 } else { -1i64 };
                let next = ((active as i64) + dir).rem_euclid(n as i64) as usize;
                on_change.call(next);
            },
            for (i, label) in tabs.iter().enumerate() {
                {
                    let idx = i;
                    let is_active = i == active;
                    rsx! {
                        button {
                            key: "dot-{i}",
                            class: if is_active {
                                "h-2 w-2 rounded-full bg-white transition-colors"
                            } else {
                                "h-2 w-2 rounded-full border border-white/40 transition-colors hover:border-white/70"
                            },
                            "aria-label": "{label}",
                            "aria-pressed": "{is_active}",
                            "data-testid": "dot-tab-{i}",
                            type: "button",
                            onclick: move |_| on_change.call(idx),
                        }
                    }
                }
            }
        }
    }
}
