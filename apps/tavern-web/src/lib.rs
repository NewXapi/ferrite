//! Tavern shell assembled from tavern-web page crates.
//!
//! 优化响应用户需求:
//! 1. 顶栏 FERRITE Logo 绑定点击跳转到 Tavern 品牌主页
//! 2. 导航栏完整包含: 剧本库 / 互动剧情 / 创作中心 / 参数设置
//! 3. 移动端/平板响应式升级 (对齐 refer 移动端参考图):
//!    - 手机端底部常驻原生导航栏 (首页/剧本库/剧情/创作/设置)
use dioxus::prelude::*;
use shared_web::{AuthModal, UserBadge};
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
    let mut auth_modal_open = use_signal(|| false);

    let is_chat = section() == Section::Chat;
    let is_home = section() == Section::Home;
    rsx! {
        div {
            class: "relative flex h-screen w-screen overflow-hidden bg-zinc-950 text-zinc-100 selection:bg-purple-900 selection:text-white font-sans",
            class: if theme_light() { "light" } else { "" },
            // 桌面端顶部胶囊导航条 (仅在非 Chat 且非全屏 Home 时显示)
            if !is_chat && !is_home {
                header { class: "fixed top-3.5 left-1/2 z-30 -translate-x-1/2 pointer-events-auto hidden md:block",
                    div { class: "flex items-center gap-3 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-4 py-1.5 shadow-2xl backdrop-blur-2xl transition-all",
                        // 品牌区域: 点击 FERRITE 直接跳转到剧本库大厅
                        button {
                            class: "group flex items-center gap-1.5 pr-2 border-r border-zinc-800 transition-opacity hover:opacity-80",
                            title: "点击前往 Tavern 剧本大厅",
                            onclick: move |_| section.set(Section::Characters),
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
                        UserBadge {
                            on_open_login: move |_| auth_modal_open.set(true),
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

            // 主工作舞台 (底部预留移动端高度)
            main {
                class: if is_chat {
                    "flex h-full w-full min-h-0 min-w-0 flex-1 flex-col pb-14 md:pb-0"
                } else if is_home {
                    "flex h-full w-full min-h-0 min-w-0 flex-1 flex-col pb-14 md:pb-0"
                } else {
                    "flex h-full w-full min-h-0 min-w-0 flex-1 flex-col p-4 sm:p-6 pt-4 sm:pt-6 md:pt-16 lg:pt-18 pb-18 md:pb-6"
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

            // ==========================================
            // 移动端专用原生底栏 (对齐 refer/3.png 与 refer/272.jpg 移动端标准导航)
            // ==========================================
            nav { class: "fixed bottom-0 inset-x-0 z-40 flex md:hidden h-14 items-center justify-around border-t border-zinc-800/80 bg-zinc-950/95 backdrop-blur-2xl px-2 py-1 select-none",
                button {
                    class: if section() == Section::Home { "flex flex-col items-center gap-0.5 text-purple-400" } else { "flex flex-col items-center gap-0.5 text-zinc-500 hover:text-zinc-300 transition-colors" },
                    onclick: move |_| section.set(Section::Home),
                    span { class: "text-sm", "🏛️" }
                    span { class: "text-[10px] font-medium", "首页" }
                }
                button {
                    class: if section() == Section::Characters { "flex flex-col items-center gap-0.5 text-purple-400" } else { "flex flex-col items-center gap-0.5 text-zinc-500 hover:text-zinc-300 transition-colors" },
                    onclick: move |_| section.set(Section::Characters),
                    span { class: "text-sm", "📚" }
                    span { class: "text-[10px] font-medium", "剧本库" }
                }
                button {
                    class: if section() == Section::Chat { "flex flex-col items-center gap-0.5 text-purple-400" } else { "flex flex-col items-center gap-0.5 text-zinc-500 hover:text-zinc-300 transition-colors" },
                    onclick: move |_| section.set(Section::Chat),
                    span { class: "text-sm", "💬" }
                    span { class: "text-[10px] font-medium", "剧情" }
                }
                button {
                    class: if section() == Section::Studio { "flex flex-col items-center gap-0.5 text-purple-400" } else { "flex flex-col items-center gap-0.5 text-zinc-500 hover:text-zinc-300 transition-colors" },
                    onclick: move |_| section.set(Section::Studio),
                    span { class: "text-sm", "✍️" }
                    span { class: "text-[10px] font-medium", "创作" }
                }
                button {
                    class: if section() == Section::Settings { "flex flex-col items-center gap-0.5 text-purple-400" } else { "flex flex-col items-center gap-0.5 text-zinc-500 hover:text-zinc-300 transition-colors" },
                    onclick: move |_| section.set(Section::Settings),
                    span { class: "text-sm", "⚙️" }
                    span { class: "text-[10px] font-medium", "设置" }
                }
            }

            // 通用认证弹窗 (对齐 shared-web 规范)
            AuthModal {
                open: auth_modal_open(),
                on_close: move |_| auth_modal_open.set(false),
                on_success: move |_user| {
                    // 登录成功直接导向剧情库
                    section.set(Section::Characters);
                    auth_modal_open.set(false);
                },
            }
        }
    }
}
