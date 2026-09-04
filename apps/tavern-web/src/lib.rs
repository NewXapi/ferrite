//! Tavern shell assembled from tavern-web page crates.
//!
//! 优化顶栏：在互动剧情 (Chat) 模式下，顶栏完全由 ChatPage 沉浸式接管 (对齐图2)，
//! 消除任何悬浮顶栏与模型胶囊的重叠遮挡；
//! 在剧本库与设置页，展示极简优雅的顶栏，支持跨页面无缝流转。

use dioxus::prelude::*;
use tavern_page_characters::CharactersPage;
use tavern_page_chat::ChatPage;
use tavern_page_settings::SettingsPage;

/// Top-level tavern sections, in navigation order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Characters,
    Chat,
    Settings,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Characters => "剧本库",
            Section::Chat => "互动剧情",
            Section::Settings => "参数设置",
        }
    }
}

pub const SECTIONS: [Section; 3] = [Section::Characters, Section::Chat, Section::Settings];

/// Root tavern shell
#[component]
pub fn TavernApp() -> Element {
    let mut section = use_signal(|| Section::Chat); // 默认直接沉浸到图2的互动剧情态
    let mut theme_light = use_signal(|| false);

    let is_chat = section() == Section::Chat;

    rsx! {
        div {
            class: "relative flex h-screen w-screen overflow-hidden bg-zinc-950 text-zinc-100 selection:bg-purple-900 selection:text-white font-sans",
            class: if theme_light() { "light" } else { "" },

            // 仅在非 Chat 模式 (剧本库 / 设置) 显示顶部导航条
            if !is_chat {
                header { class: "fixed top-3.5 left-1/2 z-30 -translate-x-1/2 pointer-events-auto",
                    div { class: "flex items-center gap-3 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-4 py-1.5 shadow-2xl backdrop-blur-2xl transition-all",
                        div { class: "flex items-center gap-1.5 pr-2 border-r border-zinc-800",
                            span { class: "font-serif text-xs font-bold tracking-wider text-zinc-100", "FERRITE" }
                            span { class: "text-[9px] font-mono tracking-widest text-zinc-500 uppercase", "TAVERN" }
                        }
                        nav { class: "flex items-center gap-1",
                            for s in SECTIONS {
                                {
                                    let is_current = section() == s;
                                    rsx! {
                                        button {
                                            key: "{s.label()}",
                                            class: if is_current {
                                                "rounded-full bg-zinc-100 px-3.5 py-1 text-xs font-semibold text-zinc-900 shadow-sm transition-all"
                                            } else {
                                                "rounded-full px-3.5 py-1 text-xs font-medium text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-200"
                                            },
                                            onclick: move |_| section.set(s),
                                            "{s.label()}"
                                        }
                                    }
                                }
                            }
                        }
                        button {
                            class: "flex h-6 w-6 items-center justify-center rounded-full text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200 text-xs",
                            title: "切换光暗模式",
                            onclick: move |_| theme_light.set(!theme_light()),
                            if theme_light() { "☀️" } else { "🌙" }
                        }
                    }
                }
            }

            // 主工作舞台
            main {
                class: if is_chat {
                    "flex h-full w-full min-h-0 min-w-0 flex-1 flex-col"
                } else {
                    "flex h-full w-full min-h-0 min-w-0 flex-1 flex-col p-4 sm:p-6 pt-16 sm:pt-18"
                },
                match section() {
                    Section::Characters => rsx! {
                        CharactersPage {
                            on_enter_story: move |_| section.set(Section::Chat),
                        }
                    },
                    Section::Chat => rsx! {
                        ChatPage {
                            on_goto_characters: move |_| section.set(Section::Characters),
                            on_goto_settings: move |_| section.set(Section::Settings),
                            on_toggle_theme: move |_| theme_light.set(!theme_light()),
                            theme_light: theme_light(),
                        }
                    },
                    Section::Settings => rsx! {
                        SettingsPage {}
                    },
                }
            }
        }
    }
}
