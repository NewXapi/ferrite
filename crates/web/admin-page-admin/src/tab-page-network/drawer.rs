//! 抽屉组件：页签栏（`DrawerTabs`）、带标题的头部（`DrawerHeader`）
//! 与渠道导入表单（`ImportPanel`）。

use dioxus::prelude::*;

use super::data::*;
use crate::drawer_write::{DrawerNotice, DrawerNoticeBar, create_channel_import};

/// 抽屉头：标题 + 三个页签（节点/设置/导入）+ 关闭。
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
                    title: "关闭",
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
#[component]
pub fn ImportPanel() -> Element {
    let mut url = use_signal(String::new);
    let mut key = use_signal(String::new);
    let mut alias = use_signal(String::new);
    let mut notice = use_signal(|| DrawerNotice::Idle);

    let can_import = !url.read().trim().is_empty() && !key.read().trim().is_empty();

    let import = move |_| {
        let n = alias.peek().trim().to_string();
        let name = if n.is_empty() { "新渠道".into() } else { n };
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
            notice.set(DrawerNotice::Err("API Key 不能为空".into()));
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
                Err(e) => ns.set(DrawerNotice::Err(format!("导入失败：{e}"))),
            }
        });
    };

    rsx! {
        div { class: "space-y-3",
            DrawerNoticeBar { notice, on_clear: move |_| notice.set(DrawerNotice::Idle) }
            label { class: "block space-y-1.5",
                span { class: "text-[11px] text-zinc-500", "渠道名（可选）" }
                input {
                    class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                    value: "{alias.read()}",
                    placeholder: EXAMPLE_CHANNEL,
                    oninput: move |e| alias.set(e.value()),
                }
            }
            label { class: "block space-y-1.5",
                span { class: "text-[11px] text-zinc-500", "Base URL" }
                input {
                    class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                    value: "{url.read()}",
                    placeholder: "https://…",
                    oninput: move |e| url.set(e.value()),
                }
            }
            label { class: "block space-y-1.5",
                span { class: "text-[11px] text-zinc-500", "API Key（多 key 换行）" }
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
                "导入渠道"
            }
        }
    }
}
