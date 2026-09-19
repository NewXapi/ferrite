//! 抽屉组件：页签栏（`DrawerTabs`）、带标题的头部（`DrawerHeader`）
//! 与渠道导入表单（`ImportPanel`）。

use dioxus::prelude::*;

use super::data::*;
use super::shared::{
    BTN_CLOSE_TITLE, BTN_IMPORT, BTN_IMPORT_CHANNEL, BTN_SETTINGS, EXAMPLE_CHANNEL,
    LBL_API_KEY_MULTI, LBL_BASE_URL, LBL_CHANNEL_NAME_OPT, LBL_NODES, MSG_DEFAULT_CHANNEL_NAME,
    MSG_IMPORT_FAILED_PREFIX, MSG_IMPORT_KEY_REQUIRED,
};
use crate::drawer_write::{DrawerNotice, DrawerNoticeBar, create_channel_import};

/// 抽屉头：标题 + 三个页签（节点/设置/导入）+ 关闭。
///
/// 【是什么】抽屉顶部的页签条:节点 / 设置 / 导入三个等宽页签。
///
/// 【做什么】渲染三个页签并按 `active` 高亮当前项;不渲染标题与关闭按钮
/// (那是 `DrawerHeader` 的职责)、不管页签内容。
///
/// 【交互逻辑】点击某页签 → `on_tab.call(t)` 把目标 `DrawerTab` 抛回调用方
/// (页面据以写 `drawer_tab` signal)。数据交互:纯本地状态,不发网络。
///
/// 【样式】外壳 `shrink-0 border-b border-zinc-800`;页签 `flex-1 py-1.5
/// text-xs font-medium transition-colors`,激活态 `border-b-2 border-zinc-100
/// text-zinc-100`,未激活 `border-b-2 border-transparent text-zinc-500
/// hover:text-zinc-300`。
///
/// 【子组件组成】无子组件:原生 `div` / `button`。
///
/// 【数据流】
/// - 对内(入):`active`(当前页签,由调用方持有)。
/// - 对外(出):`on_tab(DrawerTab)` → 调用方切换当前页签。
#[component]
pub fn DrawerTabs(active: DrawerTab, on_tab: EventHandler<DrawerTab>) -> Element {
    rsx! {
        div { class: "shrink-0 border-b border-zinc-800",
            div { class: "flex",
                for (t, label) in [(DrawerTab::Node, LBL_NODES), (DrawerTab::Settings, BTN_SETTINGS), (DrawerTab::Import, BTN_IMPORT)] {
                    {
                        let active = t == active;
                        let tone = if active {
                            "border-b-2 border-zinc-100 text-zinc-100"
                        } else {
                            "border-b-2 border-transparent text-zinc-500 hover:text-zinc-300"
                        };
                        rsx! {
                            button {
                                class: "flex-1 py-1.5 text-xs font-medium transition-colors {tone}",
                                onclick: move |_| on_tab.call(t),
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 设置页内的纵向路线导航：只用小点，hover 出文字，点击滚动到卡片
///
/// 【是什么】完整抽屉头:页签条(复用 `DrawerTabs` 的布局但内联实现)+ 标题行
/// (标题 + 副标题 + 关闭按钮)。
///
/// 【做什么】为节点检视/导入两种抽屉提供统一的页签 + 标题外壳;不管页签
/// 内容,不持有状态。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点页签 → `on_tab.call(t)` 抛回调用方切页签;
/// - 点「✕」→ `on_close.call(e)` 抛回调用方(调用方据以关抽屉/还原视图)。
/// 数据交互:纯本地状态,不发网络。
///
/// 【样式】外壳 `shrink-0 border-b border-zinc-800`;页签与 `DrawerTabs`
/// 同款激活/未激活两态;标题行 `flex items-center gap-2 border-t
/// border-zinc-800 px-3 py-2`,标题 `truncate text-sm font-medium
/// text-zinc-100`,副标题 `truncate text-[11px] text-zinc-500`。
///
/// 【子组件组成】无子组件:原生 `div` / `button` / `p`。
///
/// 【数据流】
/// - 对内(入):`tab`(当前页签)、`title`(标题串,调用方按节点类型派生)、
///   `subtitle`(副标题串)。
/// - 对外(出):`on_tab(DrawerTab)` → 调用方切页签;`on_close(MouseEvent)` →
///   调用方关抽屉。
#[component]
pub fn DrawerHeader(
    tab: DrawerTab,
    title: String,
    subtitle: String,
    on_tab: EventHandler<DrawerTab>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "shrink-0 border-b border-zinc-800",
            // 页签栏放在最顶部（保持不动，下面才是标题）
            div { class: "flex",
                for (t, label) in [(DrawerTab::Node, LBL_NODES), (DrawerTab::Settings, BTN_SETTINGS), (DrawerTab::Import, BTN_IMPORT)] {
                    {
                        let active = t == tab;
                        let tone = if active {
                            "border-b-2 border-zinc-100 text-zinc-100"
                        } else {
                            "border-b-2 border-transparent text-zinc-500 hover:text-zinc-300"
                        };
                        rsx! {
                            button {
                                class: "flex-1 py-1.5 text-xs font-medium transition-colors {tone}",
                                onclick: move |_| on_tab.call(t),
                                "{label}"
                            }
                        }
                    }
                }
            }
            div { class: "flex items-center gap-2 border-t border-zinc-800 px-3 py-2",
                div { class: "min-w-0 flex-1",
                    p { class: "truncate text-sm font-medium text-zinc-100", "{title}" }
                    p { class: "truncate text-[11px] text-zinc-500", "{subtitle}" }
                }
                button {
                    class: "rounded-md px-1.5 text-zinc-500 hover:text-zinc-200",
                    title: BTN_CLOSE_TITLE,
                    onclick: move |e| on_close.call(e),
                    "✕"
                }
            }
        }
    }
}

/// 导入：凭 URL + Key 新增渠道（真实 POST /api/channel，全量
/// `ChannelUpsertRequest`）。导入后 `bump_topo_refresh()` 让画布重拉；
/// 成功/失败在抽屉内报（`DrawerNotice`），不再写本地 store 演示行。
/// 导入的渠道尚未加入调度模型（`models` 发空数组），最终由用户在渠道里
/// 「加入调度」才会进入拓扑。
///
/// 【是什么】抽屉「导入」页签的表单:渠道名(可选) + Base URL + 多行 API Key
/// + 提交按钮,顶部带写操作通知条。
///
/// 【做什么】把三个输入组装成一次 `create_channel_import`(真实 POST),
/// 成功后重置输入并 `bump_topo_refresh` 让画布重拉。不负责解析粘贴文本
/// (Key 按行拆,不做 URL/Key 自动识别)、不负责导入后的渠道调度。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 输入三个字段 → 写回组件内 `alias` / `url` / `key` signal,不发网络。
/// - 点「导入渠道」→ 先 trim:Key 拆行后为空则把错误写进 `notice` 并返回;
///   否则 `spawn` 调 `create_channel_import(name, url, "openai", ["default"],
///   keys)`,期间 `notice = Busy`,成功后 `notice = Ok` + `bump_topo_refresh()`
///   + 清空三个输入,失败则 `notice = Err("导入失败：…")`。
/// 提交按钮 `disabled` 由 `can_import`(URL 与 Key 均非空)控制。
///
/// 【样式】表单 `space-y-3`;字段 `label.block.space-y-1.5` + 标签
/// `text-[11px] text-zinc-500`;输入框统一 `rounded-md border border-zinc-800
/// bg-zinc-950 px-3 py-1.5 focus:border-zinc-500`;Key 用 `textarea` +
/// `font-mono`;提交按钮 `w-full rounded-md border border-zinc-100 bg-zinc-100
/// text-zinc-900`,可用时 hover 加深、不可用时 `cursor-not-allowed opacity-50`。
///
/// 【子组件组成】`DrawerNoticeBar`(写操作通知条,来自 `crate::drawer_write`)。
///
/// 【数据流】
/// - 对内(入):无 props;`url` / `key` / `alias` / `notice` 四个 signal 均为
///   组件内 `use_signal`。
/// - 对外(出):无 EventHandler;仅通过 `bump_topo_refresh()` 递增全局拓扑
///   刷新版本号,页面轮询发现后重拉画布数据。
#[component]
pub fn ImportPanel() -> Element {
    let mut url = use_signal(String::new);
    let mut key = use_signal(String::new);
    let mut alias = use_signal(String::new);
    let mut notice = use_signal(|| DrawerNotice::Idle);

    let can_import = !url.read().trim().is_empty() && !key.read().trim().is_empty();

    let import = move |_| {
        let n = alias.peek().trim().to_string();
        let name = if n.is_empty() {
            MSG_DEFAULT_CHANNEL_NAME.into()
        } else {
            n
        };
        let u = url.peek().trim().to_string();
        // 多 key 按行拆分、trim、去空行（与 drawer_write::create_channel_import 约定一致）
        let kvec: Vec<String> = key
            .peek()
            .trim()
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if kvec.is_empty() {
            notice.set(DrawerNotice::Err(MSG_IMPORT_KEY_REQUIRED.into()));
            return;
        }
        let groups = vec!["default".to_string()];
        let (mut ns, mut na, mut nu, mut nk) = (notice, alias, url, key);
        spawn(async move {
            ns.set(DrawerNotice::Busy);
            match create_channel_import(&name, &u, "openai", &groups, &kvec).await {
                Ok(_) => {
                    ns.set(DrawerNotice::Ok);
                    bump_topo_refresh();
                    na.set(String::new());
                    nu.set(String::new());
                    nk.set(String::new());
                }
                Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_IMPORT_FAILED_PREFIX}{e}"))),
            }
        });
    };

    rsx! {
        div { class: "space-y-3",
            DrawerNoticeBar { notice, on_clear: move |_| notice.set(DrawerNotice::Idle) }
            label { class: "block space-y-1.5",
                span { class: "text-[11px] text-zinc-500", {LBL_CHANNEL_NAME_OPT} }
                input {
                    class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                    value: "{alias.read()}",
                    placeholder: EXAMPLE_CHANNEL,
                    oninput: move |e| alias.set(e.value()),
                }
            }
            label { class: "block space-y-1.5",
                span { class: "text-[11px] text-zinc-500", {LBL_BASE_URL} }
                input {
                    class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                    value: "{url.read()}",
                    placeholder: "https://…",
                    oninput: move |e| url.set(e.value()),
                }
            }
            label { class: "block space-y-1.5",
                span { class: "text-[11px] text-zinc-500", {LBL_API_KEY_MULTI} }
                textarea {
                    class: "min-h-[96px] w-full resize-none rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-200 outline-none placeholder:text-zinc-600 focus:border-zinc-500",
                    value: "{key.read()}",
                    placeholder: "sk-…
    sk-…",
                    oninput: move |e| key.set(e.value()),
                }
            }
            button {
                class: "w-full rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 transition-colors",
                class: if can_import { "hover:bg-zinc-300" } else { "cursor-not-allowed opacity-50" },
                disabled: !can_import,
                onclick: import,
                {BTN_IMPORT_CHANNEL}
            }
        }
    }
}
