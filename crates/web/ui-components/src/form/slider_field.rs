use dioxus::prelude::*;

/// 滑杆字段 (对齐采样抽屉)
#[component]
pub fn SliderField(
    label: &'static str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    on_change: EventHandler<f64>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-3",
            span { class: "w-28 shrink-0 text-xs font-medium text-zinc-400", "{label}" }
            input {
                r#type: "range",
                class: "h-1 flex-1 accent-zinc-100",
                min: "{min}",
                max: "{max}",
                step: "{step}",
                value: "{value}",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse() {
                        on_change.call(v);
                    }
                },
            }
            span { class: "w-12 shrink-0 text-right text-xs tabular-nums text-zinc-300", "{value:.2}" }
        }
    }
}
