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
        div { class: "inline-flex items-center gap-1 {crate::TYPE_DESC} px-1",
            button {
                class: "flex h-5 w-5 items-center justify-center rounded hover:bg-zinc-800 hover:text-zinc-100 disabled:opacity-30",
                disabled: index == 0,
                onclick: move |_| on_prev.call(()),
                "‹"
            }
            span { class: "tabular-nums {crate::TYPE_LABEL}", "{index + 1}/{total}" }
            button {
                class: "flex h-5 w-5 items-center justify-center rounded hover:bg-zinc-800 hover:text-zinc-100 disabled:opacity-30",
                disabled: index + 1 >= total,
                onclick: move |_| on_next.call(()),
                "›"
            }
        }
    }
}
