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
//! 本文件只保留状态与写回逻辑;渲染拆成 `stats`(统计区)、`toolbar`
//! (筛选与操作区)、`list`(卡片网格区)、`card`(卡片)、`modal`
//! (新建/编辑弹窗),共享类型与判定见 `shared`。
//!
//! 状态归属约定(页面层持有的都是跨组件交互的):
//! - 列表状态(rows/loading/err/reload/groups):effect 拉取 + stats/list 组件共享
//! - 筛选状态(search/filter_tier):页面算 filtered,toolbar 就地读写
//! - 弹窗表单状态(f_*/p_*/c_*):open_new/open_edit 重置 → 弹窗读写 →
//!   卡片读 c_*(通道开关),跨三处,必须放页面层
//! - 写回状态(busy/notice/submitting):通知条与各写回闭包共享

use client::ApiClient;
use contract::api::admin::GroupDto;
use contract::api::billing::AliasUpsertRequest;
use dioxus::prelude::*;

use super::list::AliasesListSection;
use super::modal::AliasFormModal;
use super::shared::{
    AliasItem, AliasModalState, LBL_STAT_AVG, LBL_STAT_CUSTOM, LBL_STAT_FREE, LBL_STAT_STANDARD,
    LBL_STAT_TOTAL, MSG_CREATE_REJECTED, MSG_DELETE_FAILED, MSG_DELETED, MSG_SAVE_FAILED, OPT_ALL,
    OPT_CUSTOM, OPT_FREE, OPT_STANDARD, PriceMode, SEC_DATA_NOTE,
};
use super::stats::AliasesStatsSection;
use super::toolbar::AliasesToolbarSection;
use crate::api::{
    delete_model_alias_api, list_groups_api, list_model_aliases_api, update_model_alias_api,
};
use crate::state::AliasRow;
/// 别名管理页
///
/// 【是什么】别名 tab 的页面入口组件,持有跨组件状态并薄组装三段区(统计/筛选/列表)与弹窗。
///
/// 【做什么】负责列表拉取(GET /api/models?size=100)与分组拉取、按条件派生 filtered
/// 与统计、以及全部写回(编辑 PUT / 删除 DELETE / 新建诚实拒绝);不负责任何视觉细节
/// —— 渲染交给 `stats` / `toolbar` / `list` / `modal` 四个子模块(本文件的 rsx 只有组装)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互(网络请求都发生在本文件):
/// - 首屏/`reload` 变化 → `use_effect` 触发:先拉分组(`list_groups_api`,失败不阻塞),
///   再拉别名列表(`list_model_aliases_api`),把结果映射为 `AliasItem` 并按别名排序后写 `rows`。
/// - 搜索/切档 → toolbar 就地写 `search` / `filter_tier`,本文件重算 `filtered`(无网络)。
/// - 卡片切定价模式 → 写回 `rows` 中该条的 `price_mode`(纯 UI 本地状态,不发网络)。
/// - 提交弹窗 → 编辑走 `update_model_alias_api`(PUT,请求体只带 `name`);新建不造数据,
///   置 `notice = MSG_CREATE_REJECTED` 诚实拒绝。
/// - 请求删除 → `delete_model_alias_api`(DELETE),成功后本地从 `rows` 移除,
///   不整体重拉,避免列表闪 loading 骨架与高度跳动。
///
/// 【样式】顶层 `div.flex flex-col gap-6`;通知条为 `rounded-xl border-zinc-700
/// bg-zinc-900`,数据来源说明条为 `bg-zinc-900/60` 的窄横幅。页面自身不写卡片/网格样式。
///
/// 【子组件组成】`AliasesStatsSection` / `AliasesToolbarSection` / `AliasesListSection`,
/// 条件渲染时挂载 `AliasFormModal`;`AliasCard` 由 list 区内部使用。
///
/// 【数据流】
/// - 对内(入):无 prop —— 页面组件不接收外部参数。
/// - 对外(出):把上面各 Signal 以 prop 下发给子组件;子组件则通过 Signal 就地读写
///   或 `EventHandler` 回调(开弹窗 / 编辑 / 删除 / 重试 / 切模式)把意图抛回本文件的
///   写回闭包,由本文件统一落成 API 调用与 `rows` / `notice` 更新。
#[component]
pub fn AliasesPage() -> Element {
    // 列表状态:跨 stats/list 两区与 effect 共享,故放页面层持有。
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut rows = use_signal(Vec::<AliasItem>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);
    // 分组列表(用于卡片展示「哪些分组可用此别名 + 各分组倍率」)
    let mut groups = use_signal(Vec::<GroupDto>::new);

    // 筛选状态:由 toolbar 就地读写,但 filtered 的派生计算在页面,
    // 所以状态提升到这一层、以 Signal 传入 toolbar(组件内零 use_signal)。
    let search = use_signal(String::new);
    let filter_tier = use_signal(|| 0usize);

    // —— 弹窗表单状态(被 open_new/open_edit 重置,弹窗读写,卡片读 c_*)——
    // 这组状态只服务弹窗本体,但由「打开弹窗」这一跨组件动作初始化(open_new /
    // open_edit 要成组重置),且通道开关 c_* 还会被卡片读到,跨越三处 ——
    // 因此不能下沉进弹窗内部,统一提升到页面层持有,以 Signal prop 注入。
    let mut modal_state = use_signal(|| AliasModalState::Closed);
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

    // —— 写回状态 ——
    // 写操作进行中 / 成功提示:由删除与提交两个写回闭包共同写、被通知条读取,
    // 跨组件共享,故放页面层。submitting 独立于 busy,只用于禁用弹窗提交按钮。
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);
    // 弹窗提交进行中(独立于删除的 busy,只禁用弹窗提交按钮)
    let submitting = use_signal(|| false);

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

    // —— 派生:统计与筛选(filtered 被 list 组件消费,计算留在页面)——
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
        (total.to_string(), LBL_STAT_TOTAL),
        (standard_count.to_string(), LBL_STAT_STANDARD),
        (custom_count.to_string(), LBL_STAT_CUSTOM),
        (free_count.to_string(), LBL_STAT_FREE),
        (format!("{:.2}×", avg_mult), LBL_STAT_AVG),
    ];

    let filter_options = vec![
        format!("{OPT_ALL} ({total})"),
        format!("{OPT_STANDARD} ({standard_count})"),
        format!("{OPT_CUSTOM} ({custom_count})"),
        format!("{OPT_FREE} ({free_count})"),
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

    // —— 写回闭包 ——
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
    // 列表组件把 (key, mode) 二元组抛上来,这里单点写回。
    let on_mode_change = move |(key, mode): (String, PriceMode)| {
        let mut items = rows().to_vec();
        if let Some(it) = items.iter_mut().find(|it| it.key == key) {
            it.price_mode = mode;
        }
        rows.set(items);
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
                    n.set(Some(MSG_DELETED.to_string()));
                }
                Err(e) => n.set(Some(format!("{MSG_DELETE_FAILED}{e}"))),
            }
            b.set(false);
        });
    };

    // 弹窗提交:校验 + 网络写回都在本闭包里,弹窗组件只抛事件(见
    // modal)。编辑走真实 PUT /api/models/{key},新建走诚实拒绝。
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
                        n.set(Some(format!("{MSG_SAVE_FAILED}{e}")));
                    }
                    sub.set(false);
                    ms.set(AliasModalState::Closed);
                });
            }
            // 新建:后端 CreateModelRequest 必填 owner 与 api_key,表单没有
            // 这两个字段的来源 — 诚实拒绝,不造数据、不假成功。
            None => {
                notice.set(Some(MSG_CREATE_REJECTED.to_string()));
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
                span { "{SEC_DATA_NOTE}" }
            }

            // 统计区(编号段 1):五张概览卡(总数/标准 1.0×/自定倍率/免费/平均倍率)。
            // 纯渲染,stats 由上方派生块算好传入;组件零状态,见 stats.rs。
            AliasesStatsSection { stats: stats.to_vec() }

            // 筛选与操作区(编号段 2):刷新/新建按钮 + 搜索框 + 分级胶囊。
            // search/filter_tier/reload 以 Signal 绑定 —— 页面要拿它们算 filtered
            // 并触发重拉,组件就地读写同一份状态;on_new 开弹窗属跨组件交互,页面闭包。
            AliasesToolbarSection {
                search,
                filter_tier,
                reload,
                filter_options,
                on_new: open_new,
            }

            // 卡片网格区(编号段 3):四态(错误/加载/空/网格)+ 新卡示例 + AliasCard 网格。
            // 数据以值传入(filtered 已在上方按 search/filter_tier 筛好);
            // on_mode_change 收 (key, mode) 单点写回 rows,消除 per-card 闭包工厂。
            AliasesListSection {
                loading: *loading.read(),
                err: err(),
                filtered,
                groups: groups.read().clone(),
                c_output_on: c_output_on(),
                c_cache_read_on: c_cache_read_on(),
                c_cache_write_on: c_cache_write_on(),
                c_completion_on: c_completion_on(),
                on_mode_change,
                on_edit: open_edit,
                on_delete: write_delete,
                on_retry: move |_| reload.set(reload() + 1),
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
