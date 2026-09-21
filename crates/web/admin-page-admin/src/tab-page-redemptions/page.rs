//! 兑换码管理页:卡片式网格,对齐 GroupsPage / ChannelsPage / UsersPanel 规范。
//! 数据来自真实后端 `/api/redemption`(列表 / 批量生成 / 停用)。
//! 后端语义:核销 (status→2)、DELETE 即停用 (status→3);无硬删、无重新启用;
//! 明文码只在生成响应里出现一次,页面用弹窗展示。
//!
//! 本文件只保留状态与写回逻辑(拉取 / `commit_generate` / `disable_red`);
//! 渲染拆成 `stats`(概览统计)、`toolbar`(筛选与操作)、`list`(卡片网格)、
//! `card`(卡片)、`modal`(生成弹窗 / 明文码展示),共享类型与映射见 `shared`。
//!
//! 状态归属约定(页面层持有的都是跨组件交互的):
//! - 列表状态(reds/loading/err/reload):effect 拉取 + 三组件共享
//! - 筛选状态(search/filter_tier):页面算 filtered,toolbar 就地读写
//! - 弹窗状态(modal_state/f_count/f_quota):toolbar 的生成按钮开弹窗并预填
//!   表单 → 弹窗读写 → commit_generate 提交,跨三处,必须放页面层
//! - copied_key(复制高亮)是 list 组件内部视觉状态,已下沉进组件

use client::ApiClient;
use dioxus::prelude::*;

use crate::api::{disable_redemption_api, generate_redemptions_api, list_redemptions_api};

use super::list::RedemptionsListSection;
use super::modal::{GeneratedCodesModal, RedemptionGenerateModal};
use super::shared::{
    LBL_STAT_AVAILABLE, LBL_STAT_DISABLED, LBL_STAT_TOTAL, LBL_STAT_UNUSED, LBL_STAT_USED, OPT_ALL,
    OPT_DISABLED, OPT_UNUSED, OPT_USED, RedModalState, RedRowFE, map_redemption_view,
};
use super::stats::RedemptionsStatsSection;
use super::toolbar::RedemptionsToolbarSection;

/// 兑换码管理页。
///
/// 【是什么】兑换码 tab 的页面入口:自上而下是统计区(段 1)、筛选与操作区(段 2)、
/// 卡片网格区(段 3),外加按 `RedModalState` 三选一挂载的弹窗(生成表单 / 明文码展示 / 无)。
///
/// 【做什么】持有本 tab 的全部跨组件状态,挂载即拉取列表,把统计与筛选派生好之后以值
/// 传给三个区段组件,并实现停用与生成两个写回闭包。不负责任何视觉细节(§2 硬约束 H2)、
/// 不负责卡片内部状态(复制高亮在 `list.rs`)、不负责弹窗表单交互(在 `modal.rs`)。
///
/// 【交互逻辑】用户操作 → 页面行为 → 数据交互:
/// - 挂载 / `reload` 变化 → `use_effect` 调 `list_redemptions_api`(第 1 页 100 条),
///   成功映射为 `RedRowFE` 写入 `reds`,失败写 `err`。
/// - 点卡片「停用」→ `disable_red`:调 `disable_redemption_api`,成功 `reload + 1`
///   (行状态要变),失败写 `err`。
/// - 点「生成兑换码」→ `open_generate`:重置 `f_count`/`f_quota` 后开生成弹窗。
/// - 弹窗提交 → `commit_generate`:clamp 数量到 1–100、把 ¥ 面额换算成后端内部单位
///   (×500000),调 `generate_redemptions_api`,成功切到明文码弹窗,失败写 `err`。
/// 数据交互:本文件是本 tab 全部网络请求的唯一发起处。
///
/// 【样式】根节点 `div.flex.flex-col.gap-6`(无额外装饰,子区段自带卡片外壳);
/// 三个区段由各自组件提供独立外壳与 `id`。
///
/// 【子组件组成】`RedemptionsStatsSection`(段 1)、`RedemptionsToolbarSection`(段 2)、
/// `RedemptionsListSection`(段 3)、`RedemptionGenerateModal` / `GeneratedCodesModal`
/// (按弹窗状态二选一挂载)。
///
/// 【数据流】
/// - 对内(入):无 prop(页面级组件,由路由挂载)。
/// - 对外(出):无;状态通过 Signal prop / 值 prop 下发,子组件事件回到本页闭包。
///
/// 状态块:以下三组 signal 因**跨组件交互**而提升到本层(§2.2)——
/// 列表状态被三个区段共享;筛选状态页面要用它算 `filtered_rows`;弹窗状态要跨
/// toolbar(开)与弹窗(读写)、页面(提交)三处;`copied_key` 则相反,已下沉进 `list.rs`。
#[component]
pub fn RedemptionsPage() -> Element {
    // —— 列表状态:e_effect 拉取 + stats/toolbar/list 三组件共享 ——
    let mut reds = use_signal(Vec::<RedRowFE>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    // —— 筛选状态:页面要据此算 filtered_rows,toolbar 就地读写同一份 ——
    let search = use_signal(String::new);
    let filter_tier = use_signal(|| 0usize);

    // —— 弹窗状态与表单:由 toolbar 的生成按钮开弹窗并预填,弹窗读写,
    //    页面 commit_generate 提交并切换状态,跨三处,必须放页面层 ——
    let mut modal_state = use_signal(|| RedModalState::Closed);
    // 生成弹窗字段(后端只支持 面额+数量;活动名/有效期无对应字段)
    let mut f_count = use_signal(|| "1".to_string());
    let mut f_quota = use_signal(|| "50".to_string());

    // 拉取(挂载/刷新/写回后)
    use_effect(move || {
        let _ = reload();
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_redemptions_api(&client, None, Some(1), Some(100)).await {
                Ok((items, _total)) => {
                    reds.set(items.into_iter().map(map_redemption_view).collect());
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    // —— 派生:统计与筛选(filtered_rows 被 list 组件消费,计算留在页面)——
    let red_list = reds.read().clone();
    let total = red_list.len();
    let unused_count = red_list.iter().filter(|r| r.status == 1).count();
    // 2=已核销 / 3=已停用(对齐后端 admin-billing/redeem.rs 写库口径)
    let used_count = red_list.iter().filter(|r| r.status == 2).count();
    let disabled_count = red_list.iter().filter(|r| r.status == 3).count();

    let total_quota: f64 = red_list.iter().map(|r| r.quota_cny).sum();
    let available_quota: f64 = red_list
        .iter()
        .filter(|r| r.status == 1)
        .map(|r| r.quota_cny)
        .sum();

    let stats: [(String, &str); 5] = [
        (total.to_string(), LBL_STAT_TOTAL),
        (unused_count.to_string(), LBL_STAT_UNUSED),
        (used_count.to_string(), LBL_STAT_USED),
        (disabled_count.to_string(), LBL_STAT_DISABLED),
        (format!("¥{:.0}", available_quota), LBL_STAT_AVAILABLE),
    ];
    let _ = total_quota;

    let filter_options = vec![
        format!("{OPT_ALL} ({total})"),
        format!("{OPT_UNUSED} ({unused_count})"),
        format!("{OPT_USED} ({used_count})"),
        format!("{OPT_DISABLED} ({disabled_count})"),
    ];

    let filtered_rows: Vec<RedRowFE> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        red_list
            .into_iter()
            .filter(|r| {
                if !q.is_empty() && !r.code_preview.to_lowercase().contains(&q) {
                    return false;
                }
                // tier 与 filter_options 一一对应:1=未使用 / 2=已核销 / 3=已停用
                match tier {
                    1 => r.status == 1,
                    2 => r.status == 2,
                    3 => r.status == 3,
                    _ => true,
                }
            })
            .collect()
    };

    // —— 写回闭包 ——
    let err_for_effect = err;
    let reload_for_effect = reload;

    let disable_red = move |key: String| {
        let mut err = err_for_effect;
        let mut reload = reload_for_effect;
        spawn(async move {
            let client = ApiClient::shared().clone();
            match disable_redemption_api(&client, &key).await {
                Ok(_) => reload.set(reload() + 1),
                Err(e) => {
                    err.set(Some(e.to_string()));
                }
            }
        });
    };

    let commit_generate = move |_| {
        let cnt = f_count
            .peek()
            .trim()
            .parse::<u32>()
            .unwrap_or(1)
            .clamp(1, 100);
        // 后端 quota 是内部计费单位(500000 = ¥1);表单输入的是 ¥
        let q_cny = f_quota
            .peek()
            .trim()
            .parse::<f64>()
            .unwrap_or(10.0)
            .max(0.0);
        let quota_units = (q_cny * 500_000.0) as i64;
        if quota_units <= 0 {
            return;
        }

        let mut modal_state = modal_state;
        let mut err = err;
        let mut reload = reload;
        spawn(async move {
            let client = ApiClient::shared().clone();
            match generate_redemptions_api(&client, quota_units, cnt).await {
                Ok(codes) => {
                    reload.set(reload() + 1);
                    modal_state.set(RedModalState::Codes(codes));
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                }
            }
        });
    };

    // toolbar 的生成按钮:预填默认面额/数量后开弹窗(原编号段 2 内联逻辑)
    let open_generate = move |_| {
        f_count.set("1".to_string());
        f_quota.set("50".to_string());
        modal_state.set(RedModalState::Generate);
    };

    rsx! {
        div { class: "flex flex-col gap-6",
            // 统计区(编号段 1):五张概览卡(总数/未使用/已核销/已停用/可用面额)。
            // 纯渲染,stats 由上方派生块算好传入;组件零状态,见 stats.rs。
            RedemptionsStatsSection { stats: stats.to_vec() }

            // 筛选与操作区(编号段 2):生成按钮 + 搜索框 + 状态胶囊。
            // search/filter_tier 以 Signal 绑定 —— 页面要拿它们算 filtered_rows,
            // 组件就地读写同一份状态;on_generate 预填表单并开弹窗,属跨组件交互。
            RedemptionsToolbarSection {
                search,
                filter_tier,
                reload,
                filter_options,
                on_generate: open_generate,
            }

            // 卡片网格区(编号段 3):四态(错误/加载/空/网格)+ 新卡示例 + RedemptionCard 网格。
            // 数据以值传入(filtered_rows 已在上方筛好);on_disable 走停用 API;
            // 复制高亮(copied_key)是组件内部视觉状态,不下沉到页面。
            RedemptionsListSection {
                loading: *loading.read(),
                err: err(),
                filtered_rows,
                on_disable: disable_red,
                on_retry: move |_| reload.set(reload() + 1),
            }
        }

        // 生成弹窗 / 明文码展示
        match modal_state() {
            RedModalState::Generate => rsx! {
                RedemptionGenerateModal {
                    count: f_count,
                    quota: f_quota,
                    on_cancel: move |_| modal_state.set(RedModalState::Closed),
                    on_submit: commit_generate,
                }
            },
            RedModalState::Codes(codes) => rsx! {
                GeneratedCodesModal { codes,
                    on_close: move |_| modal_state.set(RedModalState::Closed),
                }
            },
            RedModalState::Closed => rsx! {},
        }
    }
}
