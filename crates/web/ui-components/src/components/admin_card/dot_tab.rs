use dioxus::prelude::*;

/// 渲染用于切换卡片内容的圆点按钮组。
///
/// `tabs` 为每颗圆点提供可访问名称，`active` 是当前按下的圆点索引，
/// `on_change` 在用户以 Tab 聚焦后按 Enter 或 Space，或直接点击圆点时接收
/// 所选索引。圆点是普通按钮，不采用不完整的 ARIA tab 语义。
///
/// 当 `tabs` 为空或 `active` 超出范围时不会产生错误：前者不渲染按钮，后者
/// 不标记任何按钮为按下。
///
/// 例如，可将此组件放入 `AdminCard` 标题栏以切换实体摘要内容。
#[component]
pub fn DotTabBar(
    tabs: Vec<&'static str>,
    active: usize,
    on_change: EventHandler<usize>,
) -> Element {
    rsx! {
        div {
            class: "flex items-center gap-1.5",
            role: "group",
            "aria-label": "内容页签",
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
