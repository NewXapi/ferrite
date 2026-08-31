//! 计费 tab:分组倍率、模型定价和实时计算器。
use dioxus::prelude::*;
use crate::api::ModelPriceRow;
use crate::state::EntityStore;

#[component]
pub fn BillingPanel() -> Element {
    let store = use_context::<EntityStore>();
    let group_ratios = store.group_ratios;
    let model_prices = store.model_prices;
    let mut selected_group = use_signal(|| "default".to_string());
    let mut selected_model = use_signal(|| "gpt-4o".to_string());
    let mut input = use_signal(|| 1000.0f64);
    let mut output = use_signal(|| 500.0f64);
    let groups = group_ratios.read().clone();
    let prices = model_prices.read().clone();
    let ratio = groups.iter().find(|r| r.group == selected_group()).map(|r| r.ratio).unwrap_or(1.0);
    let price = prices.iter().find(|p| p.alias == selected_model()).cloned().unwrap_or(ModelPriceRow { alias: selected_model(), input_per_m: 0.0, output_per_m: 0.0 });
    let total = ratio * (input() * price.input_per_m + output() * price.output_per_m) / 1_000_000.0;
    rsx! {
        div { class: "space-y-8",
            section { h2 { class: "mb-3 text-lg font-semibold", "分组倍率" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5", for row in groups.iter().cloned() { RatioCard { key: "{row.group}", row, group_ratios } } }
            }
            section { h2 { class: "mb-3 text-lg font-semibold", "模型定价（美元 / 1M tokens）" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5", for row in prices.iter().cloned() { PriceCard { key: "{row.alias}", row, model_prices } } }
            }
            section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4",
                h2 { class: "mb-4 text-lg font-semibold", "价格计算器" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                    label { class: "text-xs text-zinc-500", "用户分组", select { class: "mt-1 w-full rounded-md border border-zinc-700 bg-zinc-950 px-2 py-2 text-xs text-zinc-100", value: "{selected_group}", onchange: move |e| selected_group.set(e.value()), for row in groups.iter() { option { value: "{row.group}", "{row.group}" } } } }
                    label { class: "text-xs text-zinc-500", "模型", select { class: "mt-1 w-full rounded-md border border-zinc-700 bg-zinc-950 px-2 py-2 text-xs text-zinc-100", value: "{selected_model}", onchange: move |e| selected_model.set(e.value()), for row in prices.iter() { option { value: "{row.alias}", "{row.alias}" } } } }
                    label { class: "text-xs text-zinc-500", "输入 token", input { r#type: "number", class: "mt-1 w-full rounded-md border border-zinc-700 bg-zinc-950 px-2 py-2 text-xs", value: "{input}", oninput: move |e| if let Ok(v) = e.value().parse() { input.set(v) } } }
                    label { class: "text-xs text-zinc-500", "输出 token", input { r#type: "number", class: "mt-1 w-full rounded-md border border-zinc-700 bg-zinc-950 px-2 py-2 text-xs", value: "{output}", oninput: move |e| if let Ok(v) = e.value().parse() { output.set(v) } } }
                    div { class: "rounded-md bg-zinc-950 p-3 text-xs", div { class: "text-zinc-500", "倍率 {ratio:.2} × 单价" } div { class: "mt-1 text-lg font-semibold text-emerald-400", "${total:.6}" } }
                }
            }
        }
    }
}
#[component]
fn RatioCard(row: crate::api::GroupRatioRow, group_ratios: Signal<Vec<crate::api::GroupRatioRow>>) -> Element {
    let mut group_ratios = group_ratios; let group = row.group.clone();
    rsx! { div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4", div { class: "text-sm font-medium", "{row.group}" }, input { r#type: "number", step: "0.01", class: "mt-3 w-full rounded-md border border-zinc-700 bg-zinc-950 px-2 py-2 text-xs", value: "{row.ratio}", onchange: move |e| if let Ok(v) = e.value().parse() { if let Some(item) = group_ratios.write().iter_mut().find(|item| item.group == group) { item.ratio = v; } } } } }
}
#[component]
fn PriceCard(row: ModelPriceRow, model_prices: Signal<Vec<ModelPriceRow>>) -> Element {
    let mut model_prices = model_prices; let alias_input = row.alias.clone(); let alias_output = row.alias.clone();
    rsx! { div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4", div { class: "font-mono text-xs text-zinc-300", "{row.alias}" }, label { class: "mt-3 block text-[10px] text-zinc-500", "输入", input { r#type: "number", class: "mt-1 w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-xs", value: "{row.input_per_m}", onchange: move |e| if let Ok(v) = e.value().parse() { if let Some(item) = model_prices.write().iter_mut().find(|item| item.alias == alias_input) { item.input_per_m = v; } } } }, label { class: "mt-2 block text-[10px] text-zinc-500", "输出", input { r#type: "number", class: "mt-1 w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-xs", value: "{row.output_per_m}", onchange: move |e| if let Ok(v) = e.value().parse() { if let Some(item) = model_prices.write().iter_mut().find(|item| item.alias == alias_output) { item.output_per_m = v; } } } } } }
}
