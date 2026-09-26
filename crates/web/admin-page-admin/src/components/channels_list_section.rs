//! 渠道卡片网格区(编号段 3):标题计数 + 四态分支 + 新卡牌(ui::ChannelCard)网格。
//!
//! 纯展示组件:数据以值传入;交互通过 EventHandler 抛回页面。
//! on_toggle 收 (key, target_status) —— 卡片算好目标状态(1=启用 2=停用)抛上来,
//! 页面直接写 API,组件内不持状态。
//!
//! 边界:筛选在 `page.rs` 调 `filter_channels` 完成,本文件不再过滤;启停/删除/
//! 单字段编辑的网络写回都在页面闭包里,本文件只转发 key 与字段。

use contract::api::admin::ChannelDto;
use dioxus::prelude::*;

use crate::shared::{
    BTN_RETRY, MSG_EMPTY_CHANNELS, MSG_LOAD_FAILED_CHANNELS, MSG_LOADING_LIST_CHANNELS,
    OPT_BADGE_LOADING, SEC_LIST_CHANNELS,
};

/// 渠道卡片网格区:错误 / 加载 / 空 / 网格 四态。
///
/// 【是什么】渠道 tab 的编号段 3:标题行 + 计数徽标 + 四态分支 + 新卡牌网格。
/// 网格即 ui-components 的 `ChannelCard`(UI 决策记录 §2.2/§2.3:展示卡 + ≤3 tab
/// + 行内 Popover 编辑),本文件只做四态分支与网格排版。
///
/// 【做什么】按 `err` / `loading` / `filtered` 渲染四种形态之一,有数据时把每条
/// `ChannelDto` 铺成 `ui::ChannelCard`。不负责筛选、不拉数据、不发任何写请求
/// (只把意图抛回页面)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 错误态「重试」→ `on_retry`(MouseEvent)抛回页面 `reload + 1`。
/// - 卡片行内 Popover 保存 → 卡片抛 `(字段, 原始值)`,本组件补 `key` 组成三元组
///   调 `on_edit`,页面按字段构造 `UpdateChannelBody` 最小 diff PUT。
/// - 卡片「完整编辑」行(密钥/模型/分组等弹窗专属能力)→ `on_full_edit.call(key)`,
///   页面 `open_edit` 回填表单。
/// - 卡片「启用/停用」→ 组件先算目标状态(`status == 1` 时目标 2,否则 1),
///   调 `on_toggle.call((key, target))`,由页面落成 `WriteOp::Toggle` 发 API。
/// - 卡片「删除」(Dialog 确认后)→ `on_delete.call(key)`,由页面落成
///   `WriteOp::Delete` 发 API。
/// 数据交互:本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 `section#channels-sec-list` 为 `scroll-mt-8 space-y-4`;标题
/// `{ui::TYPE_TITLE}` + 右侧 `rounded-full bg-zinc-800` 计数胶囊;
/// 错误态红底 `border-red-800/60 bg-red-950/40`,加载/空态为虚线描边
/// `border-dashed border-zinc-700 bg-zinc-900/50 py-16`;网格
/// `grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5`(手机 1 / 中屏 3 / 大屏 5 列)。
///
/// 【子组件组成】`ui::ChannelCard`(唯一卡片,含行内 Popover 编辑 / 启停 / 删除确认)。
///
/// 【数据流】
/// - 对内(入):`loading`(首屏加载态;后台刷新走页面的 `refreshing`,不传这里)、
///   `err`(优先于 loading 渲染)、`filtered`(页面已筛好的渠道列表)、
///   `on_edit` / `on_full_edit` / `on_toggle` / `on_delete` / `on_retry` 五个回调。
/// - 对外(出):`on_edit((String, ChannelEditField, String))` → 页面
///   `commit_channel_field` 最小 diff PUT;`on_full_edit(String)` → 页面 `open_edit`
///   开弹窗;`on_toggle((String, i16))` → 页面 `on_toggle_card` →
///   `set_channel_status_api`;`on_delete(String)` → 页面 `on_delete_card` →
///   `delete_channel_api`;`on_retry(MouseEvent)` → 页面重拉。
#[component]
pub fn ChannelsListSection(
    /// 列表加载中(骨架态;后台刷新不清列表,走 refreshing,不传这里)
    loading: bool,
    /// 拉取失败摘要(错误态;Some 时优先于 loading 渲染)
    err: Option<String>,
    /// 筛选后的渠道列表,页面派生
    filtered: Vec<ChannelDto>,
    /// 行内 Popover 保存:(key, 字段, 原始字符串),页面最小 diff PUT
    on_edit: EventHandler<(String, ui::ChannelEditField, String)>,
    /// 请求完整编辑(开弹窗:密钥/模型/分组)
    on_full_edit: EventHandler<String>,
    /// 启停切换:(key, 目标状态 1|2)
    on_toggle: EventHandler<(String, i16)>,
    /// 请求删除(Dialog 确认后)
    on_delete: EventHandler<String>,
    /// 错误态「重试」
    on_retry: EventHandler<MouseEvent>,
) -> Element {
    // 分页：列表内部 UI 状态（不跨组件）；筛选后条数变小时 clamp 到最后一页，
    // 不落空页（详见 aliases list 同款注释）。
    let mut page = use_signal(|| 0usize);
    let visible = ui::page_slice(&filtered, page(), ui::CARD_PAGE_SIZE).to_vec();

    rsx! {
        section { id: "channels-sec-list", class: "scroll-mt-8 space-y-4",
            ui::SectionHeader {
                title: SEC_LIST_CHANNELS.to_string(),
                badge: if loading { OPT_BADGE_LOADING.to_string() } else { format!("{} 个渠道", filtered.len()) },
                trailing: rsx! {
                    ui::Pager {
                        total: filtered.len(),
                        page,
                        on_change: move |p| page.set(p),
                        testid: "channels-pager",
                    }
                },
            }

            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                    p { class: "text-sm text-red-300", "{MSG_LOAD_FAILED_CHANNELS}" }
                    p { class: "mt-1 text-xs {ui::C_DANGER}", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-border px-3 py-1.5 {ui::TYPE_DESC} hover:bg-secondary",
                        onclick: on_retry,
                        "{BTN_RETRY}"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "{MSG_LOADING_LIST_CHANNELS}" }
                }
            } else if filtered.is_empty() {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "{MSG_EMPTY_CHANNELS}" }
                }
            } else {
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    "data-testid": "channels-list",
                    for c in visible {
                        {
                            let edit_key = c.key.clone();
                            let full_key = c.key.clone();
                            let toggle_key = c.key.clone();
                            let delete_key = c.key.clone();
                            // 后端 status 语义: 1=启用 2=停用 (channels status 校验 [1,2])
                            let target = if c.status == 1 { 2 } else { 1 };
                            rsx! {
                                ui::ChannelCard {
                                    key: "{c.key}",
                                    channel: c,
                                    on_edit: move |(field, value): (ui::ChannelEditField, String)| on_edit.call((edit_key.clone(), field, value)),
                                    on_full_edit: move |_| on_full_edit.call(full_key.clone()),
                                    on_toggle: move |_| on_toggle.call((toggle_key.clone(), target)),
                                    on_delete: move |_| on_delete.call(delete_key.clone()),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
