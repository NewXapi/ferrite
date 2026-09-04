//! Tavern shell assembled from tavern-web page crates.
//!
//! 优化响应用户需求:
//! 1. 顶栏 FERRITE Logo 绑定点击跳转到 Tavern 品牌主页
//! 2. 导航栏完整包含: 剧本库 / 互动剧情 / 创作中心 / 参数设置
//! 3. 支持在剧本库点击「创作」直达创作中心
//! 4. 互动剧情下顶栏完全接管，输入框上方提供模型切换与快捷交互

use dioxus::prelude::*;
use tavern_page_characters::{CharactersPage, StudioPage};
use tavern_page_chat::ChatPage;
use tavern_page_home::HomePage;
use tavern_page_settings::SettingsPage;

/// Top-level tavern sections, in navigation order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Home,
    Characters,
    Chat,
    Studio,
    Settings,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Home => "首页",
            Section::Characters => "剧本库",
            Section::Chat => "互动剧情",
            Section::Studio => "创作中心",
            Section::Settings => "参数设置",
        }
    }
}

pub const NAV_SECTIONS: [Section; 4] = [
    Section::Characters,
    Section::Chat,
    Section::Studio,
    Section::Settings,
];

/// Root tavern shell
#[component]
pub fn TavernApp() -> Element {
    let mut section = use_signal(|| Section::Characters); // 默认直接进入 5 栏剧本库大厅
    let mut theme_light = use_signal(|| false);

    let is_chat = section() == Section::Chat;
    let is_home = section() == Section::Home;

    rsx! {
        div {
            class: "relative flex h-screen w-screen overflow-hidden bg-zinc-950 text-zinc-100 selection:bg-purple-900 selection:text-white font-sans",
            class: if theme_light() { "light" } else { "" },

            // 仅在非 Chat 且非全屏 Home 时，显示顶部胶囊导航条 (包含创作中心)
            if !is_chat && !is_home {
                header { class: "fixed top-3.5 left-1/2 z-30 -translate-x-1/2 pointer-events-auto",
                    div { class: "flex items-center gap-3 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-4 py-1.5 shadow-2xl backdrop-blur-2xl transition-all",
                        // 品牌区域: 点击 FERRITE 直接跳转到主页 Landing
                        button {
                            class: "group flex items-center gap-1.5 pr-2 border-r border-zinc-800 transition-opacity hover:opacity-80",
                            title: "点击返回 Tavern 官方品牌主页",
                            onclick: move |_| section.set(Section::Home),
                            span { class: "font-serif text-xs font-bold tracking-wider text-transparent bg-clip-text bg-gradient-to-r from-purple-200 to-pink-200 group-hover:from-white group-hover:to-purple-200",
                                "FERRITE"
                            }
                            span { class: "text-[9px] font-mono tracking-widest text-zinc-500 uppercase", "TAVERN" }
                        }

                        nav { class: "flex items-center gap-1",
                            for s in NAV_SECTIONS {
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
                class: if is_chat || is_home {
                    "flex h-full w-full min-h-0 min-w-0 flex-1 flex-col"
                } else {
                    "flex h-full w-full min-h-0 min-w-0 flex-1 flex-col p-4 sm:p-6 pt-16 sm:pt-18"
                },
                match section() {
                    Section::Home => rsx! {
                        HomePage {
                            on_start: move |_| section.set(Section::Characters),
                            on_explore_studio: move |_| section.set(Section::Studio),
                        }
                    },
                    Section::Characters => rsx! {
                        CharactersPage {
                            on_enter_story: move |_| section.set(Section::Chat),
                            on_goto_studio: move |_| section.set(Section::Studio),
                        }
                    },
                    Section::Chat => rsx! {
                        ChatPage {
                            on_goto_characters: move |_| section.set(Section::Characters),
                            on_goto_settings: move |_| section.set(Section::Settings),
                            on_goto_home: move |_| section.set(Section::Home),
                            on_toggle_theme: move |_| theme_light.set(!theme_light()),
                            theme_light: theme_light(),
                        }
                    },
                    Section::Studio => rsx! {
                        StudioPage {}
                    },
                    Section::Settings => rsx! {
                        SettingsPage {}
                    },
                }
            }
        }
    }
}
