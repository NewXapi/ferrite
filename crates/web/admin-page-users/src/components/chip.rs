//! 可点选胶囊 chip:选中反色的纯呈现组件,供分组/角色选择器与表单页签复用。
//! 语义(选了什么、集合怎么变)归调用侧;本组件只管外观与可点性。

use dioxus::prelude::*;

/// chip 尺寸:选择器内小号 / 弹窗页签中号。
#[derive(Clone, Copy, PartialEq, Default)]
pub enum ChipSize {
    /// 选择器内的小号(`px-2.5 py-0.5`)。
    #[default]
    Sm,
    /// 弹窗页签的中号(`px-3 py-1`)。
    Md,
}

/// 可点选胶囊 chip。
///
/// 【是什么】纯呈现的胶囊按钮:选中白底反色,未选灰边带 hover;点击抛 Press 事件。
///
/// 【做什么】只负责外观与可点性;选了什么、集合怎么变由调用侧在 on_press
/// 里处理。不认识业务,不发请求。
///
/// 【交互逻辑】点击 → `on_press(())`。
///
/// 【样式】`rounded-full border {size} text-xs font-medium transition-colors`;
/// 选中 `border-zinc-100 bg-zinc-100 text-zinc-900`,未选
/// `border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500`;
/// `aria-pressed` 标注选中。
///
/// 【子组件组成】无。
///
/// 【数据流】
/// - 对内(入):`label` / `selected` / `testid` / `size`(默认 Sm)。
/// - 对外(出):`on_press(())`。
#[component]
pub fn Chip(
    label: String,
    selected: bool,
    testid: String,
    #[props(default)] size: ChipSize,
    on_press: EventHandler<()>,
) -> Element {
    let size = match size {
        ChipSize::Sm => "px-2.5 py-0.5",
        ChipSize::Md => "px-3 py-1",
    };
    let tone = if selected {
        "border-zinc-100 bg-zinc-100 text-zinc-900"
    } else {
        "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
    };

    rsx! {
        button {
            class: "rounded-full border {size} {ui::TYPE_DESC} transition-colors {tone}",
            "data-testid": "{testid}",
            "aria-pressed": "{selected}",
            onclick: move |_| on_press.call(()),
            "{label}"
        }
    }
}
