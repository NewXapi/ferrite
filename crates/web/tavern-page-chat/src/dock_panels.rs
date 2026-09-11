//! dock 面板组件：把原侧栏/中央区散落的内联 UI 抽成四区可停靠的面板。
//!
//! 面板标题栏是拖拽把手：跨区移动与区内重排的事件链挂在
//! [`crate::layout`] 的根容器上，本文件的面板只负责声明 draggable id
//! 与收起态图标轨。

use dioxus::prelude::*;
use tavern_state::{STATE, open_chat};

use crate::dock;

/// 通用面板外壳：带标题栏（拖拽把手）与收起/展开。
///
/// 标题栏 `ondblclick` 展开该区（收起态只剩 40px 图标轨时双击恢复）；
/// 右上角按钮手动收起。
#[component]
pub fn DockPanel(
    /// 面板稳定 id（与 [`dock::zone_item_ids`] 对应）。
    item_id: &'static str,
    /// 面板标题。
    title: &'static str,
    /// 收起态图标（40px 图标轨显示）。
    icon: &'static str,
    /// 内容区。
    content: Element,
) -> Element {
    let zone = dock::zone_of_item(item_id).unwrap_or_else(|| dock::default_zone(item_id));
    let collapsed = dock::zone_collapsed(zone);

    rsx! {
        div {
            class: "flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border border-zinc-800/60 bg-zinc-950/60",
            role: "group",
            aria_label: "面板 {title}",
            "data-testid": "panel-{item_id}",

            div {
                class: "flex h-7 shrink-0 cursor-grab items-center gap-1.5 border-b border-zinc-800/50 bg-zinc-900/80 px-2 text-[10px] font-bold text-zinc-400 select-none",
                // ondoubleclick：dioxus 0.7 起 ondblclick 已废弃
                ondoubleclick: move |_| {
                    let z = dock::zone_of_item(item_id).unwrap_or_else(|| dock::default_zone(item_id));
                    if dock::zone_collapsed(z) {
                        dock::collapse_zone(z, false);
                    }
                },
                span { class: "text-[9px] text-zinc-600", "⠿" }
                span { "{title}" }
                button {
                    class: "ml-auto text-[10px] text-zinc-500 hover:text-zinc-200",
                    title: if collapsed { "展开面板" } else { "收起面板" },
                    name: "btn-panel-collapse-{item_id}",
                    aria_label: "收起面板 {title}",
                    onclick: move |_| dock::collapse_zone(zone, true),
                    if collapsed { "⊞" } else { "⊟" }
                }
            }

            if collapsed {
                div { class: "flex flex-1 items-center justify-center",
                    button {
                        class: "flex h-8 w-8 items-center justify-center rounded-lg bg-zinc-800/70 text-sm hover:bg-zinc-700",
                        name: "btn-expand-{item_id}",
                        aria_label: "展开面板 {title}",
                        onclick: move |_| {
                            let z = dock::zone_of_item(item_id).unwrap_or_else(|| dock::default_zone(item_id));
                            dock::collapse_zone(z, false);
                        },
                        "{icon}"
                    }
                }
            } else {
                div { class: "min-h-0 flex-1 overflow-y-auto p-2 no-scrollbar", { content } }
            }
        }
    }
}

/// 角色卡面板：角色徽标 + 名称简介 + 剧本详情/赞赏快捷按钮。
#[component]
pub fn CharacterPanel(
    /// 剧本详情弹窗开关（只读传值，面板内取写引用）。
    #[props(into)]
    detail_modal_open: Signal<bool>,
    /// 赞赏作品弹窗开关（只读传值，面板内取写引用）。
    #[props(into)]
    donate_modal_open: Signal<bool>,
) -> Element {
    let mut detail_modal_open = detail_modal_open;
    let mut donate_modal_open = donate_modal_open;
    let character_info = use_memo(move || {
        STATE.with(|s| {
            s.character
                .as_ref()
                .map(|(_, char)| {
                    let name = &char.name;
                    let desc_str = char.description.clone();
                    format!(
                        "{} - {}",
                        name,
                        desc_str.chars().take(30).collect::<String>()
                    )
                })
                .unwrap_or("未选择角色".to_string())
        })
    });

    let char_monogram = use_memo(move || {
        STATE.with(|s| {
            s.character
                .as_ref()
                .and_then(|(_, c)| c.name.chars().next())
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| "?".to_string())
        })
    });

    rsx! {
        div { class: "flex flex-col gap-2",
            div { class: "flex flex-col gap-1",
                div { class: "flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-purple-600 to-pink-600 text-sm font-bold text-white shadow-md",
                    aria_label: "当前角色",
                    "{char_monogram()}"
                }
                span { class: "truncate text-xs font-bold text-zinc-100", "{character_info()}" }
            }
            div { class: "grid grid-cols-2 gap-2",
                button {
                    class: "flex items-center justify-center gap-1.5 rounded-xl border border-zinc-800 bg-zinc-800/70 py-2 text-xs font-medium text-zinc-200 transition-colors hover:bg-zinc-700 active:scale-95",
                    name: "btn-detail",
                    onclick: move |_| detail_modal_open.set(true),
                    span { "📖" }
                    span { "剧本详情" }
                }
                button {
                    class: "flex items-center justify-center gap-1.5 rounded-xl border border-amber-500/30 bg-amber-500/10 py-2 text-xs font-medium text-amber-300 transition-colors hover:bg-amber-500/20 active:scale-95",
                    name: "btn-donate",
                    onclick: move |_| donate_modal_open.set(true),
                    span { "☕" }
                    span { "赞赏作品" }
                }
            }
        }
    }
}

/// 会话时间线面板：新对话按钮 + 会话列表（跟随 `STATE.character` 拉取）。
#[component]
pub fn SessionsPanel() -> Element {
    let mut sessions = use_signal(Vec::<crate::SessionItem>::new);

    use_effect(move || {
        let file_name = STATE.with(|s| s.character.as_ref().map(|(f, _)| f.clone()));
        if let Some(file_name) = file_name {
            spawn(async move {
                match tavern_client::recent_chats(file_name).await {
                    Ok(chats) => {
                        sessions.set(
                            chats
                                .into_iter()
                                .map(|c| crate::SessionItem { title: c.file_name })
                                .collect(),
                        );
                    }
                    Err(e) => {
                        STATE.with_mut(|s| {
                            s.last_error = Some(format!("加载聊天列表失败: {e}"));
                        });
                    }
                }
            });
        }
    });

    let sessions_v: Vec<crate::SessionItem> = sessions();

    // 会话按钮独立构建，避免外层 rsx 里 for + 嵌套 rsx! 的解析坑
    let session_items: Vec<Element> = sessions_v
        .iter()
        .map(|s| {
            let is_active = STATE.with(|st| st.chat.as_deref() == Some(s.title.as_str()));
            let title = s.title.clone();
            let btn_class = if is_active {
                "group flex w-full flex-col gap-1 rounded-xl border border-purple-500/50 bg-purple-950/30 p-3 text-left shadow-sm ring-1 ring-purple-500/30"
            } else {
                "group flex w-full flex-col gap-1 rounded-xl border border-zinc-800/80 bg-zinc-950/40 p-3 text-left text-zinc-400 hover:border-zinc-700 hover:bg-zinc-900/60"
            };
            let el = rsx! {
                button {
                    key: "session-{title}",
                    class: btn_class,
                    title: "{title}",
                    name: "session-{title}",
                    aria_label: "会话 {title}",
                    onclick: move |_| {
                        let chat_name = title.clone();
                        spawn(async move {
                            open_chat(chat_name).await;
                        });
                    },
                    "{title}"
                }
            };
            el
        })
        .collect();
    rsx! {
        div { class: "flex flex-col gap-2",
            button {
                class: "flex items-center justify-center gap-1 rounded-lg bg-purple-600/80 px-2.5 py-1 text-[11px] font-semibold text-white hover:bg-purple-600 transition-colors",
                title: "新对话",
                name: "btn-new-chat",
                onclick: move |_| {
                    let file_name = STATE
                        .with(|state| state.character.as_ref().map(|(f, _)| f.clone()));
                    if let Some(file_name) = file_name {
                        spawn(async move {
                            tavern_state::select_character(file_name).await;
                        });
                    }
                },
                "+ 新对话"
            }
            div { class: "flex flex-col gap-2",
                { session_items.iter() }
            }
        }
    }
}

/// prompt 导航面板：垂直横线导航（每条 = 一次用户 prompt），悬停预览、
/// 点击定位。高亮状态由中央区 `active_prompt` 信号驱动；悬停预览序号用
/// 本地可写信号承接（调用方只读传值）。
#[component]
pub fn PromptPanel(
    /// 当前滚动到的消息索引。
    #[props(into)]
    active_prompt: ReadSignal<usize>,
    /// 初始悬停预览的 prompt 序号（面板内部自管理悬停状态）。
    #[props(into)]
    hovered_prompt: Option<usize>,
) -> Element {
    let prompts = use_memo(move || {
        STATE
            .read()
            .messages
            .iter()
            .enumerate()
            .filter_map(|(i, m)| {
                let (content, _, _, _) = crate::msg_display(m);
                if m.is_user {
                    Some((i, m.name.clone(), content))
                } else {
                    None
                }
            })
            .collect()
    });

    // 悬停预览序号只在下方行闭包的 hp 副本里写入，本体只读不需要 mut
    let hovered = use_signal(|| hovered_prompt);

    let active_prompt_idx = {
        let active_msg = active_prompt();
        let prompts_v: Vec<(usize, String, String)> = prompts();
        prompts_v
            .iter()
            .rposition(|(mi, _, _)| *mi <= active_msg)
            .unwrap_or(0)
    };

    if prompts.is_empty() {
        return rsx! { div { class: "text-[11px] text-zinc-600", "（暂无 prompt）" } };
    }

    let prompts_v: Vec<(usize, String, String)> = prompts();
    let prompts_c = prompts();
    let hovered_c = hovered;

    // 每个 prompt 行独立构建为 Element，避免外层 rsx 里 for + 嵌套 rsx! 的解析坑
    let prompt_rows: Vec<Element> = prompts_v
        .iter()
        .enumerate()
        .map(|(pi, (midx, name, content))| {
            let is_active = active_prompt_idx == pi;
            let name_c = name.clone();
            let content_c = content.clone();
            let midx_c = *midx;
            let tt = format!("{}: {}", name_c, content_c);
            let label = format!("{}", pi + 1);
            let mut hp = hovered_c;
            rsx! {
                div {
                    key: "prompt-nav-{pi}",
                    class: if is_active {
                        "h-[3px] w-6 cursor-pointer rounded-full bg-purple-400 shadow-[0_0_8px] shadow-purple-500/70 transition-all"
                    } else {
                        "h-[3px] w-5 cursor-pointer rounded-full bg-zinc-600 hover:bg-zinc-300 hover:w-6 transition-all"
                    },
                    "data-testid": format!("prompt-nav-{}", pi),
                    title: "{tt}",
                    aria_label: "prompt {label}",
                    onmouseenter: move |_| hp.set(Some(pi)),
                    onmouseleave: move |_| hp.set(None),
                    onclick: move |_| {
                        dioxus::document::eval(&format!("document.getElementById('story-node-{midx_c}')?.scrollIntoView({{ behavior: 'smooth', block: 'start' }});"));
                    },
                }
            }
        })
        .collect();

    let hover_preview = hovered()
        .and_then(|hi| {
            prompts_c
                .get(hi)
                .map(|(_midx, name, content)| (name.clone(), content.clone()))
        })
        .map(|(n, c)| format!("{n}: {c}"))
        .unwrap_or_default();

    rsx! {
        div { class: "flex flex-col gap-1.5",
            { prompt_rows.iter() }
            if !hover_preview.is_empty() {
                div {
                    class: "pointer-events-none mt-1 rounded-lg border border-zinc-700/60 bg-zinc-900/90 p-2 text-[11px] leading-5 text-zinc-200",
                    "{hover_preview}"
                }
            }
        }
    }
}

/// 模型选择面板：轮次指示 + 当前模型切换（原 composer hover 下拉）。
#[component]
pub fn ModelPanel(
    /// 模型下拉展开开关（只读传值，面板内取写引用）。
    #[props(into)]
    model_dropdown_open: Signal<bool>,
) -> Element {
    let mut model_dropdown_open = model_dropdown_open;
    let current_model =
        use_memo(move || STATE.with(|s| s.model.clone().unwrap_or_else(|| "未设置".to_string())));
    let models = use_memo(move || {
        let mut v = vec!["agnes-2.5-flash".to_string()];
        if let Some(m) = STATE.with(|s| s.model.clone())
            && !v.iter().any(|x| x == &m)
        {
            v.push(m);
        }
        v
    });

    let models_v: Vec<String> = models();
    let current_model_v = current_model();

    // 下拉项独立构建，避免外层 rsx 里 for + 嵌套 rsx! 的解析坑
    let model_items: Vec<Element> = models_v
        .iter()
        .map(|model_name| {
            let model_name = model_name.clone();
            rsx! {
                button {
                    key: "model-{model_name}",
                    class: "flex w-full items-center justify-between rounded-lg px-2.5 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100",
                    name: "model-{model_name}",
                    onclick: move |_| {
                        STATE.with_mut(|s| {
                            s.model = Some(model_name.clone());
                        });
                        model_dropdown_open.set(false);
                    },
                    span { "{model_name}" }
                    if current_model_v == model_name {
                        span { class: "text-[10px] text-emerald-400", "✓" }
                    }
                }
            }
        })
        .collect();

    rsx! {
        div { class: "flex flex-col gap-2",
            div { class: "flex items-center gap-1 rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-0.5 text-zinc-400 text-xs",
                button { class: "hover:text-zinc-200 px-0.5", "‹" }
                span { class: "text-[10px] font-medium tabular-nums text-zinc-200", "第 1 轮 · 共 3 轮" }
                button { class: "hover:text-zinc-200 px-0.5", "›" }
            }
            div { class: "relative flex flex-col gap-1",
                button {
                    class: "flex items-center gap-1.5 rounded-full border border-purple-500/40 bg-zinc-950/80 px-3 py-1 text-xs font-semibold text-purple-200 shadow-sm transition-all hover:border-purple-400",
                    name: "btn-model-toggle",
                    aria_label: "切换模型",
                    onclick: move |_| model_dropdown_open.set(!model_dropdown_open()),
                    span { "⚡" }
                    span { "{current_model()}" }
                    span { class: "text-[10px] text-zinc-400", "⌵" }
                }
                if model_dropdown_open() {
                    div {
                        class: "flex flex-col gap-0.5 rounded-xl border border-zinc-800 bg-zinc-900 p-1",
                        { model_items.iter() }
                    }
                }
            }
        }
    }
}
