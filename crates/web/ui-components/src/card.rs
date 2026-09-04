use dioxus::prelude::*;

/// 状态/备忘录卡片 (任务系统、档案、备忘录)
#[component]
pub fn StatusCard(
    title: String,
    #[props(default = "emerald".to_string())] color: String,
    children: Element,
) -> Element {
    let border_color = match color.as_str() {
        "purple" => "border-purple-500/40 bg-purple-950/20 text-purple-200",
        "amber" => "border-amber-500/40 bg-amber-950/20 text-amber-200",
        "rose" => "border-rose-500/40 bg-rose-950/20 text-rose-200",
        "cyan" => "border-cyan-500/40 bg-cyan-950/20 text-cyan-200",
        _ => "border-emerald-500/40 bg-emerald-950/20 text-emerald-200",
    };
    let badge_color = match color.as_str() {
        "purple" => "bg-purple-500/20 text-purple-300 border-purple-500/30",
        "amber" => "bg-amber-500/20 text-amber-300 border-amber-500/30",
        "rose" => "bg-rose-500/20 text-rose-300 border-rose-500/30",
        "cyan" => "bg-cyan-500/20 text-cyan-300 border-cyan-500/30",
        _ => "bg-emerald-500/20 text-emerald-300 border-emerald-500/30",
    };
    rsx! {
        div { class: "flex flex-col gap-2 rounded-2xl border p-3.5 shadow-md {border_color}",
            div { class: "flex items-center gap-2",
                span { class: "rounded-lg border px-2 py-0.5 text-xs font-semibold {badge_color}",
                    "{title}"
                }
            }
            div { class: "text-xs leading-5 text-zinc-300",
                {children}
            }
        }
    }
}

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
                    span { class: "text-sm", "🎮" }
                    span { class: "text-xs font-semibold tracking-wide text-purple-200", "{title}" }
                }
                span { class: "text-[10px] text-purple-300/60", "点击选择分支行动" }
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
                                span { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded-md bg-purple-900/60 text-xs font-bold text-purple-300 group-hover:bg-purple-600 group-hover:text-white transition-colors",
                                    "{opt_key}"
                                }
                                span { class: "text-xs leading-5 text-zinc-300 group-hover:text-zinc-100 transition-colors",
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
