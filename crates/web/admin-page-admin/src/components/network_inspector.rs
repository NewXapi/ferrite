//! 节点检视器抽屉：`NodeInspector` 按节点类型分发到 `GroupInspect` /
//! `AliasInspect` / `DispatchInspect`，底层输入控件 `BoundArea` /
//! `BoundField` / `InspectList` 等。

use dioxus::prelude::*;

use crate::components::network_drawer::DrawerHeader;
use crate::components::network_shared::{
    BTN_DELETE, BTN_DELETE_CHANNEL_TITLE, BTN_DELETE_GROUP_TITLE, BTN_SAVE, BTN_SAVE_CHANNEL,
    BTN_SAVE_DISPLAY, EXAMPLE_CHANNEL, FIELD_DISPLAY, LBL_ALIAS, LBL_DISPATCH, LBL_GROUP,
    MSG_ALIAS_ABSENT, MSG_ALIAS_DISPLAY_LOCKED, MSG_ALIAS_LOCKED, MSG_API_KEY_EDIT,
    MSG_CHANNEL_MISSING_PREFIX, MSG_CHANNEL_NAME, MSG_DELETE_CONFIRM, MSG_DELETE_DISABLED_TITLE,
    MSG_GROUP_ABSENT, MSG_GROUP_MISSING_PREFIX, MSG_GROUP_NAME_LOCKED, MSG_INSPECT_ALIASES_EMPTY,
    MSG_INSPECT_ALIASES_TITLE, MSG_INSPECT_DISPATCH_EMPTY, MSG_INSPECT_DISPATCH_TITLE,
    MSG_INSPECT_GROUPS_EMPTY, MSG_INSPECT_GROUPS_TITLE, MSG_INSPECT_ROUTED_EMPTY,
    MSG_INSPECT_ROUTED_TITLE, MSG_MISSING_SUFFIX, MSG_MODEL_NAME_READONLY, MSG_OWNER_CHANNEL,
    MSG_PH_DEFAULT_GROUP, MSG_SAVE_DISABLED_TITLE, MSG_SAVE_FAILED_PREFIX, MSG_SAVE_TITLE,
    MSG_WRITE_FAILED_PREFIX,
};
use crate::drawer_write::{
    DrawerNotice, DrawerNoticeBar, delete_channel, delete_group, find_channel_by_name,
    find_group_by_name, update_channel, update_group_display,
};
use crate::network_data::*;
use crate::network_physics::{accent_color, node_title_from_store};
use crate::state::EntityStore;

/// 侧边抽屉：左键点节点后就地编辑该实体。
///
/// `absolute` 覆盖画布右侧，画布尺寸恒定，开合不引起重排。
/// 三层的编辑内容不同：
/// - 分组：名字（可改）
/// - 模型别名：名字（可改）
/// - 调度模型：模型名**只读**（来自上游，改了就路由不到），
///   附带展示所属渠道的 URL/Key，渠道本身在设置页改
///
/// 【是什么】节点检视抽屉:页签头 + 类型色点 + 写操作通知条 + 主体
/// (`GroupInspect` / `AliasInspect` / `DispatchInspect` 三选一)+ 底部操作条
/// + 删除确认弹窗。
///
/// 【做什么】按 `node` 类型分流到对应检视体;负责底部的删除写入路径(定位
/// 后端实体 → 删除 → `bump_topo_refresh`)。不负责各检视体内的字段编辑
/// (在各 Inspect 组件里)、不负责画布本身。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点底部「删除」→ `confirming = true` 开确认弹窗;确认后 `do_delete`
///   `spawn` 异步:分组走 `find_group_by_name` + `delete_group`,调度节点走
///   `find_channel_by_name` + `delete_channel`(按名定位失败则 Err),成功后
///   `notice = Ok` 并 `bump_topo_refresh()`,失败 `notice = Err`。
/// - 点「✕」/点页签 → `on_close.call(e)` / `on_tab.call(t)` 抛回页面。
/// - 别名节点 `can_delete = false`,删除按钮禁用(无删除端点)。
/// 数据交互:删除路径发真实 DELETE 请求;其余为本地状态。
///
/// 【样式】抽屉 `absolute inset-y-0 right-0 z-20 flex w-full flex-col border-l
/// border-zinc-800 bg-zinc-900/97 backdrop-blur sm:w-[320px]`(窄屏满宽、sm
/// 起 320px);类型色点行 `border-b border-zinc-800 px-3 py-1.5`;主体
/// `min-h-0 flex-1 space-y-3 overflow-y-auto scroll-subtle p-3`;底部操作条
/// `flex shrink-0 items-center gap-2 border-t border-zinc-800 px-3 py-2`。
///
/// 【子组件组成】`DrawerHeader`(页签 + 标题)、`DrawerNoticeBar`(写通知条)、
/// `GroupInspect` / `AliasInspect` / `DispatchInspect`(三种检视体)、
/// `ui::Dialog`(删除确认弹窗)。
///
/// 【数据流】
/// - 对内(入):`node`(当前检视的节点,页面 `inspect()` 提供)、
///   `on_tab` / `on_close`(页面回调)。
/// - 对外(出):`on_tab(DrawerTab)` → 页面切抽屉页别;`on_close(MouseEvent)`
///   → 页面关抽屉并还原焦点前的位置(`saved_positions`);删除成功通过
///   `bump_topo_refresh()` 触发画布重拉。
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
        BTN_DELETE_GROUP_TITLE
    } else {
        BTN_DELETE_CHANNEL_TITLE
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
                    Ok(None) => Err(format!(
                        "{MSG_GROUP_MISSING_PREFIX}{node_name}{MSG_MISSING_SUFFIX}"
                    )),
                    Err(e) => Err(e),
                },
                NodeKey::Dispatch(_) => match find_channel_by_name(&ch_name).await {
                    Ok(Some(c)) => delete_channel(&c.key).await,
                    Ok(None) => Err(format!(
                        "{MSG_CHANNEL_MISSING_PREFIX}{ch_name}{MSG_MISSING_SUFFIX}"
                    )),
                    Err(e) => Err(e),
                },
                _ => Ok(()),
            };
            match res {
                Ok(()) => {
                    ns.set(DrawerNotice::Ok);
                    bump_topo_refresh();
                }
                Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_WRITE_FAILED_PREFIX}{e}"))),
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
                    title: if can_delete { del_label } else { MSG_DELETE_DISABLED_TITLE },
                    onclick: move |_| confirming.set(true),
                    {BTN_DELETE}
                }
                span { class: "flex-1" }
                button {
                    class: "rounded-md border border-zinc-100 bg-zinc-100 px-2.5 py-1 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    disabled: !can_delete,
                    title: if can_delete { MSG_SAVE_TITLE } else { MSG_SAVE_DISABLED_TITLE },
                    {BTN_SAVE}
                }
            }
            if confirming() {
                ui::Dialog {
                    title: del_label.to_string(),
                    open: true,
                    on_confirm: do_delete,
                    on_cancel: move |_| confirming.set(false),
                    p { {MSG_DELETE_CONFIRM} }
                }
            }
        }
    }
}

/// 分组检视体:分组名(锁读)+ 展示名(可改)+ 包含的别名列表。
///
/// 【是什么】检视抽屉里「分组」节点的编辑体:锁读的分组名、可改的展示名、
/// 该分组连到的模型别名列表。
///
/// 【做什么】从 `store.groups` 读第 `index` 行,从当前边集算该分组的出边别名;
/// 提供展示名保存写路径。不负责分组改名(后端无此列,诚实锁读)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 改展示名 → 写 `display_sig`(本地编辑态,非逐键写后端)。
/// - 点「保存展示名」→ `save_display` `spawn`:`find_group_by_name` 定位 →
///   `update_group_display`(写 remark 列),成功后 `notice = Ok` +
///   `bump_topo_refresh()`,定位失败/写失败 → `notice = Err`。
/// 数据交互:保存路径发真实 PUT 请求;列表中若 `index` 越界直接返回「不存在」。
///
/// 【样式】顶部 `DrawerNoticeBar`;字段用 `BoundField`(标签
/// `text-[11px] text-zinc-500` + 圆角描边输入框);保存按钮 `w-full rounded-md
/// border border-zinc-100 bg-zinc-100 ... hover:bg-zinc-300`;别名列表用
/// `InspectList`(圆角 pill chips)。
///
/// 【子组件组成】`DrawerNoticeBar`、`BoundField`、`InspectList`。
///
/// 【数据流】
/// - 对内(入):`index`(分组在 store 中的下标,来自 `NodeInspector`)。
/// - 对外(出):无直接 EventHandler;保存成功经 `bump_topo_refresh()` 让画布
///   重拉;写通知条经 `on_clear` 复位。
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
        return rsx! { p { class: "text-xs text-zinc-600", {MSG_GROUP_ABSENT} } };
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
                    Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}"))),
                },
                Ok(None) => ns.set(DrawerNotice::Err(format!(
                    "{MSG_GROUP_MISSING_PREFIX}{name}{MSG_MISSING_SUFFIX}"
                ))),
                Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}"))),
            }
        });
    };

    rsx! {
        DrawerNoticeBar { notice: group_notice, on_clear: move |_| group_notice.set(DrawerNotice::Idle) }
        BoundField {
            label: MSG_GROUP_NAME_LOCKED,
            value: gname.clone(),
            placeholder: "vip",
            on_change: move |_| (),
        }
        div { class: "space-y-1",
            BoundField {
                label: FIELD_DISPLAY,
                value: display_sig,
                placeholder: MSG_PH_DEFAULT_GROUP,
                on_change: move |v: String| display_sig.set(v),
            }
            button {
                class: "w-full rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                onclick: save_display,
                {BTN_SAVE_DISPLAY}
            }
        }
        InspectList { title: MSG_INSPECT_ALIASES_TITLE, items: aliases, empty: MSG_INSPECT_ALIASES_EMPTY }
    }
}

/// 别名检视体:别名/展示名双锁读 + 所属分组 + 路由到的调度模型。
///
/// 【是什么】检视抽屉里「模型别名」节点的只读检视体:两个锁读字段 + 两张
/// 关联列表。
///
/// 【做什么】从 `store.aliases` 读第 `index` 行,从边集算该别名的入边(所属
/// 分组)与出边(路由到的调度模型)。不负责任何写路径(该节点无删除端点,
/// 写路径按 UUID key 定位,此处只有名字定位不到,诚实锁读)。
///
/// 【交互逻辑】纯展示,无交互 —— 两个 `BoundField` 的 `on_change` 均为空闭包,
/// 列表项上的「✕」按钮当前未挂 onclick;无网络调用。
///
/// 【样式】两个字段用 `BoundField`(锁读标签 + 输入框);两张 `InspectList`
/// (标题 `text-[11px] text-zinc-500` + 圆角 pill chips,空态
/// `text-[11px] text-zinc-600`)。
///
/// 【子组件组成】`BoundField`、`InspectList`。
///
/// 【数据流】
/// - 对内(入):`index`(别名在 store 中的下标,来自 `NodeInspector`)。
/// - 对外(出):无 EventHandler / 无 signal 写回。
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
        return rsx! { p { class: "text-xs text-zinc-600", {MSG_ALIAS_ABSENT} } };
    };
    // 别名写路径（#175/#182）按 UUID key 走 /api/models/{key}，本 inspector
    // 只有名字定位不到 key → 锁读不丢写（与 b37a407 的锁读修复同型）。
    rsx! {
        BoundField {
            label: MSG_ALIAS_LOCKED,
            value: r.alias.clone(),
            placeholder: "gpt-4o",
            on_change: move |_| (),
        }
        BoundField {
            label: MSG_ALIAS_DISPLAY_LOCKED,
            value: r.display.clone(),
            placeholder: "GPT-4o",
            on_change: move |_| (),
        }
        InspectList { title: MSG_INSPECT_GROUPS_TITLE, items: groups, empty: MSG_INSPECT_GROUPS_EMPTY }
        InspectList { title: MSG_INSPECT_DISPATCH_TITLE, items: dispatch, empty: MSG_INSPECT_DISPATCH_EMPTY }
    }
}

/// 调度模型检视体:只读模型名 + 所属渠道编辑区 + 被哪些别名路由。
///
/// 【是什么】检视抽屉里「调度模型」节点的编辑体:锁读的模型名、所属渠道的
/// 名称/URL/Key 三字段(可改)+ 被哪些别名路由列表。
///
/// 【做什么】从 `view.dispatch[index]` 取 (渠道序号, 模型名),再取
/// `store.channels` 对应行做渠道编辑;提供渠道最小 diff 写路径。
/// 不负责模型改名(来自上游,只读)、不负责渠道的删除。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 改渠道名/URL/Key → 写组件内 `ch_name` / `ch_url` / `ch_keys` signal。
/// - 点「保存渠道」→ `save_channel` `spawn`:`find_channel_by_name`(用渲染期
///   原名 `orig_name` 定位,非编辑框现值)→ `update_channel`;keys 留空则传
///   `None`(请求体缺席 = 后端保留现有密钥);成功后 `notice = Ok`、清空 Key
///   输入、`bump_topo_refresh()` 并同步本地 store 行,失败 → `notice = Err`。
/// 数据交互:保存发真实 PUT 请求。
///
/// 【样式】模型名只读框 `rounded-md border border-zinc-800 bg-zinc-950
/// font-mono text-sm`;渠道编辑区 `rounded-lg border border-zinc-800
/// bg-zinc-950 p-3` + `space-y-2`;保存按钮 `w-full rounded-md border
/// border-zinc-100 bg-zinc-100 ... hover:bg-zinc-300`。
///
/// 【子组件组成】`DrawerNoticeBar`、`BoundField`(渠道名/URL)、`BoundArea`
/// (多行 Key)、`InspectList`(被路由列表)。
///
/// 【数据流】
/// - 对内(入):`index`(调度模型在 `view.dispatch` 中的下标,来自
///   `NodeInspector`)。
/// - 对外(出):无直接 EventHandler;保存成功经 `bump_topo_refresh()` 触发画布
///   重拉,并就地改 `store.channels` 对应行。
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
                    Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}"))),
                },
                Ok(None) => ns.set(DrawerNotice::Err(format!(
                    "{MSG_CHANNEL_MISSING_PREFIX}{lookup}{MSG_MISSING_SUFFIX}"
                ))),
                Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}"))),
            }
        });
    };

    rsx! {
        DrawerNoticeBar { notice: ch_notice, on_clear: move |_| ch_notice.set(DrawerNotice::Idle) }
        div { class: "space-y-1",
            span { class: "text-[11px] text-zinc-500", {MSG_MODEL_NAME_READONLY} }
            div { class: "rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 font-mono text-sm text-zinc-300", "{model_name}" }
        }
        if row.is_some() {
            div { class: "space-y-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3",
                span { class: "text-[11px] uppercase tracking-wider text-zinc-600", {MSG_OWNER_CHANNEL} }
                BoundField {
                    label: MSG_CHANNEL_NAME,
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
                    label: MSG_API_KEY_EDIT,
                    value: ch_keys,
                    placeholder: "sk-…\nsk-…",
                    on_change: move |v: String| ch_keys.set(v),
                }
                button {
                    class: "w-full rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    onclick: save_channel,
                    {BTN_SAVE_CHANNEL}
                }
            }
        }
        InspectList { title: MSG_INSPECT_ROUTED_TITLE, items: aliases, empty: MSG_INSPECT_ROUTED_EMPTY }
    }
}

/// 多行受控输入：给 Key 这种可包含多行的字段用。
///
/// 【是什么】带标签的多行受控输入(textarea)。
///
/// 【做什么】渲染标签 + 圆角描边的 textarea;受控(值来自 prop)、可纵向
/// 拉伸。不持状态、不校验。
///
/// 【交互逻辑】用户输入 → `on_change.call(e.value())` 把当前串抛给调用方 →
/// 调用方写回对应 signal,值再回流进 `value`。不发网络。
///
/// 【样式】标签 `text-[11px] text-zinc-500`;textarea `min-h-[72px] w-full
/// resize-y rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5
/// font-mono text-xs ... focus:border-zinc-500`。
///
/// 【子组件组成】无:原生 `label` / `span` / `textarea`。
///
/// 【数据流】
/// - 对内(入):`label`(标签静态串)、`value`(当前值)、`placeholder`。
/// - 对外(出):`on_change(String)` → 调用方写回 signal。
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
///
/// 【是什么】带标签的单行受控输入。
///
/// 【做什么】渲染标签 + 圆角描边的 input;受控、不持状态、不校验。
///
/// 【交互逻辑】用户输入 → `on_change.call(e.value())` 把当前串抛给调用方 →
/// 调用方写回 signal,值再回流进 `value`。不发网络。
///
/// 【样式】标签 `text-[11px] text-zinc-500`;input `w-full rounded-md border
/// border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm ... focus:border-zinc-500`。
///
/// 【子组件组成】无:原生 `label` / `span` / `input`。
///
/// 【数据流】
/// - 对内(入):`label`(标签静态串)、`value`(当前值)、`placeholder`。
/// - 对外(出):`on_change(String)` → 调用方写回 signal。
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

/// 一行只读凭据(标签 + 等宽值)。
///
/// 【是什么】一行只读的「短标签 + 值」文本行。
///
/// 【做什么】纯展示一行;不持状态、无交互。
///
/// 【交互逻辑】纯展示,无交互。
///
/// 【样式】行 `flex items-baseline gap-2`;标签 `w-8 shrink-0 text-[11px]
/// text-zinc-600`;值 `truncate font-mono text-[11px] text-zinc-400`。
///
/// 【子组件组成】无:原生 `div` / `span`。
///
/// 【数据流】
/// - 对内(入):`label`(短标签)、`value`(值)均为静态串。
/// - 对外(出):无 EventHandler / Signal。
#[component]
fn CredRow(label: &'static str, value: &'static str) -> Element {
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "w-8 shrink-0 text-[11px] text-zinc-600", "{label}" }
            span { class: "truncate font-mono text-[11px] text-zinc-400", "{value}" }
        }
    }
}

/// 单行输入,值与写回都走传入的 `Signal<String>`(可不经父组件透传)。
///
/// 【是什么】带标签的单行输入,值直接绑定到传入的 signal。
///
/// 【做什么】渲染标签 + input;读写均走 `value` signal,不额外透传回调。
/// 不持自有状态、不校验。
///
/// 【交互逻辑】用户输入 → `value.set(e.value())` 直接写 signal;展示读
/// `value.read()`。不发网络。
///
/// 【样式】与 `BoundField` 同款:标签 `text-[11px] text-zinc-500`,input
/// `w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5
/// text-sm ... focus:border-zinc-500`。
///
/// 【子组件组成】无:原生 `label` / `span` / `input`。
///
/// 【数据流】
/// - 对内(入):`label`、`value`(可读写的 signal 句柄)、`placeholder`。
/// - 对外(出):直接写回 `value` signal。
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

/// 带标题的字符串 chips 列表,空时显示空态文案。
///
/// 【是什么】一个带小标题的 pill 列表块(用于检视器里的关联项展示)。
///
/// 【做什么】按 `items` 渲染圆角 pill chips,空则显示 `empty` 文案;
/// 每个 chip 尾带一个「✕」(当前未挂 onclick)。不持状态。
///
/// 【交互逻辑】纯展示,无交互 —— chip 上的「✕」按钮当前无 onclick 处理。
///
/// 【样式】块 `space-y-1.5`;标题 `text-[11px] text-zinc-500`;空态
/// `text-[11px] text-zinc-600`;chips 容器 `flex flex-wrap gap-1.5`;
/// 单个 chip `inline-flex items-center gap-1 rounded-full border
/// border-zinc-700 bg-zinc-900 px-2 py-0.5 text-[11px] text-zinc-300`。
///
/// 【子组件组成】无:原生 `div` / `span` / `p` / `button`。
///
/// 【数据流】
/// - 对内(入):`title`(标题静态串)、`items`(字符串列表)、
///   `empty`(空态静态串)。
/// - 对外(出):无 EventHandler / Signal。
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
