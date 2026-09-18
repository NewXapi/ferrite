//! 节点检视器抽屉：`NodeInspector` 按节点类型分发到 `GroupInspect` /
//! `AliasInspect` / `DispatchInspect`，底层输入控件 `BoundArea` /
//! `BoundField` / `InspectList` 等。

use dioxus::prelude::*;

use super::data::*;
use super::drawer::DrawerHeader;
use super::physics::{accent_color, node_title_from_store};
use crate::drawer_write::{
    DrawerNotice, DrawerNoticeBar, delete_channel, delete_group, find_channel_by_name,
    find_group_by_name, update_channel, update_group_display,
};
use crate::state::EntityStore;

/// 侧边抽屉：左键点节点后就地编辑该实体。
///
/// `absolute` 覆盖画布右侧，画布尺寸恒定，开合不引起重排。
/// 三层的编辑内容不同：
/// - 分组：名字（可改）
/// - 模型别名：名字（可改）
/// - 调度模型：模型名**只读**（来自上游，改了就路由不到），
///   附带展示所属渠道的 URL/Key，渠道本身在设置页改
#[component]
pub fn NodeInspector(
    node: NodeKey,
    on_tab: EventHandler<DrawerTab>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let title = node_title_from_store(node);
    let kind_label = match node {
        NodeKey::Group(_) => LBL_GROUP,
        NodeKey::Mapping(_) => LBL_ALIAS,
        NodeKey::Dispatch(_) => LBL_DISPATCH,
    };
    let accent = accent_color(node);
    // 渲染期捕获 store（Copy）：事件闭包 / spawn 内禁止 use_context——
    // hook 只能在组件渲染期调用，事件回调里调会 panic 或破坏 hook 序。
    let store = use_context::<EntityStore>();
    // 删除接真实后端：分组/渠道走 delete_group/delete_channel（Dialog 确认）；
    // 别名无删除端点（models 域删除按 UUID key 走 delete_model_alias_api，
    // 此处只有名字定位不到）→ 锁读不丢写。
    let can_delete = !matches!(node, NodeKey::Mapping(_));
    let del_label = if matches!(node, NodeKey::Group(_)) {
        "删除分组"
    } else {
        "删除渠道"
    };
    let mut confirming = use_signal(|| false);
    // mut：rsx 里 DrawerNoticeBar 的 on_clear 要 notice.set(...)；闭包按 Copy 捕获
    let mut notice = use_signal(|| DrawerNotice::Idle);
    // 闭包按 move 捕获，先 clone 出删除用的名字，title 本体留给 DrawerHeader。
    let del_node_name = title.clone();

    let do_delete = move |_| {
        confirming.set(false);
        let mut ns = notice;
        let target = node;
        // 节点名与调度节点所属渠道名都在渲染期取好，spawn 内只做网络调用。
        let node_name = del_node_name.clone();
        let ch_name = match target {
            NodeKey::Dispatch(i) => {
                let view = GraphView::from_store(&store);
                view.dispatch
                    .get(i)
                    .and_then(|(ci, _)| view.channels.get(*ci).cloned())
                    .unwrap_or_default()
            }
            _ => String::new(),
        };
        spawn(async move {
            ns.set(DrawerNotice::Busy);
            let res: Result<(), String> = match target {
                NodeKey::Group(_) => match find_group_by_name(&node_name).await {
                    Ok(Some(g)) => delete_group(&g.key).await,
                    Ok(None) => Err(format!("分组「{node_name}」不存在于后端（可能已被删除）")),
                    Err(e) => Err(e),
                },
                NodeKey::Dispatch(_) => match find_channel_by_name(&ch_name).await {
                    Ok(Some(c)) => delete_channel(&c.key).await,
                    Ok(None) => Err(format!("渠道「{ch_name}」不存在于后端（可能已被删除）")),
                    Err(e) => Err(e),
                },
                _ => Ok(()),
            };
            match res {
                Ok(()) => {
                    ns.set(DrawerNotice::Ok);
                    bump_topo_refresh();
                }
                Err(e) => ns.set(DrawerNotice::Err(format!("删除失败：{e}"))),
            }
        });
    };

    rsx! {
        aside { class: "absolute inset-y-0 right-0 z-20 flex w-full flex-col border-l border-zinc-800 bg-zinc-900/97 backdrop-blur sm:w-[320px]",
            // 节点检视也有页签 —— 方便切到设置/导入
            DrawerHeader {
                tab: DrawerTab::Node,
                title: title.clone(),
                subtitle: kind_label.to_string(),
                on_tab: move |t: DrawerTab| on_tab.call(t),
                on_close: on_close,
            }
            // 类型色点行：补上视觉线索，不占正式空间
            div { class: "shrink-0 border-b border-zinc-800 px-3 py-1.5",
                span { class: "h-2 w-2 rounded-full", style: "background: {accent}" }
            }
            // 删除写操作的结果反馈（成功/失败/进行中），紧跟头部不遮字段
            DrawerNoticeBar { notice, on_clear: move |_| notice.set(DrawerNotice::Idle) }
            // 主体。inspector 带 key：焦点态内点其他节点只换 inspect 不换树，
            // 不重挂的话 use_signal 初始值（编辑框内容）会串到下一个节点。
            div { class: "min-h-0 flex-1 space-y-3 overflow-y-auto scroll-subtle p-3",
                match node {
                    NodeKey::Group(i) => rsx! { GroupInspect { key: "{i}", index: i } },
                    NodeKey::Mapping(i) => rsx! { AliasInspect { key: "{i}", index: i } },
                    NodeKey::Dispatch(i) => rsx! { DispatchInspect { key: "{i}", index: i } },
                }
            }
            // 底部操作条
            div { class: "flex shrink-0 items-center gap-2 border-t border-zinc-800 px-3 py-2",
                button {
                    class: "rounded-md border border-zinc-800 px-2.5 py-1 text-xs text-zinc-400 hover:border-red-700 hover:text-red-400",
                    disabled: !can_delete,
                    title: if can_delete { del_label } else { "别名无删除端点（锁读）" },
                    onclick: move |_| confirming.set(true),
                    "删除"
                }
                span { class: "flex-1" }
                button {
                    class: "rounded-md border border-zinc-100 bg-zinc-100 px-2.5 py-1 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    disabled: !can_delete,
                    title: if can_delete { "保存按钮在各编辑卡片内（展示名/渠道）" } else { "别名锁读，无写路径" },
                    "保存"
                }
            }
            if confirming() {
                ui::Dialog {
                    title: del_label.to_string(),
                    open: true,
                    on_confirm: do_delete,
                    on_cancel: move |_| confirming.set(false),
                    p { "确认删除？该操作直接生效于后端，不可撤销。" }
                }
            }
        }
    }
}

#[component]
pub fn GroupInspect(index: usize) -> Element {
    let store = use_context::<EntityStore>();
    let row = store.groups.read().get(index).cloned();
    // 别名列表 = 当前数据驱动边里 Group(index) 的出边映射
    let view = GraphView::from_store(&store);
    let edges = edges_of(&view);
    let aliases: Vec<String> = edges
        .iter()
        .filter_map(|&(u, l)| {
            (u == NodeKey::Group(index) && matches!(l, NodeKey::Mapping(_))).then_some(l)
        })
        .map(|l| view_title(&view, l))
        .collect();
    let Some(r) = row else {
        return rsx! { p { class: "text-xs text-zinc-600", "该分组不存在" } };
    };
    // 分组改名/删除走真实后端（drawer_write）：分组名后端无更新路径
    // （UpdateGroupRequest 只有 ratio/model_whitelist/remark/status），
    // 锁读不静默丢写；展示名写 remark 列；删除先经 Dialog 确认。
    let gname = r.name.clone();
    // 展示名用本地编辑态：改完点「保存展示名」一次性提交。不走每击键
    // 即时写——那会把一次保存放大成 列表×2 + PUT 三发请求且并发乱序。
    let mut display_sig = use_signal(|| r.display.clone());
    let mut group_notice = use_signal(|| DrawerNotice::Idle);
    // 闭包按 move 捕获，先 clone 出保存用的名字，gname 本体留给锁读字段。
    let save_name = gname.clone();

    let save_display = move |_| {
        let v = display_sig.peek().trim().to_string();
        let (mut ns, name) = (group_notice, save_name.clone());
        spawn(async move {
            ns.set(DrawerNotice::Busy);
            match find_group_by_name(&name).await {
                Ok(Some(g)) => match update_group_display(&g, &v).await {
                    Ok(_) => {
                        ns.set(DrawerNotice::Ok);
                        bump_topo_refresh();
                    }
                    Err(e) => ns.set(DrawerNotice::Err(format!("保存失败：{e}"))),
                },
                Ok(None) => ns.set(DrawerNotice::Err(format!(
                    "分组「{name}」不存在于后端（可能已被删除）"
                ))),
                Err(e) => ns.set(DrawerNotice::Err(format!("保存失败：{e}"))),
            }
        });
    };

    rsx! {
        DrawerNoticeBar { notice: group_notice, on_clear: move |_| group_notice.set(DrawerNotice::Idle) }
        BoundField {
            label: "分组名（锁读：后端无改名路径）",
            value: gname.clone(),
            placeholder: "vip",
            on_change: move |_| (),
        }
        div { class: "space-y-1",
            BoundField {
                label: FIELD_DISPLAY,
                value: display_sig,
                placeholder: "默认分组",
                on_change: move |v: String| display_sig.set(v),
            }
            button {
                class: "w-full rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                onclick: save_display,
                "保存展示名"
            }
        }
        InspectList { title: "包含的模型别名", items: aliases, empty: "拖端口连线以加入别名" }
    }
}

#[component]
pub fn AliasInspect(index: usize) -> Element {
    let store = use_context::<EntityStore>();
    let row = store.aliases.read().get(index).cloned();
    let view = GraphView::from_store(&store);
    let edges = edges_of(&view);
    // 所属分组 = Group(i)→Mapping(index) 的边
    let groups: Vec<String> = edges
        .iter()
        .filter_map(|&(u, l)| {
            (l == NodeKey::Mapping(index) && matches!(u, NodeKey::Group(_))).then_some(u)
        })
        .map(|u| view_title(&view, u))
        .collect();
    // 路由到的调度模型 = Mapping(index)→Dispatch(ci) 的边
    let dispatch: Vec<String> = edges
        .iter()
        .filter_map(|&(u, l)| {
            (u == NodeKey::Mapping(index) && matches!(l, NodeKey::Dispatch(_))).then_some(l)
        })
        .map(|l| view_title(&view, l))
        .collect();
    let Some(r) = row else {
        return rsx! { p { class: "text-xs text-zinc-600", "该别名不存在" } };
    };
    // 别名写路径（#175/#182）按 UUID key 走 /api/models/{key}，本 inspector
    // 只有名字定位不到 key → 锁读不丢写（与 b37a407 的锁读修复同型）。
    rsx! {
        BoundField {
            label: "别名（锁读：写路径按 UUID key，此处定位不到）",
            value: r.alias.clone(),
            placeholder: "gpt-4o",
            on_change: move |_| (),
        }
        BoundField {
            label: "展示名（锁读：后端 models 域无对应列）",
            value: r.display.clone(),
            placeholder: "GPT-4o",
            on_change: move |_| (),
        }
        InspectList { title: "所属分组", items: groups, empty: "未加入任何分组" }
        InspectList { title: "路由到的调度模型", items: dispatch, empty: "未连接调度模型" }
    }
}

#[component]
pub fn DispatchInspect(index: usize) -> Element {
    let store = use_context::<EntityStore>();
    let view = GraphView::from_store(&store);
    let edges = edges_of(&view);
    let (ci, model_name) = view
        .dispatch
        .get(index)
        .cloned()
        .unwrap_or((0, String::new()));
    let row = store.channels.read().get(ci).cloned();
    // 被哪些别名路由 = Mapping(i)→Dispatch(index) 的边
    let aliases: Vec<String> = edges
        .iter()
        .filter_map(|&(u, l)| {
            (l == NodeKey::Dispatch(index) && matches!(u, NodeKey::Mapping(_))).then_some(u)
        })
        .map(|u| view_title(&view, u))
        .collect();

    // 渠道编辑接真实后端（update_channel 最小 diff）：name/url 直改，
    // keys 仅当用户重输时携带（留空 = 保留现有密钥，掩码值绝不回传）。
    // 定位用渲染期 store 行的原名（`orig_name`），不是编辑框现值——
    // 用户改过名字后按新名查后端必然落空。
    let orig_name = row.as_ref().map(|c| c.name.clone()).unwrap_or_default();
    let chan_name = orig_name.clone();
    let chan_url = row.as_ref().map(|c| c.url.clone()).unwrap_or_default();
    let mut ch_name = use_signal(|| chan_name.clone());
    let mut ch_url = use_signal(|| chan_url.clone());
    let mut ch_keys = use_signal(String::new);
    let mut ch_notice = use_signal(|| DrawerNotice::Idle);
    // store.channels 的写句柄（Signal::write 需 &mut；EntityStore 是 Copy，
    // 单独拷出声明 mut，store 本体只读使用）
    let mut channels_sig = store.channels;

    let save_channel = move |_| {
        let n = ch_name.peek().trim().to_string();
        let u = ch_url.peek().trim().to_string();
        let kraw = ch_keys.peek().trim().to_string();
        // 用户未重输 keys → None（请求体缺席 keys 字段 = 保留现值）
        let kvec: Vec<String> = kraw
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let new_keys = (!kvec.is_empty()).then_some(kvec.clone());
        let (mut ns, lookup, curl) = (ch_notice, orig_name.clone(), u);
        spawn(async move {
            ns.set(DrawerNotice::Busy);
            match find_channel_by_name(&lookup).await {
                Ok(Some(c)) => match update_channel(&c.key, &n, &curl, new_keys.clone()).await {
                    Ok(_) => {
                        ns.set(DrawerNotice::Ok);
                        ch_keys.set(String::new()); // 已提交，编辑区清空
                        bump_topo_refresh();
                        // 本地 store 行同步改名/URL/keys，本会话后续
                        // 按名定位与设置页显示不与后端脱节。
                        if let Some(r) = channels_sig.write().get_mut(ci)
                            && r.name == lookup
                        {
                            r.name = n;
                            r.url = curl;
                            if let Some(k) = new_keys {
                                r.keys = k.join("\n");
                            }
                        }
                    }
                    Err(e) => ns.set(DrawerNotice::Err(format!("保存失败：{e}"))),
                },
                Ok(None) => ns.set(DrawerNotice::Err(format!(
                    "渠道「{lookup}」不存在于后端（可能已被删除）"
                ))),
                Err(e) => ns.set(DrawerNotice::Err(format!("保存失败：{e}"))),
            }
        });
    };

    rsx! {
        DrawerNoticeBar { notice: ch_notice, on_clear: move |_| ch_notice.set(DrawerNotice::Idle) }
        div { class: "space-y-1",
            span { class: "text-[11px] text-zinc-500", "模型名（只读，来自上游）" }
            div { class: "rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 font-mono text-sm text-zinc-300", "{model_name}" }
        }
        if row.is_some() {
            div { class: "space-y-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3",
                span { class: "text-[11px] uppercase tracking-wider text-zinc-600", "所属渠道" }
                BoundField {
                    label: "渠道名称",
                    value: ch_name,
                    placeholder: EXAMPLE_CHANNEL,
                    on_change: move |v: String| ch_name.set(v),
                }
                BoundField {
                    label: "Base URL",
                    value: ch_url,
                    placeholder: "https://…",
                    on_change: move |v: String| ch_url.set(v),
                }
                BoundArea {
                    label: "API Key（多 key 一行一个；留空 = 保留现有密钥）",
                    value: ch_keys,
                    placeholder: "sk-…\nsk-…",
                    on_change: move |v: String| ch_keys.set(v),
                }
                button {
                    class: "w-full rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    onclick: save_channel,
                    "保存渠道"
                }
            }
        }
        InspectList { title: "被哪些别名路由", items: aliases, empty: "未被任何别名引用" }
    }
}

/// 多行受控输入：给 Key 这种可包含多行的字段用。
#[component]
pub fn BoundArea(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            textarea {
                class: "min-h-[72px] w-full resize-y rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 font-mono text-xs text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |e| on_change.call(e.value()),
            }
        }
    }
}

/// 受控输入：直接写回 store 对应字段，随打随存。
#[component]
fn BoundField(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |e| on_change.call(e.value()),
            }
        }
    }
}

#[component]
fn CredRow(label: &'static str, value: &'static str) -> Element {
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "w-8 shrink-0 text-[11px] text-zinc-600", "{label}" }
            span { class: "truncate font-mono text-[11px] text-zinc-400", "{value}" }
        }
    }
}

#[component]
fn InspectField(label: &'static str, value: Signal<String>, placeholder: &'static str) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value.read()}",
                placeholder: "{placeholder}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

#[component]
fn InspectList(title: &'static str, items: Vec<String>, empty: &'static str) -> Element {
    rsx! {
        div { class: "space-y-1.5",
            span { class: "text-[11px] text-zinc-500", "{title}" }
            if items.is_empty() {
                p { class: "text-[11px] text-zinc-600", "{empty}" }
            } else {
                div { class: "flex flex-wrap gap-1.5",
                    for it in items.iter() {
                        span { class: "inline-flex items-center gap-1 rounded-full border border-zinc-700 bg-zinc-900 px-2 py-0.5 text-[11px] text-zinc-300",
                            "{it}"
                            button { class: "text-zinc-600 hover:text-red-400", "✕" }
                        }
                    }
                }
            }
        }
    }
}
