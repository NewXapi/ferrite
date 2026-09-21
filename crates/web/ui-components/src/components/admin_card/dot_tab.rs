use dioxus::prelude::*;

/// 渲染用于切换卡片内容的底部横线页签条。
///
/// 维护者批注（2026-09-21）：卡牌 tab 移到底部、占满底部，「一条横线来占位」，
/// 选中用明暗反映（白 / 灰），高度压低（2px，约 1/4 字高）。因此圆点改为
/// **等分分段底栏**：每个 tab 一个 `flex-1` 区段，区段底部 2px 横条——
/// 激活 = `bg-zinc-100`（白），非激活 = `bg-zinc-800`（灰）；整条读起来就是
/// 一条通栏横线，激活区段提亮。aria / testid / 滚轮循环切换契约与圆点版相同
/// （`dot-tab-{i}` testid 保留，既有断言不受影响）。
#[component]
pub fn DotTabBar(
    tabs: Vec<&'static str>,
    active: usize,
    on_change: EventHandler<usize>,
) -> Element {
    let n = tabs.len();
    rsx! {
        div {
            class: "mt-3 flex w-full items-stretch gap-1",
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
                    // 横条 class 必须完整字面量出现在源码里，Tailwind 才会生成对应 CSS。
                    let bar = if is_active {
                        "absolute inset-x-0 bottom-0 h-0.5 rounded-full bg-zinc-100 transition-colors"
                    } else {
                        "absolute inset-x-0 bottom-0 h-0.5 rounded-full bg-zinc-800 transition-colors"
                    };
                    rsx! {
                        button {
                            key: "tab-{i}",
                            class: "relative flex-1 pb-2 pt-1.5",
                            "aria-label": "{label}",
                            "aria-pressed": "{is_active}",
                            "data-testid": "dot-tab-{i}",
                            type: "button",
                            onclick: move |_| on_change.call(idx),
                            span { class: "{bar}", "aria-hidden": "true" }
                        }
                    }
                }
            }
        }
    }
}
