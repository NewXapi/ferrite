//! 分组 tab:分组 CRUD、引用检查、key 归属与派生模型展示。
use dioxus::prelude::*;
use std::collections::BTreeSet;
use crate::api::GroupRow;
use crate::state::EntityStore;

#[component]
pub fn GroupsPanel() -> Element {
    let store = use_context::<EntityStore>();
    let mut groups = store.groups;
    let channels = store.channels;
    let keys = store.api_keys;
    let mut new_name = use_signal(String::new);
    let mut new_display = use_signal(String::new);
    let mut filter = use_signal(|| "全部".to_string());
    let group_snapshot = use_memo(move || groups.read().clone());
    let key_snapshot = use_memo(move || keys.read().clone());
    let channel_snapshot = use_memo(move || channels.read().clone());
    let active_filter = filter();
    let create = move |_| {
        let name = new_name.read().trim().to_string();
        if name.is_empty() || groups.read().iter().any(|g| g.name == name) { return; }
        groups.write().push(GroupRow { name: name.clone(), display: if new_display.read().trim().is_empty() { name } else { new_display.read().trim().to_string() }, enabled: true, description: String::new() });
        new_name.set(String::new()); new_display.set(String::new());
    };
    rsx! {
        div { class: "flex flex-col gap-8",
            section {
                h2 { class: "mb-3 text-lg font-semibold", "分组配置" }
                div { class: "mb-4 flex flex-wrap gap-2",
                    input { class: "min-w-0 flex-1 rounded-md border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs", placeholder: "内部名称", value: "{new_name}", oninput: move |e| new_name.set(e.value()) }
                    input { class: "min-w-0 flex-1 rounded-md border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs", placeholder: "展示名称", value: "{new_display}", oninput: move |e| new_display.set(e.value()) }
                    button { class: "rounded-md bg-zinc-100 px-4 py-2 text-xs font-medium text-zinc-900", onclick: create, "＋ 新建" }
                }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                    for group in group_snapshot.read().iter().cloned() {
                        GroupCard { key: "{group.name}", group, groups, channels, keys }
                    }
                }
            }
            section {
                h2 { class: "mb-3 text-lg font-semibold", "Key 分组归属" }
                div { class: "mb-4 flex flex-wrap gap-2",
                    button { class: if active_filter == "全部" { "rounded-full bg-zinc-100 px-3 py-1 text-xs text-zinc-900" } else { "rounded-full border border-zinc-700 px-3 py-1 text-xs text-zinc-400" }, onclick: move |_| filter.set("全部".into()), "全部" }
                    for group in group_snapshot.read().iter().cloned() {
                        GroupFilter { key: "{group.name}", group, active: active_filter.clone(), filter }
                    }
                }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                    for key in key_snapshot.read().iter().cloned() {
                        if active_filter == "全部" || active_filter == key.group {
                            KeyCard { key: "{key.name}", key_row: key, groups: group_snapshot.read().clone(), keys }
                        }
                    }
                }
            }
            section {
                h2 { class: "mb-3 text-lg font-semibold", "分组可用模型（派生·只读）" }
                div { class: "flex flex-col gap-3",
                    for group in group_snapshot.read().iter().cloned() {
                        DerivedModelsCard { key: "{group.name}", group, channels: channel_snapshot.read().clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn GroupCard(group: GroupRow, groups: Signal<Vec<GroupRow>>, channels: Signal<Vec<crate::api::ChannelRow>>, keys: Signal<Vec<crate::api::KeyRow>>) -> Element {
    let mut groups = groups;
    let channels = channels;
    let keys = keys;
    let name = group.name.clone();
    let channel_refs = channels.read().iter().filter(|c| c.groups.contains(&name)).count();
    let key_refs = keys.read().iter().filter(|k| k.group == name).count();
    let mut display = use_signal(|| group.display.clone());
    let mut description = use_signal(|| group.description.clone());
    let mut editing = use_signal(|| false);
    rsx! {
        div { class: "flex flex-col gap-3 rounded-xl border border-zinc-800 bg-zinc-900 p-4",
            div { class: "flex items-start justify-between gap-2",
                div { div { class: "font-medium", "{group.display}" } div { class: "font-mono text-[10px] text-zinc-500", "{name}" } }
                span { class: if group.enabled { "text-[10px] text-emerald-400" } else { "text-[10px] text-zinc-500" }, if group.enabled { "启用" } else { "停用" } }
            }
            div { class: "text-[10px] text-zinc-500", "渠道 {channel_refs} · Key {key_refs}" }
            if *editing.read() {
                input { class: "rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-xs", value: "{display}", oninput: move |e| display.set(e.value()) }
                input { class: "rounded border border-zinc-700 bg-zinc-950 px-2 py-1 text-xs", value: "{description}", placeholder: "描述", oninput: move |e| description.set(e.value()) }
                button { class: "rounded border border-zinc-600 px-2 py-1 text-xs", onclick: move |_| { if let Some(g) = groups.write().iter_mut().find(|g| g.name == name) { g.display = display.read().clone(); g.description = description.read().clone(); } editing.set(false); }, "保存" }
            } else {
                div { class: "flex gap-2 border-t border-zinc-800 pt-3",
                    button { class: "text-xs text-zinc-400", onclick: move |_| editing.set(true), "编辑" }
                    button { class: "text-xs text-red-400", onclick: move |_| { if channel_refs == 0 && key_refs == 0 { groups.write().retain(|g| g.name != name); } }, "删除" }
                }
            }
        }
    }
}

#[component]
fn KeyCard(key_row: crate::api::KeyRow, groups: Vec<GroupRow>, keys: Signal<Vec<crate::api::KeyRow>>) -> Element {
    let mut keys = keys;
    let key_name = key_row.name.clone();
    rsx! { div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4 text-xs",
        div { class: "font-medium", "{key_row.name}" }
        div { class: "mt-1 font-mono text-[10px] text-zinc-500", "{key_row.masked}" }
        select { class: "mt-3 w-full rounded border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-xs", value: "{key_row.group}", onchange: move |e| { if let Some(item) = keys.write().iter_mut().find(|item| item.name == key_name) { item.group = e.value(); } }, for group in groups { option { value: "{group.name}", selected: group.name == key_row.group, "{group.display}" } } }
    } }
}

#[component]
fn GroupFilter(group: GroupRow, active: String, mut filter: Signal<String>) -> Element {
    let name = group.name.clone();
    rsx! { button { class: if active == name { "rounded-full bg-zinc-100 px-3 py-1 text-xs text-zinc-900" } else { "rounded-full border border-zinc-700 px-3 py-1 text-xs text-zinc-400" }, onclick: move |_| filter.set(name.clone()), "{group.display}" } }
}

#[component]
fn DerivedModelsCard(group: GroupRow, channels: Vec<crate::api::ChannelRow>) -> Element {
    let mut models = BTreeSet::new();
    for channel in channels { if channel.enabled && channel.groups.contains(&group.name) { models.extend(channel.dispatch); } }
    rsx! { div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4",
        div { class: "mb-2 text-sm font-medium", "{group.display}" }
        div { class: "flex flex-wrap gap-2", for model in models { span { class: "rounded bg-zinc-950 px-2 py-1 text-[10px] text-zinc-400", "{model}" } } }
    } }
}
