use dioxus::prelude::*;
use crate::api::ChannelRow;
use crate::state::EntityStore;

#[component]
pub fn ChannelsPanel() -> Element {
    let store = use_context::<EntityStore>();
    let mut channels = store.channels;
    let mut draft_name = use_signal(String::new);
    let mut draft_url = use_signal(String::new);
    rsx! {
        div { class: "space-y-4",
            div { class: "flex flex-wrap gap-2",
                input { class: "min-w-0 flex-1 rounded-md border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs", placeholder: "渠道名称", value: "{draft_name}", oninput: move |e| draft_name.set(e.value()) }
                input { class: "min-w-0 flex-1 rounded-md border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs", placeholder: "https://...", value: "{draft_url}", oninput: move |e| draft_url.set(e.value()) }
                button { class: "rounded-md bg-zinc-100 px-4 py-2 text-xs text-zinc-900", onclick: move |_| {
                    let name = draft_name();
                    if !name.trim().is_empty() {
                        channels.write().push(ChannelRow { name, url: draft_url(), keys: String::new(), candidates: vec![], dispatch: vec![], enabled: true, groups: vec!["default".into()], remark: String::new() });
                        draft_name.set(String::new()); draft_url.set(String::new());
                    }
                }, "＋ 新建渠道" }
            }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                for (index, channel) in channels.read().iter().cloned().enumerate() {
                    {
                        let remove_name = channel.name.clone();
                        let enabled = channel.enabled;
                        rsx! { div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-4 text-xs",
                            div { class: "flex justify-between gap-2", div { class: "truncate font-medium", "{channel.name}" }, button { class: if enabled { "text-emerald-400" } else { "text-zinc-500" }, onclick: move |_| channels.write()[index].enabled = !enabled, if enabled { "启用" } else { "停用" } } }
                            div { class: "mt-2 truncate font-mono text-[10px] text-zinc-500", "{channel.url}" }
                            div { class: "mt-3 flex flex-wrap gap-1", for group in channel.groups { span { class: "rounded bg-zinc-950 px-2 py-1 text-[10px] text-zinc-400", "{group}" } } }
                            div { class: "mt-2 flex flex-wrap gap-1", for model in channel.dispatch { span { class: "rounded bg-zinc-800 px-2 py-1 text-[10px]", "{model}" } } }
                            button { class: "mt-4 text-red-400", onclick: move |_| channels.write().retain(|row| row.name != remove_name), "删除" }
                        } }
                    }
                }
            }
        }
    }
}
