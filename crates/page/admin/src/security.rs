use dioxus::prelude::*;
use crate::api::RateRuleRow;
use crate::state::EntityStore;

#[component]
pub fn SecurityPanel() -> Element {
    let store = use_context::<EntityStore>();
    let mut rules = store.rate_rules;
    let mut words = store.banned_words;
    let mut new_rule = use_signal(String::new);
    let mut new_word = use_signal(String::new);
    let mut blocking = use_signal(|| true);
    rsx! {
        div { class: "space-y-8",
            section {
                h2 { class: "mb-3 text-lg font-semibold", "速率限制" }
                div { class: "mb-3 flex gap-2",
                    input { class: "min-w-0 flex-1 rounded-md border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs", placeholder: "规则名称", value: "{new_rule}", oninput: move |e| new_rule.set(e.value()) }
                    button { class: "rounded-md bg-zinc-100 px-4 py-2 text-xs text-zinc-900", onclick: move |_| { let name = new_rule(); if !name.is_empty() { rules.write().push(RateRuleRow { name, scope: "所有分组".into(), limit: 60, window: "每分钟".into(), enabled: true }); new_rule.set(String::new()); } }, "＋ 添加规则" }
                }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                    for (index, rule) in rules.read().iter().cloned().enumerate() {
                        SecurityRuleCard { key: "{rule.name}", index, rule, rules }
                    }
                }
            }
            section {
                h2 { class: "mb-3 text-lg font-semibold", "敏感词拦截" }
                label { class: "mb-3 flex items-center gap-2 text-xs text-zinc-400", input { r#type: "checkbox", checked: blocking, onchange: move |e| blocking.set(e.checked()) }, "开启拦截" }
                div { class: "flex gap-2",
                    input { class: "min-w-0 flex-1 rounded-md border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs", placeholder: "输入敏感词", value: "{new_word}", oninput: move |e| new_word.set(e.value()) }
                    button { class: "rounded-md border border-zinc-600 px-4 py-2 text-xs", onclick: move |_| { let word = new_word(); if !word.trim().is_empty() { words.write().push(word); new_word.set(String::new()); } }, "添加" }
                }
                div { class: "mt-4 flex flex-wrap gap-2",
                    for (index, word) in words.read().iter().cloned().enumerate() {
                        WordChip { key: "{word}", index, word, words }
                    }
                }
            }
        }
    }
}

#[component]
fn SecurityRuleCard(index: usize, rule: RateRuleRow, rules: Signal<Vec<RateRuleRow>>) -> Element {
    let mut rules = rules;
    let name = rule.name.clone();
    rsx! { div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4 text-xs",
        div { class: "flex justify-between gap-2", div { class: "font-medium", "{rule.name}" }, button { class: if rule.enabled { "text-emerald-400" } else { "text-zinc-500" }, onclick: move |_| rules.write()[index].enabled = !rule.enabled, if rule.enabled { "启用" } else { "停用" } } }
        div { class: "mt-2 text-zinc-500", "{rule.scope} · {rule.limit}/{rule.window}" }
        button { class: "mt-4 text-red-400", onclick: move |_| rules.write().retain(|row| row.name != name), "删除" }
    } }
}

#[component]
fn WordChip(index: usize, word: String, words: Signal<Vec<String>>) -> Element {
    let mut words = words;
    rsx! { button { class: "rounded-full border border-zinc-700 bg-zinc-900 px-3 py-1 text-xs text-zinc-300", onclick: move |_| { words.write().remove(index); }, "{word} ×" } }
}
