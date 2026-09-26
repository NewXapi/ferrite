use dioxus::prelude::*;

/// 玩家行动决策单项
#[derive(Clone, PartialEq)]
pub struct ChoiceOption {
    pub key: String,
    pub text: String,
}

/// 玩家决策 (行动选项) 卡片
#[component]
pub fn ChoiceCard(
    title: String,
    options: Vec<ChoiceOption>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "flex flex-col overflow-hidden rounded-2xl border border-purple-500/40 bg-zinc-900/90 shadow-xl shadow-purple-950/20",
            div { class: "flex items-center justify-between border-b border-purple-500/30 bg-purple-950/60 px-4 py-2.5",
                div { class: "flex items-center gap-2",
                    span { class: "{crate::TYPE_DESC} tracking-wide text-purple-200", "{title}" }
                }
                span { class: "{crate::TYPE_LABEL} text-purple-300/60", "点击选择分支行动" }
            }
            div { class: "flex flex-col divide-y divide-zinc-800/60 p-1.5",
                for opt in options {
                    {
                        let opt_text = opt.text.clone();
                        let opt_key = opt.key.clone();
                        rsx! {
                            button {
                                key: "{opt.key}",
                                class: "group flex items-start gap-3 rounded-xl p-2.5 text-left transition-colors hover:bg-purple-900/20",
                                onclick: move |_| on_select.call(opt_text.clone()),
                                span { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded-md bg-purple-900/60 {crate::TYPE_DESC} text-purple-300 group-hover:bg-purple-600 group-hover:text-white transition-colors",
                                    "{opt_key}"
                                }
                                span { class: "{crate::TYPE_DESC} leading-5 group-hover:text-zinc-100 transition-colors",
                                    "{opt.text}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
