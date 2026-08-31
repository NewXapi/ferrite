use dioxus::prelude::*;
use std::collections::BTreeSet;
use crate::api::PresentationRow;
use crate::state::EntityStore;

#[component]
pub fn ShowcasePanel() -> Element {
    let store = use_context::<EntityStore>();
    let presentations = store.presentations;
    let aliases = store.channels.read().iter().flat_map(|channel| channel.dispatch.iter().cloned()).collect::<BTreeSet<_>>();
    rsx! { div { class: "space-y-4",
        div { class: "text-sm text-zinc-500", "别名来自渠道调度模型；这里仅配置总览展示信息。" }
        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
            for alias in aliases { ShowcaseCard { key: "{alias}", alias, presentations } }
        }
    } }
}

#[component]
fn ShowcaseCard(alias: String, presentations: Signal<Vec<PresentationRow>>) -> Element {
    let mut presentations = presentations;
    let row = presentations.read().iter().find(|row| row.alias == alias).cloned().unwrap_or(PresentationRow { alias: alias.clone(), display_name: String::new(), description: String::new(), icon: String::new(), badge: String::new(), tags: vec![], sort_order: 0, visible: true });
    let display = if row.display_name.is_empty() { alias.clone() } else { row.display_name.clone() };
    let alias_name = alias.clone();
    let alias_description = alias.clone();
    let alias_visible = alias.clone();
    rsx! { div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4",
        div { class: "mb-4 flex aspect-[4/3] items-center justify-center rounded-lg border border-zinc-800 bg-zinc-950 text-3xl text-zinc-500", if row.icon.is_empty() { "◆" } else { img { src: "{row.icon}", class: "h-14 w-14 object-contain" } } }
        div { class: "truncate text-sm font-medium", "{display}" }
        div { class: "mt-1 font-mono text-[10px] text-zinc-500", "{alias}" }
        input { class: "mt-3 w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-xs", value: "{row.display_name}", placeholder: "展示名", oninput: move |e| upsert(&mut presentations, &alias_name, |item| item.display_name = e.value()) }
        textarea { class: "mt-2 w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-xs", value: "{row.description}", placeholder: "描述", oninput: move |e| upsert(&mut presentations, &alias_description, |item| item.description = e.value()) }
        label { class: "mt-3 flex items-center gap-2 text-xs text-zinc-400", input { r#type: "checkbox", checked: row.visible, onchange: move |e| upsert(&mut presentations, &alias_visible, |item| item.visible = e.checked()) }, "前台可见" }
    } }
}
fn upsert(presentations: &mut Signal<Vec<PresentationRow>>, alias: &str, edit: impl FnOnce(&mut PresentationRow)) { let mut rows = presentations.write(); if let Some(row) = rows.iter_mut().find(|row| row.alias == alias) { edit(row); } else { let mut row = PresentationRow { alias: alias.into(), display_name: String::new(), description: String::new(), icon: String::new(), badge: String::new(), tags: vec![], sort_order: 0, visible: true }; edit(&mut row); rows.push(row); } }
