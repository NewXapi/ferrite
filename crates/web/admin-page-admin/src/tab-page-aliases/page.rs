//! 模型别名管理页:卡片式网格,对齐 GroupsPage 规范。
//!
//! 数据接线(对齐 GroupsPage 模式,本地 signal 不触碰 EntityStore):
//! - 列表:挂载/reload 时 `list_model_aliases_api` 拉 GET /api/models?size=100,
//!   只映射 ModelView 的 key(写路径定位 UUID)与 name;价格/倍率字段后端无
//!   对应列,展示为 0/1.0(见页面说明条)。
//! - 编辑:`update_model_alias_api` PUT /api/models/{key},请求体只携带 name
//!   (后端 models 域唯一与别名对应的列)。
//! - 删除:`delete_model_alias_api` DELETE /api/models/{key}。
//! - 新建:后端 POST /api/models 的 CreateModelRequest 必填 owner 与 api_key,
//!   表单没有这两个字段的来源 — 提交时诚实提示,不造数据、不假成功。
//!
//! 本文件只保留状态与写回逻辑(拉取 / submit_alias / open_edit / open_new /
//! delete);渲染拆成 `card`(卡片)与 `modal`
//! (新建/编辑弹窗),共享类型与判定见 `shared`。

use client::ApiClient;
use contract::api::admin::GroupDto;
use contract::api::billing::AliasUpsertRequest;
use dioxus::prelude::*;
use ui::{AliasCard as PrototypeAliasCard, SegmentedCapsule};

use super::card::AliasCard;
use super::modal::AliasFormModal;
use super::shared::{
    AliasItem, AliasModalState, PriceMode, SEC_FILTER, SEC_LIST, SEC_STATS, usable_groups_for,
};
use crate::api::{
    delete_model_alias_api, list_groups_api, list_model_aliases_api, update_model_alias_api,
};
use crate::state::AliasRow;
use crate::tab_page_groups::StatCard;
/// 别名管理页
#[component]
pub fn AliasesPage() -> Element {
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut rows = use_signal(Vec::<AliasItem>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    // 弹窗提交进行中(独立于删除的 busy,只禁用弹窗提交按钮)
    let submitting = use_signal(|| false);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| AliasModalState::Closed);
    // 分组列表(用于卡片展示「哪些分组可用此别名 + 各分组倍率」)
    let mut groups = use_signal(Vec::<GroupDto>::new);

    let mut f_name = use_signal(String::new);
    let mut f_display = use_signal(String::new);
    let mut f_input = use_signal(|| "0.0175".to_string());
    let mut f_output = use_signal(|| "0.07".to_string());
    let mut f_mult = use_signal(|| "1.0".to_string());
    // 定价模式:per-card 独立,存在 AliasItem 里(见下方),弹窗打开时从该 card 读
    let mut f_price_mode = use_signal(|| PriceMode::PerToken);
    // 弹窗活动 tab:0 基本 / 1 按量定价 / 2 按次定价
    let mut f_modal_tab = use_signal(|| 0usize);
    // 按量/按次价格项的本地状态(后端无 pricing 列,纯 UI 占位;打开弹窗时重置默认值)
    let mut p_input = use_signal(|| "3".to_string());
    let mut p_output = use_signal(|| "15".to_string());
    let mut p_cache_read = use_signal(|| "0.3".to_string());
    let mut p_cache_write = use_signal(|| "0.75".to_string());
    let mut p_completion = use_signal(|| "2.5".to_string());
    let mut p_per_call = use_signal(|| "0.05".to_string());
    // 各通道启用状态:「基本」tab 的胶囊开关控制,卡片只渲染启用中的通道
    let mut c_output_on = use_signal(|| true);
    let mut c_cache_read_on = use_signal(|| true);
    let mut c_cache_write_on = use_signal(|| true);
    let mut c_completion_on = use_signal(|| true);

    // 挂载即拉取真实列表 + 分组(卡片要展示各分组倍率);reload 变化时重拉
    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            // 分组白名单拉取(独立,失败不阻塞别名列表)
            if let Ok(gs) = list_groups_api(&client).await {
                groups.set(gs);
            }
            match list_model_aliases_api(&client).await {
                Ok(list) => {
                    let mut items: Vec<AliasItem> = list
                        .into_iter()
                        .map(|m| AliasItem {
                            key: m.key,
                            row: AliasRow {
                                alias: m.name,
                                display: String::new(),
                                input_per_1k: 0.0,
                                output_per_1k: 0.0,
                                multiplier: 1.0,
                            },
                            price_mode: PriceMode::PerToken,
                        })
                        .collect();
                    // 与 EntityStore::hydrate 的 /api/models 映射保持一致:按别名排序
                    items.sort_by(|a, b| a.row.alias.cmp(&b.row.alias));
                    rows.set(items);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let alias_list = rows
        .read()
        .iter()
        .map(|it| it.row.clone())
        .collect::<Vec<_>>();
    let total = alias_list.len();
    let free_count = alias_list.iter().filter(|a| a.multiplier == 0.0).count();
    let standard_count = alias_list
        .iter()
        .filter(|a| (a.multiplier - 1.0).abs() < 0.001)
        .count();
    let custom_count = alias_list
        .iter()
        .filter(|a| a.multiplier != 0.0 && (a.multiplier - 1.0).abs() >= 0.001)
        .count();
    let avg_mult = if total > 0 {
        alias_list.iter().map(|a| a.multiplier).sum::<f64>() / (total as f64)
    } else {
        1.0
    };

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总别名数"),
        (standard_count.to_string(), "标准 1.0× 别名"),
        (custom_count.to_string(), "自定倍率别名"),
        (free_count.to_string(), "免费别名 (0×)"),
        (format!("{:.2}×", avg_mult), "平均加价倍率"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("标准 1.0× ({standard_count})"),
        format!("自定倍率 ({custom_count})"),
        format!("免费通道 ({free_count})"),
    ];

    let filtered: Vec<(usize, AliasItem)> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        rows()
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                let a = &it.row;
                if !q.is_empty()
                    && !a.alias.to_lowercase().contains(&q)
                    && !a.display.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => (a.multiplier - 1.0).abs() < 0.001,
                    2 => a.multiplier != 0.0 && (a.multiplier - 1.0).abs() >= 0.001,
                    3 => a.multiplier == 0.0,
                    _ => true,
                }
            })
            .map(|(i, it)| (i, it.clone()))
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_display.set(String::new());
        f_input.set("0.0175".to_string());
        f_output.set("0.07".to_string());
        f_mult.set("1.0".to_string());
        f_price_mode.set(PriceMode::PerToken);
        f_modal_tab.set(0);
        // 价格项重置为默认占位值(新建无后端数据,UI 层先给一个合理默认)
        p_input.set("3".to_string());
        p_output.set("15".to_string());
        p_cache_read.set("0.3".to_string());
        p_cache_write.set("0.75".to_string());
        p_completion.set("2.5".to_string());
        p_per_call.set("0.05".to_string());
        c_output_on.set(true);
        c_cache_read_on.set(true);
        c_cache_write_on.set(true);
        c_completion_on.set(true);
        modal_state.set(AliasModalState::New);
    };

    let open_edit = move |key: String| {
        if let Some(it) = rows().iter().find(|it| it.key == key) {
            f_name.set(it.row.alias.clone());
            f_display.set(it.row.display.clone());
            f_input.set(format!("{}", it.row.input_per_1k));
            f_output.set(format!("{}", it.row.output_per_1k));
            f_mult.set(format!("{}", it.row.multiplier));
            f_price_mode.set(it.price_mode);
            f_modal_tab.set(0);
            modal_state.set(AliasModalState::Edit(key));
        }
    };

    // 卡片面板上的定价 toggle:更新该 card 独立的定价模式(per-card,不共享),
    // 写回 rows 里对应 item 的 price_mode;后端不落地,纯 UI 本地状态。
    // `make_mode_handler` 每次返回独立 EventHandler,move 进 rsx 闭包。
    let make_mode_handler = |key: String| -> EventHandler<PriceMode> {
        let mut items_sig = rows;
        EventHandler::new(move |mode: PriceMode| {
            let mut items = items_sig().to_vec();
            if let Some(it) = items.iter_mut().find(|it| it.key == key) {
                it.price_mode = mode;
            }
            items_sig.set(items);
        })
    };

    // 删除:走真实 DELETE,成功后本地从 rows 移除该项(不整体重拉,避免列表
    // 闪 loading 骨架 + 高度剧变引起页面跳动);只有错误才提示,成功静默。
    let write_delete = move |key: String| {
        let (mut b, mut n, mut items_sig) = (busy, notice, rows);
        spawn(async move {
            b.set(true);
            n.set(None);
            let client = ApiClient::shared().clone();
            match delete_model_alias_api(&client, &key).await {
                Ok(_) => {
                    // 成功:本地直接移除,不 bump reload
                    let mut items = items_sig().to_vec();
                    items.retain(|it| it.key != key);
                    items_sig.set(items);
                    n.set(Some("已删除".to_string()));
                }
                Err(e) => n.set(Some(format!("删除失败:{e}"))),
            }
            b.set(false);
        });
    };

    // 弹窗提交:校验 + 网络写回都在本闭包里,弹窗组件只抛事件(见
    // modal)。编辑走真实 PUT /api/models/{key},新建诚实拒绝。
    let submit_alias = move |_| {
        let name = f_name.peek().trim().to_string();
        if name.is_empty() {
            return;
        }
        // 先取出现有状态再 match,避免读锁未释放就 set
        let editing_key = match modal_state() {
            AliasModalState::Edit(k) => Some(k),
            AliasModalState::New => None,
            AliasModalState::Closed => return,
        };
        match editing_key {
            // 编辑:PUT /api/models/{key}。请求体只带 name — 后端 models 域
            // 唯一与别名页对应的列只有 name,display/价格/倍率/定价模式无对应列,
            // 由后端 UpdateModelRequest(全 Option)忽略,不写库。定价字段为 UI
            // 层本地状态,后端落地时再扩展 models 域。
            Some(k) => {
                let (mut sub, mut n, mut ms) = (submitting, notice, modal_state);
                spawn(async move {
                    sub.set(true);
                    n.set(None);
                    let client = ApiClient::shared().clone();
                    let req = AliasUpsertRequest {
                        name,
                        ..Default::default()
                    };
                    if let Err(e) = update_model_alias_api(&client, &k, &req).await {
                        n.set(Some(format!("保存失败:{e}")));
                    }
                    sub.set(false);
                    ms.set(AliasModalState::Closed);
                });
            }
            // 新建:后端 CreateModelRequest 必填 owner 与 api_key,表单没有
            // 这两个字段的来源 — 诚实拒绝,不造数据、不假成功。
            None => {
                notice.set(Some(
                    "新建未执行:后端创建模型需要 owner 与 api_key 字段,当前表单未提供".to_string(),
                ));
                modal_state.set(AliasModalState::Closed);
            }
        }
    };

    rsx! {
            div { class: "flex flex-col gap-6",
                // 通知条(成功/错误/进行中,对齐 GroupsPage)
                if let Some(msg) = notice() {
                    div {
                        role: "status",
                        class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
                        "{msg}"
                        if busy() { " ···" }
                    }
                }

                // 数据与写路径说明(后端 models 端点暂无计费字段)
                div { class: "flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700/60 bg-zinc-900/60 px-4 py-2.5 text-xs text-zinc-400",
                    span { "别名来自真实 /api/models;编辑与删除已接后端;定价模式与价格配置为 UI 层本地状态,后端扩展 pricing 列前保存不写库;新建暂未开放(后端需要 owner/api_key 字段)" }
                }
                // 1. 统计区
                section { id: "aliases-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选与操作区
                section {
                    id: "aliases-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                            span { class: "text-xs text-zinc-500", "按倍率与资费规则快速筛选" }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                                "data-testid": "refresh-aliases",
                                onclick: move |_| reload.set(reload() + 1),
                                "刷新"
                            }
                            button {
                                class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                                onclick: open_new,
                                "✚ 新建别名"
                            }
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索别名 ID 或展示名称 (如 gpt-4o, claude-sonnet)...",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                    }

                    div { class: "flex flex-wrap gap-2",
                        SegmentedCapsule {
                            items: filter_options,
                            active: filter_tier(),
                            on_select: move |i: usize| filter_tier.set(i),
                        }
                    }
                }

                // 3. 卡片网格区
                section { id: "aliases-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                        span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                            if loading() { "加载中…" } else { "{filtered.len()} 个" }
                        }
                    }

                    if let Some(e) = err() {
                        div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                            p { class: "text-sm text-red-300", "加载别名失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                onclick: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if loading() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "正在加载模型别名…" }
                        }
                    } else if filtered.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的模型别名" }
                        }
                    } else {
                        if let Some((prototype_index, prototype_item)) = filtered.first() {
                            {
                                // 新卡示例仅消费当前筛选结果的首条真实数据；旧卡片网格与其写路径保持不变。
                                let prototype_key = prototype_item.key.clone();
                                let prototype_alias = prototype_item.row.alias.clone();
                                let prototype_display = prototype_item.row.display.clone();
                                let prototype_input_per_1k = prototype_item.row.input_per_1k;
                                let prototype_output_per_1k = prototype_item.row.output_per_1k;
                                let prototype_multiplier = prototype_item.row.multiplier;
                                let prototype_index = *prototype_index;
                                // 分组可用性:白名单为空(全可用)或显式包含该别名,
                                // 与旧卡片网格共用 usable_groups_for 判定。
                                let prototype_usable_groups =
                                    usable_groups_for(&prototype_alias, &groups.read());
                                rsx! {
                                    div {
                                        class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                        role: "region",
                                        "aria-label": "别名新卡示例",
                                        "data-testid": "alias-card-prototype",
                                        PrototypeAliasCard {
                                            alias: prototype_alias,
                                            display: prototype_display,
                                            input_per_1k: prototype_input_per_1k,
                                            output_per_1k: prototype_output_per_1k,
                                            multiplier: prototype_multiplier,
                                            index: prototype_index,
                                            usable_groups: prototype_usable_groups,
                                            alias_key: prototype_key.clone(),
                                        }
                                    }
                                }
                            }
                        }
                        div {
                            class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            role: "list",
                            "aria-label": "别名列表",
                            "data-testid": "aliases-list",
                            for (idx, it) in filtered {
                                {
                                    let key_ref = it.key.clone();
                                    rsx! {
                                        AliasCard {
                                            key: "{it.key}",
                                            alias_key: it.key,
                                            alias: it.row,
                                            index: idx,
                                            price_mode: it.price_mode,
                                            groups: groups.read().clone(),
                                            c_output_on: c_output_on(),
                                            c_cache_read_on: c_cache_read_on(),
                                            c_cache_write_on: c_cache_write_on(),
                                            c_completion_on: c_completion_on(),
                                            on_mode_change: make_mode_handler(key_ref),
                                            on_edit: open_edit,
                                            on_delete: write_delete,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if matches!(modal_state(), AliasModalState::New | AliasModalState::Edit(_)) {
                AliasFormModal {
                    editing: matches!(modal_state(), AliasModalState::Edit(_)),
                    alias: f_name,
                    display: f_display,
                    input_rate: f_input,
                    output_rate: f_output,
                    multiplier: f_mult,
                    price_mode: f_price_mode,
                    active_tab: f_modal_tab,
                    p_input,
                    p_output,
                    p_cache_read,
                    p_cache_write,
                    p_completion,
                    p_per_call,
                    c_output_on,
                    c_cache_read_on,
                    c_cache_write_on,
                    c_completion_on,
                    submitting,
                    on_cancel: move |_| modal_state.set(AliasModalState::Closed),
                    on_submit: submit_alias,
                }
            }
    }
}

