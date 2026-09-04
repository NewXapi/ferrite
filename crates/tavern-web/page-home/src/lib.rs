//! tavern-page-home — FERRITE / AI风月品牌主页 (Landing Page)。
//!
//! 完整对齐图7设计:
//! - 顶部全宽品牌导航条(功能特色、最新更新、精选作品、创作计划)
//! - Hero 区域:
//!   - 巨幅大标题 `FERRITE // 风月AI` + 紫粉渐变次级高光 `中文 AI 角色扮演与沉浸式文游平台`
//!   - 官方品牌站文案描述
//!   - 三重核心特性微标(多模型支持、自由创作、跨设备同步)
//!   - 醒目行动按钮(「开始体验」「了解更多」)
//!   - 核心数据里程碑(72.1亿+ 对话次数、10万+ 用户、5种 AI模型)
//!   - 右侧高质感玻璃拟态卡片展映(热门角标、互动作品示例、评分与热度)
//! - 沉浸式三大特性展厅(交互式分支决策、记忆与世界书、作者控制台)

use dioxus::prelude::*;

#[component]
pub fn HomePage(
    #[props(default)] on_start: EventHandler<()>,
    #[props(default)] on_explore_studio: EventHandler<()>,
) -> Element {
    let mut show_learn_more = use_signal(|| false);

    rsx! {
        div { class: "relative flex h-full w-full flex-col overflow-y-auto bg-gradient-to-b from-zinc-950 via-[#12071f] to-zinc-950 text-zinc-100 selection:bg-purple-900 selection:text-white",
            // ==========================================
            // 顶部独立品牌条 (参考图7顶部)
            // ==========================================
            header { class: "sticky top-0 z-20 flex h-16 shrink-0 items-center justify-between border-b border-purple-900/20 bg-zinc-950/60 px-6 backdrop-blur-xl sm:px-12",
                // 左侧品牌
                div { class: "flex items-center gap-3",
                    span { class: "font-serif text-lg font-black tracking-wider text-transparent bg-clip-text bg-gradient-to-r from-purple-200 via-pink-200 to-white",
                        "AI风月 / FERRITE"
                    }
                    span { class: "hidden rounded-full border border-purple-500/30 bg-purple-500/10 px-2 py-0.5 text-[10px] font-mono tracking-widest text-purple-300 sm:inline-block",
                        "NEXT-GEN HARNESS"
                    }
                }

                // 中部功能连接
                nav { class: "hidden items-center gap-8 text-xs font-medium text-zinc-400 md:flex",
                    button { class: "hover:text-purple-300 transition-colors", "功能特色" }
                    button { class: "hover:text-purple-300 transition-colors", "最新更新" }
                    button { class: "hover:text-purple-300 transition-colors", "精选作品" }
                    button { class: "hover:text-purple-300 transition-colors", "创作者计划" }
                }

                // 右侧进入行动
                div { class: "flex items-center gap-3",
                    button {
                        class: "rounded-full border border-purple-500/40 bg-gradient-to-r from-purple-600 via-fuchsia-600 to-pink-600 px-4 py-1.5 text-xs font-semibold text-white shadow-lg shadow-purple-600/30 transition-all hover:scale-105 hover:shadow-purple-600/50",
                        onclick: move |_| on_start.call(()),
                        "立即体验 ➜"
                    }
                }
            }

            // ==========================================
            // Hero 主视觉区 (精准复刻图7)
            // ==========================================
            div { class: "mx-auto flex w-full max-w-6xl flex-1 flex-col justify-center px-6 py-12 lg:flex-row lg:items-center lg:gap-12 lg:py-16",
                // 左侧文案排版
                div { class: "flex flex-1 flex-col gap-6",
                    // 巨幅标题
                    div { class: "flex flex-col gap-2",
                        h1 { class: "font-serif text-4xl font-extrabold tracking-tight text-white sm:text-5xl lg:text-6xl",
                            "AI风月 / 风月AI"
                        }
                        h2 { class: "font-serif text-2xl font-bold tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-rose-300 via-pink-400 to-purple-400 sm:text-3xl lg:text-4xl",
                            "中文 AI 角色扮演与沉浸式文游平台"
                        }
                    }

                    // 官方品牌站描述 (对齐图7)
                    p { class: "max-w-xl text-sm leading-6 text-zinc-400 sm:text-base",
                        "官方品牌站：多模型角色扮演、世界书创作、沉浸式文游推演、决策互动分支。复刻前沿文游互动体验，从这里开始。"
                    }

                    // 特性小胶囊组 (对齐图7)
                    div { class: "flex flex-wrap items-center gap-3 text-xs text-zinc-300",
                        div { class: "flex items-center gap-1.5 rounded-full border border-purple-500/20 bg-purple-950/40 px-3 py-1",
                            span { "⊙" }
                            span { "多模型支持" }
                        }
                        div { class: "flex items-center gap-1.5 rounded-full border border-purple-500/20 bg-purple-950/40 px-3 py-1",
                            span { "🕒" }
                            span { "自由创作" }
                        }
                        div { class: "flex items-center gap-1.5 rounded-full border border-purple-500/20 bg-purple-950/40 px-3 py-1",
                            span { "↔" }
                            span { "跨设备同步" }
                        }
                    }

                    // 行动按钮组 (对齐图7)
                    div { class: "flex flex-wrap items-center gap-4 pt-2",
                        button {
                            class: "flex items-center gap-2 rounded-xl bg-gradient-to-r from-purple-600 via-pink-600 to-rose-600 px-6 py-3 text-sm font-bold text-white shadow-xl shadow-purple-600/40 transition-all hover:scale-105 hover:shadow-purple-600/60",
                            onclick: move |_| on_start.call(()),
                            span { "开始体验" }
                            span { "➜" }
                        }
                        button {
                            class: "rounded-xl border border-zinc-700/80 bg-zinc-900/60 px-6 py-3 text-sm font-semibold text-zinc-200 transition-colors hover:bg-zinc-800",
                            onclick: move |_| show_learn_more.set(!show_learn_more()),
                            "了解更多"
                        }
                        button {
                            class: "rounded-xl border border-purple-500/30 bg-purple-950/40 px-5 py-3 text-sm font-semibold text-purple-300 transition-colors hover:bg-purple-900/40",
                            onclick: move |_| on_explore_studio.call(()),
                            "🛠️ 创作者工作台"
                        }
                    }

                    // 真实指标数据展示 (精准复刻图7底部: 72.1亿+、10万+、5种)
                    div { class: "grid grid-cols-3 gap-4 border-t border-purple-900/20 pt-8 sm:gap-8",
                        div { class: "flex flex-col gap-1",
                            span { class: "font-serif text-2xl font-black tracking-tight text-white sm:text-3xl",
                                "72.1亿+"
                            }
                            span { class: "text-xs text-zinc-500", "对话交互次数" }
                        }
                        div { class: "flex flex-col gap-1",
                            span { class: "font-serif text-2xl font-black tracking-tight text-white sm:text-3xl",
                                "10万+"
                            }
                            span { class: "text-xs text-zinc-500", "活跃创作者与玩家" }
                        }
                        div { class: "flex flex-col gap-1",
                            span { class: "font-serif text-2xl font-black tracking-tight text-white sm:text-3xl",
                                "5种+"
                            }
                            span { class: "text-xs text-zinc-500", "前沿多大模型内核" }
                        }
                    }
                }

                // 右侧精选作品卡片展映 (精准复刻图7右侧发光卡片)
                div { class: "mt-10 flex flex-1 justify-center lg:mt-0",
                    div {
                        class: "group relative w-full max-w-sm overflow-hidden rounded-3xl border border-purple-500/30 bg-gradient-to-b from-purple-950/40 via-zinc-900/80 to-zinc-950 p-4 shadow-2xl shadow-purple-950/60 transition-all hover:border-purple-500/60 hover:shadow-purple-600/30 cursor-pointer",
                        onclick: move |_| on_start.call(()),

                        // 卡片顶部高光立绘/插画展台
                        div { class: "relative flex h-64 w-full items-center justify-center overflow-hidden rounded-2xl bg-gradient-to-br from-purple-900/40 via-fuchsia-950/30 to-zinc-900",
                            // 热门角标 (对齐图7粉色药丸)
                            div { class: "absolute left-3 top-3 z-10 flex items-center gap-1 rounded-full bg-pink-500/90 px-2.5 py-0.5 text-[10px] font-bold text-white shadow-md",
                                span { "🔥" }
                                span { "热门" }
                            }
                            // 装饰立绘展示
                            div { class: "flex flex-col items-center gap-2",
                                span { class: "text-6xl filter drop-shadow-xl", "🎭" }
                                span { class: "font-serif text-xs font-semibold tracking-wider text-purple-200", "【超真实】明星娱乐圈模拟器" }
                            }
                        }

                        // 卡片底栏信息 (对齐图7)
                        div { class: "flex flex-col gap-2 pt-4 px-1",
                            span { class: "font-serif text-base font-bold text-zinc-100",
                                "热门 AI 互动作品示例"
                            }
                            div { class: "flex items-center justify-between text-xs text-zinc-400",
                                div { class: "flex items-center gap-3",
                                    span { class: "text-amber-300 font-semibold", "★ 9.7" }
                                    span { class: "text-zinc-500", "🔥 72.1亿" }
                                }
                                span { class: "text-purple-400 group-hover:translate-x-1 transition-transform font-medium",
                                    "点击进入 ➜"
                                }
                            }
                        }
                    }
                }
            }

            // ==========================================
            // 特色展开折叠区
            // ==========================================
            if show_learn_more() {
                div { class: "mx-auto w-full max-w-6xl px-6 pb-16",
                    div { class: "grid grid-cols-1 gap-6 md:grid-cols-3",
                        div { class: "flex flex-col gap-2 rounded-2xl border border-purple-900/30 bg-zinc-900/60 p-5",
                            span { class: "text-2xl", "🎮" }
                            span { class: "text-sm font-bold text-white", "行动选项与分支决策" }
                            p { class: "text-xs leading-5 text-zinc-400",
                                "彻底摆脱单调的一问一答。系统根据剧情走向动态生成多条推演选项，点击即可主导世界线分支。"
                            }
                        }
                        div { class: "flex flex-col gap-2 rounded-2xl border border-purple-900/30 bg-zinc-900/60 p-5",
                            span { class: "text-2xl", "🧠" }
                            span { class: "text-sm font-bold text-white", "通告契约与记忆系统" }
                            p { class: "text-xs leading-5 text-zinc-400",
                                "内置任务对赌协议与娱乐圈线索捕捉备忘录，剧情进展自动记忆沉淀，多轮对话永不遗忘关键脉络。"
                            }
                        }
                        div { class: "flex flex-col gap-2 rounded-2xl border border-purple-900/30 bg-zinc-900/60 p-5",
                            span { class: "text-2xl", "🛠️" }
                            span { class: "text-sm font-bold text-white", "作者视角创作控制台" }
                            p { class: "text-xs leading-5 text-zinc-400",
                                "专属 Studio 创作中心：提供世界观、Prompt 矩阵、进场封面名句与行动分支的全面编排与一键调试。"
                            }
                        }
                    }
                }
            }
        }
    }
}
