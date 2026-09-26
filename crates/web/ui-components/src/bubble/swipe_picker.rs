use dioxus::prelude::*;

/// 分支切换器 (‹ 1/2 ›)
#[component]
pub fn SwipePicker(
    index: usize,
    total: usize,
    on_prev: EventHandler<()>,
    on_next: EventHandler<()>,
) -> Element {
    if total <= 1 {
        return rsx! {};
    }
    rsx! {
        div { class: "inline-flex items-center gap-1 {crate::T_text_xs} {crate::T_text_zinc_400} {crate::T_font_medium} px-1",
            button {
                class: "flex h-5 w-5 items-center justify-center rounded hover:{crate::T_bg_zinc_800} hover:{crate::T_text_zinc_100} disabled:opacity-30",
                disabled: index == 0,
                onclick: move |_| on_prev.call(()),
                "‹"
            }
            span { class: "tabular-nums {crate::T_text_11px}", "{index + 1}/{total}" }
            button {
                class: "flex h-5 w-5 items-center justify-center rounded hover:{crate::T_bg_zinc_800} hover:{crate::T_text_zinc_100} disabled:opacity-30",
                disabled: index + 1 >= total,
                onclick: move |_| on_next.call(()),
                "›"
            }
        }
    }
}
