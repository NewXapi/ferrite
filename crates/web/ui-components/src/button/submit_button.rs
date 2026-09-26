use dioxus::prelude::*;

/// 表单提交按钮
#[component]
pub fn SubmitButton(
    label: String,
    #[props(default = "submit".to_string())] button_type: String,
    #[props(default)] onclick: EventHandler<()>,
    /// 提交进行中：禁用按钮并显示加载态，防止重复提交。
    #[props(default = false)]
    busy: bool,
) -> Element {
    rsx! {
        button {
            class: "flex w-full items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-zinc-900 transition-all duration-200 hover:bg-zinc-200 hover:shadow-lg hover:shadow-zinc-100/10 active:scale-[0.98] active:bg-zinc-300 disabled:cursor-not-allowed disabled:opacity-50",
            r#type: "{button_type}",
            disabled: busy,
            onclick: move |_| onclick.call(()),
            if busy {
                span {
                    class: "size-4 shrink-0 animate-spin rounded-full border-2 border-zinc-900/30 border-t-zinc-900"
                }
            }
            "{label}"
        }
    }
}
