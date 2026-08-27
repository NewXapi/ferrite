//! 管理工作区：拓扑画布 + 右侧检视抽屉，一屏内既看关系又改实体。
//!
//! 布局原则：
//! - 画布始终填满，抽屉 `absolute` 覆盖右侧，画布尺寸恒定（不因开合位移）。
//! - 图层芯片控制可见性 + 当前活动层（决定新增建什么）。
//! - 选择条只在有选中时出现，浮在底部，不挤压画布。
//! - 抽屉按选中数量变形：0 概览 / 1 详编 / N 同类批量 / N 混类通用。
//! - 抽屉可「展开」为占满面板的重编辑视图，响应式 1 / 3 / 5 栏。
//!
//! 数据 mock。

use std::collections::HashSet;

use dioxus::prelude::*;

use crate::network::{ManageState, NetworkPanel, NodeKey, NodeKind, node_title, subtitle};

#[component]
pub fn ManageWorkspace() -> Element {
    let state = use_context::<ManageState>();
    let mut drawer_open = use_signal(|| true);
    let mut expanded_editor = use_signal(|| false);

    let selection = state.selected.read().clone();
    let count = selection.len();

    // 左手可达的快捷键：1/2/3 切活动层，Q/W/E 切图层可见性，
    // A 全选活动层，Esc 清空，Tab 开合抽屉，F 展开重编辑。
    let onkeydown = move |e: KeyboardEvent| {
        let mut visible = state.visible;
        let mut active = state.active_kind;
        let mut selected = state.selected;
        match e.key() {
            Key::Character(c) => match c.as_str() {
                "1" => active.set(NodeKind::Group),
                "2" => active.set(NodeKind::Mapping),
                "3" => active.set(NodeKind::Channel),
                "q" | "Q" => toggle_kind(&mut visible, NodeKind::Group),
                "w" | "W" => toggle_kind(&mut visible, NodeKind::Mapping),
                "e" | "E" => toggle_kind(&mut visible, NodeKind::Channel),
                "a" | "A" => {
                    e.prevent_default();
                    let kind = *active.peek();
                    selected.set(all_of_kind(kind));
                }
                "f" | "F" => {
                    let next = !*expanded_editor.peek();
                    expanded_editor.set(next);
                    if next {
                        drawer_open.set(true);
                    }
                }
                _ => {}
            },
            Key::Escape => {
                selected.set(HashSet::new());
                let mut menu = state.context_menu;
                menu.set(None);
            }
            Key::Tab => {
                e.prevent_default();
                let next = !*drawer_open.peek();
                drawer_open.set(next);
            }
            Key::Delete | Key::Backspace => {
                // mock：真实实现走后端删除 + 级联检查
            }
            _ => {}
        }
    };

    rsx! {
        div {
            class: "relative flex h-full min-h-0 flex-col outline-none",
            tabindex: "0",
            onkeydown: onkeydown,
            onclick: move |_| {
                let mut menu = state.context_menu;
                if menu.peek().is_some() {
                    menu.set(None);
                }
            },

            LayerBar { drawer_open: drawer_open(), on_toggle_drawer: move |_| { let n = !drawer_open(); drawer_open.set(n); } }

            // 画布 + 抽屉：relative 容器，抽屉覆盖不挤压
            div { class: "relative min-h-0 flex-1",
                if expanded_editor() {
                    // 展开态：整块面板换成重编辑视图（1 / 3 / 5 栏）
                    ExpandedEditor { on_collapse: move |_| expanded_editor.set(false) }
                } else {
                    NetworkPanel {}
                    if drawer_open() {
                        Inspector {
                            on_expand: move |_| expanded_editor.set(true),
                            on_close: move |_| drawer_open.set(false),
                        }
                    }
                    if count > 0 {
                        SelectionBar { drawer_open: drawer_open() }
                    }
                }
            }

            if let Some((x, y, key)) = *state.context_menu.read() {
                ContextMenu { x: x, y: y, node: key }
            }
        }
    }
}

fn toggle_kind(visible: &mut Signal<HashSet<NodeKind>>, kind: NodeKind) {
    let mut set = visible.write();
    if !set.remove(&kind) {
        set.insert(kind);
    }
}

/// mock：真实实现应从可见层派生，而不是硬编码数量。
fn all_of_kind(kind: NodeKind) -> HashSet<NodeKey> {
    match kind {
        NodeKind::Group => (0..4).map(NodeKey::Group).collect(),
        NodeKind::Mapping => (0..4).map(NodeKey::Mapping).collect(),
        NodeKind::Channel => (0..6).map(NodeKey::Channel).collect(),
    }
}

// ============ 顶部图层条 ============

/// 图层芯片：眼睛控可见性，芯片本体设为活动层（决定新增建什么）。
#[component]
fn LayerBar(drawer_open: bool, on_toggle_drawer: EventHandler<MouseEvent>) -> Element {
    let state = use_context::<ManageState>();
    let visible = state.visible.read().clone();
    let active = *state.active_kind.read();

    rsx! {
        div { class: "mb-3 flex shrink-0 flex-wrap items-center gap-2",
            span { class: "text-[11px] uppercase tracking-wider text-zinc-600", "图层" }
            for (i, kind) in NodeKind::ALL.into_iter().enumerate() {
                {
                    let shown = visible.contains(&kind);
                    let is_active = active == kind;
                    let tone = if is_active {
                        "border-zinc-100 bg-zinc-100 text-zinc-900"
                    } else if shown {
                        "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                    } else {
                        "border-zinc-800 bg-zinc-950 text-zinc-600 hover:border-zinc-700"
                    };
                    let mut visible_sig = state.visible;
                    let mut active_sig = state.active_kind;
                    rsx! {
                        span { class: "inline-flex items-center overflow-hidden rounded-full border {tone}",
                            button {
                                class: "px-2 py-1 text-xs transition-opacity",
                                class: if shown { "opacity-100" } else { "opacity-40" },
                                title: "切换可见性（{['Q','W','E'][i]}）",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    toggle_kind(&mut visible_sig, kind);
                                },
                                if shown { "◉" } else { "○" }
                            }
                            button {
                                class: "pr-3 text-xs font-medium",
                                title: "设为活动层（{i + 1}）",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    active_sig.set(kind);
                                },
                                "{kind.label()}"
                            }
                        }
                    }
                }
            }
            span { class: "ml-auto text-[11px] text-zinc-600", "1/2/3 活动层 · Q/W/E 显隐 · A 全选 · Tab 抽屉 · F 展开" }
            button {
                class: "rounded-md border border-zinc-800 bg-zinc-900 px-2.5 py-1 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
                onclick: move |e| on_toggle_drawer.call(e),
                if drawer_open { "隐藏检视" } else { "显示检视" }
            }
        }
    }
}

// ============ 底部选择条 ============

/// 仅在有选中时出现，浮在画布底部；批量操作按选中构成分派。
#[component]
fn SelectionBar(drawer_open: bool) -> Element {
    let state = use_context::<ManageState>();
    let selection = state.selected.read().clone();
    let mut counts = [0usize; 3];
    for k in selection.iter() {
        let idx = match k.kind() {
            NodeKind::Group => 0,
            NodeKind::Mapping => 1,
            NodeKind::Channel => 2,
        };
        counts[idx] += 1;
    }
    let parts: Vec<String> = NodeKind::ALL
        .into_iter()
        .enumerate()
        .filter(|(i, _)| counts[*i] > 0)
        .map(|(i, k)| format!("{} {}", counts[i], k.label()))
        .collect();
    let summary = parts.join(" · ");
    let total = selection.len();
    let homogeneous = parts.len() == 1;

    rsx! {
        div {
            // 居中于「画布可见区」而非整个容器：桌面抽屉在右，手机抽屉在下，
            // 两种情况都要让条避开，否则会被压窄换行或被盖住。
            class: "pointer-events-none absolute inset-x-0 z-20 flex justify-center px-3",
            class: if drawer_open { "bottom-[54%] sm:bottom-3 sm:pr-[352px]" } else { "bottom-3" },
            div { class: "pointer-events-auto flex max-w-full items-center gap-2 overflow-x-auto whitespace-nowrap rounded-full border border-zinc-700 bg-zinc-900/95 px-3 py-1.5 shadow-lg shadow-black/40 backdrop-blur",
                span { class: "text-xs text-zinc-300", "已选 {total}" }
                span { class: "text-[11px] text-zinc-500", "{summary}" }
                span { class: "mx-1 h-4 w-px bg-zinc-700" }
                BarBtn { label: "启用" }
                BarBtn { label: "禁用" }
                if homogeneous {
                    BarBtn { label: "批量编辑" }
                }
                BarBtn { label: "整理" }
                BarBtn { label: "删除", danger: true }
                button {
                    class: "ml-1 rounded-full px-2 text-xs text-zinc-500 hover:text-zinc-200",
                    title: "清空选择（Esc）",
                    onclick: move |_| {
                        let mut selected = state.selected;
                        selected.set(HashSet::new());
                    },
                    "✕"
                }
            }
        }
    }
}

#[component]
fn BarBtn(label: &'static str, #[props(default = false)] danger: bool) -> Element {
    let tone = if danger {
        "text-zinc-400 hover:text-red-400"
    } else {
        "text-zinc-300 hover:text-zinc-100"
    };
    rsx! {
        button { class: "rounded-md px-2 py-0.5 text-xs transition-colors {tone}", "{label}" }
    }
}

// ============ 右键上下文菜单 ============

#[component]
fn ContextMenu(x: f64, y: f64, node: Option<NodeKey>) -> Element {
    let state = use_context::<ManageState>();
    let active = *state.active_kind.read();
    let items: Vec<(&'static str, bool)> = match node {
        Some(_) => vec![
            ("编辑详情", false),
            ("管理关联", false),
            ("聚焦邻居", false),
            ("复制", false),
            ("从选择移除", false),
            ("删除节点", true),
        ],
        None => vec![
            ("在此新增节点", false),
            ("全选活动层", false),
            ("整理视图", false),
            ("适配视图", false),
            ("清空选择", false),
        ],
    };
    let title = match node {
        Some(k) => node_title(k),
        None => format!("新增到「{}」", active.label()),
    };

    rsx! {
        div {
            class: "fixed z-50 min-w-[152px] overflow-hidden rounded-lg border border-zinc-700 bg-zinc-900/98 py-1 shadow-xl shadow-black/50 backdrop-blur",
            style: "left: {x}px; top: {y}px",
            onclick: move |e| e.stop_propagation(),
            div { class: "truncate border-b border-zinc-800 px-3 py-1.5 text-[11px] text-zinc-500", "{title}" }
            for (label, danger) in items {
                button {
                    class: "block w-full px-3 py-1.5 text-left text-xs transition-colors",
                    class: if danger { "text-zinc-400 hover:bg-zinc-800 hover:text-red-400" } else { "text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100" },
                    onclick: move |_| {
                        let mut menu = state.context_menu;
                        menu.set(None);
                    },
                    "{label}"
                }
            }
        }
    }
}

// ============ 右侧检视抽屉 ============

/// 覆盖式抽屉：`absolute` 定位，画布不重排。按选中数量变形。
#[component]
fn Inspector(on_expand: EventHandler<MouseEvent>, on_close: EventHandler<MouseEvent>) -> Element {
    let state = use_context::<ManageState>();
    let selection = state.selected.read().clone();
    let count = selection.len();

    let kinds: HashSet<NodeKind> = selection.iter().map(|k| k.kind()).collect();
    let single = if count == 1 {
        selection.iter().next().copied()
    } else {
        None
    };

    let heading = match (count, kinds.len()) {
        (0, _) => "图层概览".to_string(),
        (1, _) => node_title(single.unwrap()),
        (n, 1) => format!("{} 个{}", n, kinds.iter().next().unwrap().label()),
        (n, _) => format!("{n} 个节点（混合类型）"),
    };

    rsx! {
        // 手机：底部半高抽屉，上半屏留给画布（地图编辑器不能被表单全遮）。
        // sm 起：右侧固定 340px 竖抽屉。
        aside { class: "absolute inset-x-0 bottom-0 z-20 flex max-h-[52%] flex-col rounded-t-xl border-t border-zinc-800 bg-zinc-900/97 backdrop-blur sm:inset-x-auto sm:inset-y-0 sm:right-0 sm:max-h-none sm:w-[340px] sm:rounded-none sm:border-l sm:border-t-0",
            // 抽屉头
            div { class: "flex shrink-0 items-center gap-2 border-b border-zinc-800 px-3 py-2",
                div { class: "min-w-0 flex-1",
                    p { class: "truncate text-sm font-medium text-zinc-100", "{heading}" }
                    if let Some(k) = single {
                        p { class: "truncate text-[11px] text-zinc-500", "{k.kind().label()} · {subtitle(k)}" }
                    }
                }
                button {
                    class: "rounded-md border border-zinc-800 px-2 py-0.5 text-[11px] text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
                    title: "展开为重编辑视图（F）",
                    onclick: move |e| on_expand.call(e),
                    "展开"
                }
                button {
                    class: "rounded-md px-1.5 text-zinc-500 hover:text-zinc-200",
                    title: "收起（Tab）",
                    onclick: move |e| on_close.call(e),
                    "✕"
                }
            }
            // 抽屉体：滚动区
            div { class: "min-h-0 flex-1 overflow-y-auto p-3",
                match (count, kinds.len()) {
                    (0, _) => rsx! { LayerOverview {} },
                    (1, _) => rsx! { SingleForm { node: single.unwrap() } },
                    (_, 1) => rsx! { BatchForm { kind: *kinds.iter().next().unwrap() } },
                    _ => rsx! { MixedForm {} },
                }
            }
            // 抽屉底：固定操作条，不随内容滚动
            div { class: "flex shrink-0 items-center gap-2 border-t border-zinc-800 px-3 py-2",
                if count > 0 {
                    button { class: "rounded-md border border-zinc-800 px-2.5 py-1 text-xs text-zinc-400 hover:border-red-700 hover:text-red-400", "删除" }
                }
                span { class: "flex-1" }
                button { class: "rounded-md border border-zinc-800 bg-zinc-900 px-2.5 py-1 text-xs text-zinc-300 hover:border-zinc-600 hover:text-zinc-100", "重置" }
                button { class: "rounded-md border border-zinc-100 bg-zinc-100 px-2.5 py-1 text-xs font-medium text-zinc-900 hover:bg-zinc-300", "保存" }
            }
        }
    }
}

/// 0 选中：各层节点数 + 快捷新增。
#[component]
fn LayerOverview() -> Element {
    let state = use_context::<ManageState>();
    let visible = state.visible.read().clone();
    let stats = [
        (NodeKind::Group, 4usize, "分组来自倍率配置与渠道声明"),
        (NodeKind::Mapping, 4, "模型由渠道 models 派生，不可凭空建"),
        (NodeKind::Channel, 6, "唯一真实体，需 base_url 与 key"),
    ];

    rsx! {
        div { class: "space-y-3",
            p { class: "text-[11px] leading-relaxed text-zinc-500",
                "未选中节点。左键点选、Ctrl 加选、Shift 框选；右键出菜单。"
            }
            for (kind, n, note) in stats {
                div { class: "rounded-lg border border-zinc-800 bg-zinc-950 p-3",
                    div { class: "flex items-center justify-between",
                        span { class: "text-sm text-zinc-200", "{kind.label()}" }
                        span { class: "text-xs text-zinc-500", "{n}" }
                    }
                    p { class: "mt-1 text-[11px] leading-relaxed text-zinc-600", "{note}" }
                    div { class: "mt-2 flex gap-1.5",
                        if kind == NodeKind::Mapping {
                            span { class: "rounded border border-zinc-800 px-2 py-0.5 text-[11px] text-zinc-600", "不可新增" }
                        } else {
                            button { class: "rounded border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-300 hover:border-zinc-500", "新增" }
                        }
                        if !visible.contains(&kind) {
                            span { class: "rounded border border-zinc-800 px-2 py-0.5 text-[11px] text-zinc-600", "已隐藏" }
                        }
                    }
                }
            }
        }
    }
}

/// 1 选中：按类型给完整表单。
#[component]
fn SingleForm(node: NodeKey) -> Element {
    match node.kind() {
        NodeKind::Group => rsx! {
            FormSection { title: "基础",
                TextRow { label: "分组名", value: node_title(node) }
                TextRow { label: "展示名", value: "默认分组".to_string() }
                SwitchRow { label: "启用", on: true }
            }
            FormSection { title: "关联（可直接增删）",
                ChipList { items: vec!["gpt-4o", "gemini-2.5-pro"], addable: true }
            }
            FormSection { title: "引用",
                ReadRow { label: "关联渠道", value: "3 个".to_string() }
                ReadRow { label: "可用模型", value: "12 个（派生）".to_string() }
            }
        },
        NodeKind::Mapping => rsx! {
            FormSection { title: "标识",
                ReadRow { label: "别名", value: node_title(node) }
                TextRow { label: "展示名", value: "GPT-4o".to_string() }
                SwitchRow { label: "对用户可见", on: true }
            }
            FormSection { title: "展示",
                TextRow { label: "封面 URL", value: "https://cdn…/openai.svg".to_string() }
                TextRow { label: "角标", value: "推荐".to_string() }
            }
            FormSection { title: "关联（可直接增删）",
                ChipList { items: vec!["default", "vip"], addable: true }
            }
        },
        NodeKind::Channel => rsx! {
            FormSection { title: "基础",
                TextRow { label: "渠道名称", value: node_title(node) }
                TextRow { label: "渠道类型", value: "openai".to_string() }
                SwitchRow { label: "启用", on: true }
            }
            FormSection { title: "端点与认证",
                TextRow { label: "Base URL", value: "https://api.openai.com/v1".to_string() }
                TextRow { label: "API Key", value: "sk-****".to_string() }
            }
            FormSection { title: "路由",
                ChipList { items: vec!["default", "vip"], addable: true }
                ReadRow { label: "模型数", value: "3".to_string() }
            }
        },
    }
}

/// N 个同类：只显示可批量的字段。
#[component]
fn BatchForm(kind: NodeKind) -> Element {
    rsx! {
        div { class: "space-y-3",
            p { class: "rounded-md border border-zinc-800 bg-zinc-950 p-2 text-[11px] leading-relaxed text-zinc-500",
                "批量编辑「{kind.label()}」。留空的字段不会被写入，只有填了值的字段会覆盖所有选中项。"
            }
            FormSection { title: "可批量字段",
                match kind {
                    NodeKind::Group => rsx! { SwitchRow { label: "启用", on: true } },
                    NodeKind::Mapping => rsx! { SwitchRow { label: "对用户可见", on: true } },
                    NodeKind::Channel => rsx! {
                        TextRow { label: "渠道类型", value: String::new() }
                        SwitchRow { label: "启用", on: true }
                    },
                }
            }
            FormSection { title: "批量关联",
                ChipList { items: vec![], addable: true }
            }
        }
    }
}

/// N 个混类：只给通用操作。
#[component]
fn MixedForm() -> Element {
    rsx! {
        div { class: "space-y-3",
            p { class: "rounded-md border border-zinc-800 bg-zinc-950 p-2 text-[11px] leading-relaxed text-zinc-500",
                "选中了多种类型的节点。不同类型没有共同字段，只能做通用操作；要批量改字段请只选同一类型。"
            }
            FormSection { title: "通用操作",
                div { class: "flex flex-wrap gap-1.5",
                    button { class: "rounded border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 hover:border-zinc-500", "全部启用" }
                    button { class: "rounded border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 hover:border-zinc-500", "全部禁用" }
                    button { class: "rounded border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 hover:border-zinc-500", "整理布局" }
                    button { class: "rounded border border-zinc-800 px-2 py-1 text-[11px] text-zinc-400 hover:border-red-700 hover:text-red-400", "全部删除" }
                }
            }
        }
    }
}

// ============ 展开态重编辑视图 ============

/// 抽屉放不下时的全宽视图：桌面 5 栏、平板 3 栏、手机 1 栏。
#[component]
fn ExpandedEditor(on_collapse: EventHandler<MouseEvent>) -> Element {
    let state = use_context::<ManageState>();
    let selection = state.selected.read().clone();
    let single = selection.iter().next().copied();
    let heading = match single {
        Some(k) => format!("{} · {}", k.kind().label(), node_title(k)),
        None => "重编辑视图".to_string(),
    };

    rsx! {
        div { class: "flex h-full min-h-0 flex-col rounded-xl border border-zinc-800 bg-zinc-900/60",
            div { class: "flex shrink-0 items-center gap-2 border-b border-zinc-800 px-3 py-2",
                p { class: "min-w-0 flex-1 truncate text-sm font-medium text-zinc-100", "{heading}" }
                button {
                    class: "rounded-md border border-zinc-800 px-2.5 py-1 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
                    title: "回到画布（F）",
                    onclick: move |e| on_collapse.call(e),
                    "回到拓扑"
                }
            }
            // 1 栏（手机）/ 3 栏（平板）/ 5 栏（桌面）
            div { class: "grid min-h-0 flex-1 grid-cols-1 gap-3 overflow-y-auto p-3 md:grid-cols-3 xl:grid-cols-5",
                FormSection { title: "基础",
                    TextRow { label: "名称", value: single.map(node_title).unwrap_or_default() }
                    TextRow { label: "类型", value: "openai".to_string() }
                    SwitchRow { label: "启用", on: true }
                }
                FormSection { title: "端点",
                    TextRow { label: "Base URL", value: "https://api.openai.com/v1".to_string() }
                    TextRow { label: "备用 URL", value: String::new() }
                }
                FormSection { title: "认证",
                    TextRow { label: "API Key", value: "sk-****".to_string() }
                    TextRow { label: "追加 Key", value: String::new() }
                }
                FormSection { title: "路由",
                    ChipList { items: vec!["default", "vip"], addable: true }
                }
                FormSection { title: "模型",
                    ChipList { items: vec!["gpt-4o", "gpt-5", "gpt-4o-mini"], addable: true }
                }
            }
        }
    }
}

// ============ 表单原子 ============

#[component]
fn FormSection(title: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "mb-3 space-y-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3",
            h4 { class: "text-[11px] font-medium uppercase tracking-wider text-zinc-600", "{title}" }
            {children}
        }
    }
}

#[component]
fn TextRow(label: &'static str, value: String) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded border border-zinc-800 bg-zinc-900 px-2 py-1 text-xs text-zinc-200 outline-none focus:border-zinc-500",
                initial_value: "{value}",
                placeholder: "（不修改）",
            }
        }
    }
}

#[component]
fn ReadRow(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex items-baseline justify-between gap-2",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            span { class: "truncate text-xs text-zinc-300", "{value}" }
        }
    }
}

#[component]
fn SwitchRow(label: &'static str, on: bool) -> Element {
    let mut state = use_signal(|| on);
    let track = if state() {
        "bg-zinc-100"
    } else {
        "bg-zinc-800"
    };
    let knob = if state() {
        "translate-x-4"
    } else {
        "translate-x-0"
    };
    rsx! {
        div { class: "flex items-center justify-between gap-2",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            button {
                r#type: "button",
                class: "relative h-4 w-8 shrink-0 rounded-full transition-colors {track}",
                onclick: move |_| { let n = !state(); state.set(n); },
                span { class: "absolute left-0.5 top-0.5 h-3 w-3 rounded-full bg-zinc-900 transition-transform {knob}" }
            }
        }
    }
}

/// 关联芯片：直接在检视器里增删关系，不用回画布拖线。
#[component]
fn ChipList(items: Vec<&'static str>, #[props(default = false)] addable: bool) -> Element {
    rsx! {
        div { class: "flex flex-wrap gap-1.5",
            for it in items.iter() {
                span { class: "inline-flex items-center gap-1 rounded-full border border-zinc-700 bg-zinc-900 px-2 py-0.5 text-[11px] text-zinc-300",
                    "{it}"
                    button { class: "text-zinc-500 hover:text-red-400", "✕" }
                }
            }
            if addable {
                button { class: "rounded-full border border-dashed border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-500 hover:border-zinc-500 hover:text-zinc-300", "＋ 关联" }
            }
        }
    }
}
