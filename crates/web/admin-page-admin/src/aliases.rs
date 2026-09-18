//! 模型别名管理页:卡片式网格,对齐 GroupsPage 规范。
//!
//! 数据接线(对齐 GroupsPage 模式,本地 signal 不触碰 EntityStore):
//! - 列表:挂载/reload 时 `list_model_aliases_api` 拉 GET /api/models?size=100,
//!   映射 ModelView 的 key(写路径定位 UUID)、name 与定价三字段
//!   (input_per_1k/output_per_1k/multiplier,后端 0018 起落库)。
//! - 编辑:`update_model_alias_api` PUT /api/models/{key},请求体携带 name 与
//!   定价三字段(后端 `UpdateModelRequest` COALESCE 合并写库);成功后用返回
//!   view 按 key 就地刷新列表行,不整体重拉。
//! - 删除:`delete_model_alias_api` DELETE /api/models/{key}。
//! - 新建:后端 POST /api/models 的 CreateModelRequest 必填 owner 与 api_key,
//!   表单没有这两个字段的来源 — 提交时诚实提示,不造数据、不假成功。

use client::ApiClient;
use contract::api::admin::GroupDto;
use contract::api::billing::AliasUpsertRequest;
use dioxus::prelude::*;
use ui::{AliasCard as PrototypeAliasCard, SegmentedCapsule};

use crate::api::{
    delete_model_alias_api, list_groups_api, list_model_aliases_api, update_model_alias_api,
};
use crate::groups::{Modal, StatCard, parse_whitelist};
use crate::state::AliasRow;

/// 别名列表项:后端 ModelView 的 key(UUID) + 页面展示行 + per-card 定价模式。
/// key 不并入 AliasRow — AliasRow 被 entities.rs 结构体字面量构造,
/// 本页独立持有 key 以定位 PUT/DELETE 路径;price_mode 是纯 UI 概念
/// (后端 models 域无 pricing_mode 列),只驱动卡片/弹窗的展示口径,
/// 保存时统一发三定价字段(见 `AliasFormModal` 的提交注释)。
#[derive(Clone, PartialEq)]
struct AliasItem {
    key: String,
    row: AliasRow,
    price_mode: PriceMode,
}

/// 弹窗状态(Edit 携带后端模型 UUID key)
#[derive(Clone, PartialEq)]
enum AliasModalState {
    Closed,
    New,
    Edit(String),
}

/// 计算「可用此别名的分组及其倍率」。
///
/// 后端 `model_whitelist` 是「分组内可用的模型名列表」;空白名单 = 该分组
/// 可用全部模型。因此「可用此别名」= 白名单为空(默认全可用)或显式包含
/// 该别名。原型新卡与旧卡片网格共用此判定,避免两处逻辑分叉。
fn usable_groups_for(alias: &str, groups: &[GroupDto]) -> Vec<(String, f64)> {
    groups
        .iter()
        .filter(|g| {
            let names = parse_whitelist(&g.model_whitelist);
            names.is_empty() || names.iter().any(|n| n.as_str() == alias)
        })
        .map(|g| (g.name.clone(), g.ratio))
        .collect()
}

/// 别名管理页
#[component]
pub fn AliasesPage() -> Element {
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut rows = use_signal(Vec::<AliasItem>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
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
    // 按量/按次价格项的本地状态(补充通道/按次单价后端无对应列,纯 UI 占位;
    // 打开弹窗时重置默认值;输入价/输出价/倍率另走 f_input/f_output/f_mult,写库)
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
                                input_per_1k: m.input_per_1k,
                                output_per_1k: m.output_per_1k,
                                multiplier: m.multiplier,
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
    // 写回 rows 里对应 item 的 price_mode;后端无 pricing_mode 列,纯 UI 展示态。
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

    // 弹窗关闭:只关弹窗(编辑保存已按 key 就地刷新列表行,无需整体重拉)
    let close_modal = move |_| {
        modal_state.set(AliasModalState::Closed);
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

                // 数据与写路径说明
                div { class: "flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700/60 bg-zinc-900/60 px-4 py-2.5 text-xs text-zinc-400",
                    span { "别名来自真实 /api/models;编辑/删除/定价(输入价·输出价·倍率)已接后端写库;定价模式与按次单价为 UI 层展示态(后端无对应列);新建暂未开放(后端需要 owner/api_key 字段)" }
                }
                // 1. 统计区
                section { id: "aliases-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "别名概览" }
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
                            h2 { class: "text-sm font-medium text-zinc-300", "筛选与操作" }
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
                        h2 { class: "text-lg font-medium text-zinc-100", "别名列表" }
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
                    alias_key: match modal_state() {
                        AliasModalState::Edit(k) => Some(k),
                        _ => None,
                    },
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
                    notice,
                    rows,
                    on_cancel: move |_| modal_state.set(AliasModalState::Closed),
                    on_submit: close_modal,
                }
            }
    }
}

/// 别名卡片
#[component]
fn AliasCard(
    alias_key: String,
    alias: AliasRow,
    index: usize,
    /// 该 card 独立持有的定价模式(per-card,非共享)
    price_mode: PriceMode,
    /// 全部分组(用于展示「哪些分组可用此别名 + 各分组倍率」)
    groups: Vec<GroupDto>,
    /// 各补充通道的启用状态(「基本」tab 胶囊开关控制;未启用通道不出现在卡片)
    c_output_on: bool,
    c_cache_read_on: bool,
    c_cache_write_on: bool,
    c_completion_on: bool,
    on_mode_change: EventHandler<PriceMode>,
    on_edit: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    // 保留参数以维持组件签名,渲染处暂不使用
    let _ = (alias_key, on_edit, on_delete);

    let display_title = if alias.display.is_empty() {
        alias.alias.clone()
    } else {
        alias.display.clone()
    };

    // 分组倍率标签:展示可用此别名的分组及其倍率(白名单语义见 usable_groups_for)。
    let usable_groups = usable_groups_for(&alias.alias, &groups);
    // 卡片空间有限,最多展示 4 个分组标签,超出折叠
    let shown_groups = usable_groups.iter().take(4).collect::<Vec<_>>();
    let overflow_groups = usable_groups.len().saturating_sub(4);

    // 定价:按量 = 输入 + 启用中的补充通道($/1M),按次 = 单项。卡片上放静态默认值
    // 作展示,真实可编辑值在弹窗里(补充通道价格后端无对应列,此处为 UI 占位;
    // 输入价/输出价/倍率走真实写库路径)。未启用的通道(开关关闭)不出现在卡片上。
    let price_rows: Vec<(String, String)> = if price_mode == PriceMode::PerCall {
        vec![("单次调用".into(), "0.05".into())]
    } else {
        let mut rows = vec![("输入".into(), "3".into())];
        if c_output_on {
            rows.push(("输出".into(), "15".into()));
        }
        if c_cache_read_on {
            rows.push(("缓存读取".into(), "0.3".into()));
        }
        if c_cache_write_on {
            rows.push(("缓存写入".into(), "0.75".into()));
        }
        if c_completion_on {
            rows.push(("补全".into(), "2.5".into()));
        }
        rows
    };
    // 单位列:按量统一 $/1M,按次为 USD/次
    let shared_unit = if price_mode == PriceMode::PerCall {
        "USD/次".to_string()
    } else {
        "$/1M".to_string()
    };

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            "data-testid": "alias-card",
            div { class: "space-y-3.5",
                // 头部:别名 + 序号
                div { class: "flex items-center justify-between gap-2",
                    div { class: "min-w-0",
                        h3 { class: "truncate text-sm font-medium text-zinc-100", "{alias.alias}" }
                        if !display_title.is_empty() && display_title != alias.alias {
                            p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{display_title}" }
                        }
                    }
                    span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                        "#{index + 1}"
                    }
                }

                // 分组倍率标签:哪些分组可用此别名 + 各分组倍率(替代原计费倍率进度条)
                div { class: "space-y-1.5",
                    p { class: "text-[11px] text-zinc-400", "可用分组 × 倍率" }
                    div { class: "flex flex-wrap gap-1.5",
                        if shown_groups.is_empty() {
                            span { class: "text-[11px] text-zinc-500", "无分组引用" }
                        } else {
                            for (gname, gratio) in shown_groups {
                                {
                                    let ratio_str = format!("×{gratio:.1}");
                                    let tone = if (*gratio - 1.0).abs() < 0.001 {
                                        "border-zinc-700 bg-zinc-800/80 text-zinc-300"
                                    } else if *gratio < 1.0 {
                                        "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
                                    } else {
                                        "border-amber-500/30 bg-amber-500/10 text-amber-300"
                                    };
                                    rsx! {
                                        span { class: "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] {tone}",
                                            "{gname}"
                                            span { class: "text-[10px] font-mono opacity-70", "{ratio_str}" }
                                        }
                                    }
                                }
                            }
                            if overflow_groups > 0 {
                                span { class: "rounded-full border border-zinc-700 bg-zinc-800/60 px-2 py-0.5 text-[11px] text-zinc-400",
                                    "+{overflow_groups}"
                                }
                            }
                        }
                    }
                }

                // 定价:标题行(左) + 模式 toggle(右),同排;价格列表按启用通道渲染
                div { class: "space-y-2",
                    div { class: "flex items-center justify-between gap-2",
                        p { class: "text-[11px] font-medium text-zinc-400",
                            if price_mode == PriceMode::PerCall { "按次定价" } else { "按量定价" }
                        }
                        PriceModeToggle { active: price_mode, on_change: move |m: PriceMode| on_mode_change.call(m) }
                    }
                    if !price_rows.is_empty() {
                        div { class: "space-y-1",
                            for (label, value) in &price_rows {
                                div { class: "flex items-center justify-between gap-2 text-[11px]",
                                    span { class: "shrink-0 text-zinc-500", "{label}" }
                                    div { class: "flex items-baseline gap-1 font-mono",
                                        span { class: "text-zinc-200", "{value}" }
                                        span { class: "text-[10px] text-zinc-500", "{shared_unit}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 别名新建/编辑弹窗
#[component]
fn AliasFormModal(
    editing: bool,
    alias_key: Option<String>,
    alias: Signal<String>,
    display: Signal<String>,
    input_rate: Signal<String>,
    output_rate: Signal<String>,
    multiplier: Signal<String>,
    price_mode: Signal<PriceMode>,
    active_tab: Signal<usize>,
    p_input: Signal<String>,
    p_output: Signal<String>,
    p_cache_read: Signal<String>,
    p_cache_write: Signal<String>,
    p_completion: Signal<String>,
    p_per_call: Signal<String>,
    c_output_on: Signal<bool>,
    c_cache_read_on: Signal<bool>,
    c_cache_write_on: Signal<bool>,
    c_completion_on: Signal<bool>,
    notice: Signal<Option<String>>,
    /// 页面列表 signal:保存成功后按返回 view 的 key 就地刷新该行
    /// (以服务端落库值为准),签名与 EventHandler 一样 Copy,直接传入。
    rows: Signal<Vec<AliasItem>>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing {
        "编辑模型别名"
    } else {
        "新建模型别名"
    };
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建别名"
    };

    let submitting = use_signal(|| false);
    // 弹窗内 tab 信号:由页面持有(active_tab),弹窗内切换只影响弹窗本体
    let mut modal_tab = active_tab;
    // 定价模式 toggle 的写回调:更新页面级 f_price_mode
    let on_mode_change = {
        let mut pm = price_mode;
        move |mode: PriceMode| pm.set(mode)
    };

    // 工厂式复制,避免把原 signal 移动出闭包(供 rsx 中 submitting() 继续读取)
    let submitting2 = submitting;
    let on_submit2 = on_submit;
    let notice2 = notice;
    let key2 = alias_key.clone();
    let do_submit = move |_| {
        let key = key2.clone();
        let name = alias.peek().trim().to_string();
        if name.is_empty() {
            return;
        }
        let (mut sub, cb, mut note) = (submitting2, on_submit2, notice2);
        match key {
            // 编辑:PUT /api/models/{key}。请求体携带 name + 定价三字段 —
            // 后端 0018 起 UpdateModelRequest 已接三字段(全 Option,
            // COALESCE 合并写库)。三字段从弹窗本地信号解析:非法/空值回退
            // None(字段缺席 = 后端保持现值,不置零)。
            // price_mode 是纯 UI 概念(后端无 pricing_mode 列),保存策略:
            // 无论按量/按次,统一把 input/output/multiplier 三字段发后端,
            // price_mode 只影响卡片与弹窗的展示口径,不参与写库 — 这样
            // 按次定价的单次价格仍只存 UI(p_per_call 信号),而可落库的
            // 定价不与模式耦合,避免后端存半截模式状态。
            Some(k) => {
                spawn(async move {
                    sub.set(true);
                    let client = ApiClient::shared().clone();
                    let req = AliasUpsertRequest {
                        name,
                        input_per_1k: parse_price(&input_rate),
                        output_per_1k: parse_price(&output_rate),
                        multiplier: parse_price(&multiplier),
                        ..Default::default()
                    };
                    match update_model_alias_api(&client, &k, &req).await {
                        // 成功:用返回 view 按 key 就地刷新列表行(以服务端
                        // 落库值为准,不读本地信号,也不整体重拉避免列表闪骨架)。
                        Ok(view) => {
                            let mut items = rows().to_vec();
                            if let Some(it) = items.iter_mut().find(|it| it.key == view.key) {
                                it.row.alias = view.name;
                                it.row.input_per_1k = view.input_per_1k;
                                it.row.output_per_1k = view.output_per_1k;
                                it.row.multiplier = view.multiplier;
                                it.price_mode = price_mode();
                            }
                            rows.set(items);
                            note.set(Some("已保存".to_string()));
                        }
                        Err(e) => note.set(Some(format!("保存失败:{e}"))),
                    }
                    sub.set(false);
                    cb.call(()); // 关闭弹窗(列表行已就地刷新)
                });
            }
            // 新建:后端 CreateModelRequest 必填 owner 与 api_key(非 Option、
            // 无 serde default,validate_model 也强制非空),弹窗表单只收
            // name/展示名/倍率/价格,没有这两者的合法来源 — 诚实拒绝,
            // 不造数据、不假成功。这是「给已有模型加别名」页面的已知缺口,
            // 与定价写库无关(定价三字段已可通过编辑路径落库)。
            None => {
                note.set(Some(
                    "新建未执行:后端创建模型需要 owner 与 api_key 字段,当前表单未提供".to_string(),
                ));
                cb.call(());
            }
        }
    };

    // 弹窗按量 tab:紧凑单列排布 — 主通道(输入)+ 可关闭补充通道,
    // 每行 = 标题+开关 + 价格输入框(单行紧凑);开关控制通道是否启用。
    // 三 tab 内容统一固定高度,切换时弹窗不伸缩。
    let TAB_CONTENT_H: &str = "min-h-[300px]";
    let price_panel = |channel_title: String,
                       channel_desc: String,
                       mut enabled: Signal<bool>,
                       mut price: Signal<String>,
                       testid: String| {
        rsx! {
            div {
                class: "rounded-xl border border-zinc-800 bg-zinc-950/60 px-3 py-2.5",
                div { class: "flex items-center gap-3",
                    // 标题 + 悬停说明(title 属性,不占固定行高)
                    div {
                        class: "shrink-0 w-20",
                        title: "{channel_desc}",
                        p { class: "text-xs font-medium text-zinc-100 truncate", "{channel_title}" }
                    }
                    // 价格输入框
                    div { class: "flex-1 flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-900 px-2.5 py-1.5",
                        span { class: "text-[11px] text-zinc-500", "$" }
                        input {
                            class: "w-full bg-transparent font-mono text-xs text-zinc-100 focus:outline-none",
                            r#type: "text",
                            "data-testid": "{testid}",
                            value: "{price}",
                            oninput: move |e| price.set(e.value()),
                        }
                        span { class: "shrink-0 text-[10px] text-zinc-500", "USD" }
                    }
                    // 开关:点击切换启用状态
                    button {
                        class: "relative h-5 w-9 shrink-0 rounded-full transition-colors cursor-pointer",
                        style: if enabled() { "background-color: #525252" } else { "background-color: #3f3f46" },
                        "data-testid": "{testid}-toggle",
                        role: "switch",
                        aria_checked: "{enabled()}",
                        onclick: move |_| enabled.set(!enabled()),
                        div {
                            class: "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
                            style: if enabled() { "left: 18px" } else { "left: 2px" },
                        }
                    }
                }
            }
        }
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                // 顶部 tab 栏:基本 / 按量定价 / 按次定价
                div {
                    class: "flex w-full overflow-hidden rounded-lg border border-zinc-800 bg-zinc-950 p-0.5 text-xs",
                    role: "tablist",
                    "aria-label": "别名编辑选项",
                    for (i, tab_label) in (["基本", "按量定价", "按次定价"]).into_iter().enumerate() {
                        button {
                            key: "{i}",
                            class: if i == active_tab() {
                                "flex-1 rounded-md bg-zinc-100 px-3 py-2 font-medium text-zinc-900 transition-colors"
                            } else {
                                "flex-1 rounded-md px-3 py-2 text-zinc-400 transition-colors hover:text-zinc-200"
                            },
                            role: "tab",
                            aria_selected: "{i == active_tab()}",
                            "data-testid": "alias-modal-tab-{i}",
                            onclick: move |_| modal_tab.set(i),
                            "{tab_label}"
                        }
                    }
                }

                // ---- tab 0:基本 — 标识 / 展示名 / 倍率 / 定价模式 toggle(与卡片同状态) ----
                if active_tab() == 0 {
                    div { class: "space-y-4 {TAB_CONTENT_H}",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "别名标识 (API 请求匹配名)" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                "data-testid": "alias-name",
                                placeholder: "例如: gpt-4o, claude-3-5-sonnet",
                                value: "{alias}",
                                oninput: move |e| alias.set(e.value()),
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "展示名称 (可选)" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                "data-testid": "alias-display",
                                placeholder: "例如: GPT-4o 旗舰模型",
                                value: "{display}",
                                oninput: move |e| display.set(e.value()),
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "计费倍率 (multiplier ≥ 0)" }
                            input {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                                "data-testid": "alias-multiplier",
                                placeholder: "1.0",
                                value: "{multiplier}",
                                oninput: move |e| multiplier.set(e.value()),
                            }
                        }

                        // 定价模式 toggle:与卡片面板同一状态(非 compact 全宽)
                        div { class: "space-y-1.5",
                            label { class: "block text-xs text-zinc-400", "定价模式 (启用哪种定价)" }
                            PriceModeToggle { active: price_mode(), on_change: on_mode_change, compact: false }
                            p { class: "mt-1 text-[11px] text-zinc-500", "切换到按量定价后,补充通道的启用开关在「按量定价」tab" }
                        }
                    }
                }

                // ---- tab 1:按量定价 — 输入/输出/缓存读取/缓存写入/补全 5 项($/1M) ----
                if active_tab() == 1 {
                    div { class: "space-y-3 {TAB_CONTENT_H}",
                        // 输入价格:主通道,固定开启,不带 toggle
                        div { class: "space-y-1.5",
                            div {
                                label { class: "block text-sm font-medium text-zinc-100", "输入价格" }
                                p { class: "mt-0.5 text-[11px] text-zinc-500", "每 100 万输入 token 的价格。" }
                            }
                            div { class: "flex items-center gap-3 rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-2",
                                span { class: "text-xs text-zinc-500", "$" }
                                input {
                                    class: "w-full bg-transparent font-mono text-sm text-zinc-100 focus:outline-none",
                                    r#type: "text",
                                    "data-testid": "alias-input-price",
                                    value: "{p_input}",
                                    oninput: move |e| p_input.set(e.value()),
                                }
                                span { class: "shrink-0 text-[11px] text-zinc-500", "$/1M" }
                            }
                        }

                        // 补充通道:紧凑单行面板(标题+悬停说明 / 价格框 / 开关)
                        {
                            price_panel(
                                "输出价格".to_string(),
                                "生成内容的输出 token 价格(悬停标题查看)".to_string(),
                                c_output_on,
                                p_output,
                                "alias-output-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                "缓存读取价格".to_string(),
                                "缓存读取 token 价格(悬停标题查看)".to_string(),
                                c_cache_read_on,
                                p_cache_read,
                                "alias-cache-read-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                "缓存写入价格".to_string(),
                                "缓存写入 token 价格(悬停标题查看)".to_string(),
                                c_cache_write_on,
                                p_cache_write,
                                "alias-cache-write-price".to_string(),
                            )
                        }
                        {
                            price_panel(
                                "补全价格".to_string(),
                                "补全(输出)调用的 token 价格(悬停标题查看)".to_string(),
                                c_completion_on,
                                p_completion,
                                "alias-completion-price".to_string(),
                            )
                        }
                    }
                }

                // ---- tab 2:按次定价 — 每次调用固定费用 ----
                if active_tab() == 2 {
                    div { class: "space-y-3 {TAB_CONTENT_H}",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "单次调用价格" }
                            p { class: "text-[11px] text-zinc-500", "每次调用(不论 token 数)固定扣费。" }
                        }
                        div {
                            class: "flex items-center gap-3 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5",
                            span { class: "text-xs text-zinc-500", "$" }
                            input {
                                class: "w-full bg-transparent font-mono text-sm text-zinc-100 focus:outline-none",
                                r#type: "text",
                                "data-testid": "alias-call-price",
                                placeholder: "例如: 0.05",
                                value: "{p_per_call}",
                                oninput: move |e| p_per_call.set(e.value()),
                            }
                            span { class: "shrink-0 text-[11px] text-zinc-500", "USD/次" }
                        }
                        p { class: "text-[11px] text-zinc-500", "后端 models 域无按次计费列,按次价格仅 UI 层生效,不写库。" }
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    "data-testid": "alias-cancel",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    "data-testid": "alias-submit",
                    disabled: submitting(),
                    onclick: do_submit,
                    "{submit_label}"
                }
            }
        }
    }
}

// ============ 定价模式 toggle（共享组件,卡片与弹窗共用） ============

/// 把弹窗定价输入框的字符串解析为 `AliasUpsertRequest` 的 Option<f64> 语义。
///
/// - 空串 / 无法解析 / 非有限(`"NaN"`、`"inf"` 能被 `f64::parse` 接收,
///   但 serde_json 会把 NaN/±inf 序列化成 null)→ **None**:字段缺席,
///   后端 `UpdateModelRequest` 走 COALESCE 保持现值,不置零;
/// - 合法有限值(含负值与越界倍率)→ **Some** 原样发送,非法值由后端
///   `validate_model` 400 拒绝并把错误透出到页面提示条——前端不静默吞、
///   不本地钳制,避免用户以为已保存成功。
///
/// 因此「清空输入框保存」= 该列保持原值,是显式的缺席语义而非写零。
fn parse_price(s: &Signal<String>) -> Option<f64> {
    let raw = s.peek().trim().to_string();
    if raw.is_empty() {
        return None;
    }
    let n: f64 = raw.parse().ok()?;
    n.is_finite().then_some(n)
}

/// 定价模式:按量(Token 计费) / 按次(按调用次数计费)。
/// 纯 UI 概念:后端 models 域无 pricing_mode 列,该枚举只驱动卡片/弹窗的
/// 展示口径;保存时统一发 input/output/multiplier 三字段,见
/// `AliasFormModal` 的提交注释。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PriceMode {
    PerToken,
    PerCall,
}

/// 定价模式 toggle:两个分段胶囊,激活段浅色底。
///
/// - `compact`(默认 true):小号胶囊,用在卡片面板里(不占满,视觉克制)。
/// - 非 compact:全宽,用在编辑弹窗「基本」tab 里。
/// 后端 models 域无 pricing_mode 列,模式仅影响展示,保存路径见
/// `AliasFormModal` 的提交注释。
#[component]
pub fn PriceModeToggle(
    /// 当前激活模式
    active: PriceMode,
    /// 切换回调
    on_change: EventHandler<PriceMode>,
    /// 紧凑模式(卡片用);默认 true
    #[props(default = true)]
    compact: bool,
) -> Element {
    // 容器:略提亮 zinc-800/60 底;激活段用深色高对比底 + 白字(不依赖渐变对比,
    // 避免浅色字在亮底上看不清)。
    let container_cls = if compact {
        "inline-flex items-center rounded-full border border-zinc-700/60 bg-zinc-800/60 p-0.5 text-[11px] shadow-sm"
    } else {
        "flex w-full overflow-hidden rounded-lg border border-zinc-700/60 bg-zinc-800/60 p-0.5 text-xs shadow-sm"
    };
    let active_cls = if compact {
        "rounded-full bg-zinc-100 px-2.5 py-0.5 text-[11px] font-semibold text-zinc-950 shadow-sm transition-colors"
    } else {
        "flex-1 rounded-md bg-zinc-100 px-3 py-1.5 text-center font-semibold text-zinc-950 shadow-sm transition-colors"
    };
    let idle_cls = if compact {
        "rounded-full px-2.5 py-0.5 text-[11px] text-zinc-400 transition-colors hover:text-zinc-200"
    } else {
        "flex-1 rounded-md px-3 py-1.5 text-center text-zinc-400 transition-colors hover:text-zinc-200"
    };
    rsx! {
        div {
            class: "{container_cls}",
            role: "tablist",
            "aria-label": "定价模式",
            button {
                class: if active == PriceMode::PerToken {
                    "{active_cls}"
                } else {
                    "{idle_cls}"
                },
                role: "tab",
                aria_selected: "{active == PriceMode::PerToken}",
                "data-testid": "price-mode-token",
                onclick: move |_| on_change.call(PriceMode::PerToken),
                "按量"
            }
            button {
                class: if active == PriceMode::PerCall {
                    "{active_cls}"
                } else {
                    "{idle_cls}"
                },
                role: "tab",
                aria_selected: "{active == PriceMode::PerCall}",
                "data-testid": "price-mode-call",
                onclick: move |_| on_change.call(PriceMode::PerCall),
                "按次"
            }
        }
    }
}
