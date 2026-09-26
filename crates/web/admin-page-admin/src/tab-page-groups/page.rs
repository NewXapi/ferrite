//! 分组管理页入口:卡片式设计,对齐用户管理面板 (UsersPanel) 视觉规范。
//!
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_groups_api`,写入本地
//! `groups` signal;删除走 `delete_group_api`,新建/编辑走
//! `create_group_api` / `update_group_api`(新建/编辑由弹窗内部发起,
//! 见 `modal.rs::GroupFormModal`),启停与倍率分别走 `set_group_status_api` /
//! `update_group_ratio_api`。
//!
//! 本文件只保留状态、派生与写回逻辑,渲染拆给 `stats` / `toolbar` / `list` / `modal`;
//! 边界:不含任何样式细节与卡片内部交互。

use dioxus::prelude::*;

use client::ApiClient;
use contract::api::admin::GroupDto;

use super::list::GroupsList;
use super::modal::GroupFormModal;
use super::shared::{
    LBL_STAT_AVG_RATIO, LBL_STAT_CUSTOM_RATIO, LBL_STAT_DISABLED, LBL_STAT_ENABLED, LBL_STAT_TOTAL,
    MSG_BULK_FAIL_SUFFIX, MSG_BULK_OK, MSG_BULK_OK_SUFFIX, MSG_BULK_PREFIX, MSG_BULK_TAIL,
    MSG_OP_FAILED, MSG_OP_OK, ModalState, OPT_ALL, OPT_DISABLED, OPT_ENABLED, SEC_PAGE_ARIA,
    WriteOp, parse_whitelist,
};
use super::stats::GroupsStatsSection;
use super::toolbar::GroupsToolbar;
use crate::api::{delete_group_api, list_groups_api, set_group_status_api, update_group_ratio_api};

/// 分组管理页。
///
/// 【是什么】分组 tab 的页面入口:一张可滚动页面,自上而下是通知条、统计区(段 1)、
/// 筛选与操作区(段 2)、卡片网格区(段 3),外加按需挂载的新建/编辑弹窗。
///
/// 【做什么】持有本 tab 的全部跨组件状态,挂载即拉取列表,把筛选与统计派生好之后
/// 以值传给三个区段组件,并实现删除 / 启停 / 倍率 / 批量启停 / 新建编辑的写回闭包。
/// 不负责任何视觉细节(§2 硬约束 H2:页面 rsx 只做组装)、不负责卡片内部交互
/// (在 `modal.rs`)、不负责弹窗表单的提交请求(在 `GroupFormModal` 内)。
///
/// 【交互逻辑】用户操作 → 页面行为 → 数据交互:
/// - 挂载 / `reload` 变化 → `use_effect` 调 `list_groups_api` 写入 `groups`,失败置 `err`。
/// - 挂载 → 另一条 `use_effect` 并行调 `list_models_api` 取映射别名候选,失败留空。
/// - 单卡删除 → `delete_group_api`,成功后 `reload + 1` 整页重拉(行要消失)。
/// - 单卡启停 / 拖倍率 → `set_group_status_api` / `update_group_ratio_api`,成功后
///   `with_mut` 就地改本地对应项(不重拉、无闪烁);失败则追加 `reload` 与真值同步。
/// - 批量启停 → 快照 `selected` 顺序 await 每个 key,成功者就地更新,最后汇总通知并清空选中。
/// - 弹窗提交 → `close_and_reload`(关弹窗 + 重拉)。
/// 数据交互:本文件是本 tab 全部网络请求的唯一发起处(除弹窗内的创建/更新)。
///
/// 【样式】根节点 `div.flex.flex-col.gap-6` 加 `role="region"` 与
/// `aria-label=SEC_PAGE_ARIA`;通知条为 `rounded-xl border border-zinc-700 bg-zinc-900
/// px-4 py-2 text-xs text-zinc-300`(进行中追加 `···`)。
///
/// 【子组件组成】`GroupsStatsSection`(段 1)、`GroupsToolbar`(段 2)、
/// `GroupsList`(段 3)、`GroupFormModal`(条件挂载的编辑/新建弹窗)。
///
/// 【数据流】
/// - 对内(入):无 prop(页面级组件,由路由挂载)。
/// - 对外(出):无;状态通过 Signal prop 传给子组件,子组件的 EventHandler 回到本页闭包。
///
/// 状态块:以下 signal 均因**跨组件交互**而提升到本层(§2.2):列表状态被三区段共享;
/// 筛选状态页面要用它算 `filtered`;弹窗状态要跨 toolbar(开)与弹窗(读写)、页面(提交);
/// `selected` 要跨 toolbar(点选)与页面(批量);表单元字段由「打开弹窗」动作初始化。
#[component]
pub fn GroupsPage() -> Element {
    // 列表状态:effect 拉取,stats / toolbar / list 三处共享,故放页面层。
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut groups = use_signal(Vec::<GroupDto>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    // 筛选状态:页面要拿它算 filtered,故提升到页面层,toolbar 就地读写同一份。
    let search = use_signal(String::new);
    let filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| ModalState::Closed);
    // 多选集合 (卡片勾选框); 批量动作只在选中数>0 时可用
    let mut selected = use_signal(Vec::<String>::new);

    // 弹窗表单状态:仅弹窗内部使用,但因为由「打开弹窗」这一跨组件动作初始化
    // (toolbar 的新建 / list 的编辑),故提升到页面层持有,以 Signal prop 传入弹窗。
    // 表单状态
    let mut f_name = use_signal(String::new);
    let mut f_ratio = use_signal(|| "1.0".to_string());
    let mut f_remark = use_signal(String::new);
    // 模型白名单:逗号分隔输入,提交时拆分;后端校验非空字符串数组
    let mut f_whitelist = use_signal(String::new);
    // 映射别名(编辑弹窗 tab2): 逗号分隔; MVP 仅登记
    let mut f_alias = use_signal(String::new);

    // 映射别名候选 (models 域 name 列表); 与分组列表并行拉取, 失败留空。
    // 单独一条 effect(不依赖 reload),故只在这里拉一次。
    let mut alias_options = use_signal(Vec::<String>::new);
    use_effect(move || {
        spawn(async move {
            let client = ApiClient::shared().clone();
            match crate::api::list_models_api(&client).await {
                Ok(v) => alias_options.set(v.into_iter().map(|m| m.name).collect()),
                Err(_) => alias_options.set(Vec::new()),
            }
        });
    });

    // 挂载即拉取真实列表;reload 变化时重拉
    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_groups_api(&client).await {
                Ok(list) => {
                    groups.set(list);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let list = groups();
    let total = list.len();
    let enabled_count = list.iter().filter(|g| g.status == 1).count();
    let disabled_count = list.iter().filter(|g| g.status != 1).count();
    let avg_ratio = if total > 0 {
        list.iter().map(|g| g.ratio).sum::<f64>() / (total as f64)
    } else {
        1.0
    };
    let custom_count = list
        .iter()
        .filter(|g| (g.ratio - 1.0).abs() > 0.001)
        .count();

    let stats: [(String, &str); 5] = [
        (total.to_string(), LBL_STAT_TOTAL),
        (enabled_count.to_string(), LBL_STAT_ENABLED),
        (disabled_count.to_string(), LBL_STAT_DISABLED),
        (format!("{:.2}×", avg_ratio), LBL_STAT_AVG_RATIO),
        (custom_count.to_string(), LBL_STAT_CUSTOM_RATIO),
    ];

    let filter_options = vec![
        format!("{OPT_ALL} ({total})"),
        format!("{OPT_ENABLED} ({enabled_count})"),
        format!("{OPT_DISABLED} ({disabled_count})"),
    ];

    // 过滤列表
    let filtered: Vec<GroupDto> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        list.into_iter()
            .filter(|g| {
                if !q.is_empty()
                    && !g.name.to_lowercase().contains(&q)
                    && !g.remark.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => g.status == 1,
                    2 => g.status != 1,
                    _ => true,
                }
            })
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_ratio.set("1.0".to_string());
        f_remark.set(String::new());
        f_whitelist.set(String::new());
        f_alias.set(String::new());
        modal_state.set(ModalState::New);
    };

    let mut open_edit = move |key: String| {
        if let Some(g) = groups().iter().find(|g| g.key == key) {
            f_name.set(g.name.clone());
            f_ratio.set(format!("{}", g.ratio));
            f_remark.set(g.remark.clone());
            f_whitelist.set(parse_whitelist(&g.model_whitelist).join(", "));
            // MVP: 映射别名暂不落库, 打开时清空; 后端补列后从 g 回填
            f_alias.set(String::new());
            modal_state.set(ModalState::Edit(key));
        }
    };

    // 写操作助手工厂:返回独立闭包,交给卡片(删除/启停/倍率)。
    // 启用/停用与倍率是局部状态变更 — 成功后就地更新本地 groups 里对应项
    // (用 groups.update 原位改, 避免与批量路径整列表 set 互踩丢更新),
    // 不触发整页重拉(reload),避免列表闪烁; 失败时除通知外追加 reload
    // 触发整页重拉, 让 UI 与服务端真值重新同步; 删除必须整页重拉(行消失)。
    let make_write = || {
        let busy_sig = busy;
        let notice_sig = notice;
        let reload_sig = reload;
        let groups_sig = groups;
        move |key: String, op: WriteOp| {
            let (mut b, mut n, mut r, mut g) = (busy_sig, notice_sig, reload_sig, groups_sig);
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                let res = match op {
                    WriteOp::Delete => {
                        let r = delete_group_api(&client, &key).await;
                        r.map(|_| serde_json::json!(true))
                    }
                    WriteOp::ToggleStatus(s) => set_group_status_api(&client, &key, s)
                        .await
                        .map(|_| serde_json::json!(true)),
                    WriteOp::SetRatio(v) => update_group_ratio_api(&client, &key, v)
                        .await
                        .map(|_| serde_json::json!(true)),
                };
                match res {
                    Ok(_) => {
                        n.set(Some(MSG_OP_OK.to_string()));
                        match op {
                            WriteOp::Delete => {
                                // 删除:行要消失,必须整页重拉
                                r.set(r() + 1);
                            }
                            WriteOp::ToggleStatus(s) => {
                                // 启停:就地更新本地 status, 不重拉, 无闪烁;
                                // with_mut 原位改, 不整表 set, 避免与批量路径并发写互踩
                                g.with_mut(|list| {
                                    if let Some(hit) = list.iter_mut().find(|x| x.key == key) {
                                        hit.status = s;
                                    }
                                });
                            }
                            WriteOp::SetRatio(v) => {
                                // 倍率:就地更新本地 ratio, 不重拉, 卡片即时反映
                                g.with_mut(|list| {
                                    if let Some(hit) = list.iter_mut().find(|x| x.key == key) {
                                        hit.ratio = v;
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => {
                        n.set(Some(format!("{MSG_OP_FAILED}{e}")));
                        // 失败也要整页重拉: 本地列表可能已与服务端漂移(如并发
                        // 写冲突/他人改动), 重拉一次与真值重新同步。删除成功
                        // 分支已有重拉, 这里只在失败路径统一追加, 不重复。
                        r.set(r() + 1);
                    }
                }
                b.set(false);
            });
        }
    };
    let write_delete = make_write();
    let write_toggle = make_write();

    // 批量动作执行闭包工厂: 参数为 target status (1=启用, 2=停用), 返回独立的
    // onclick 闭包。不复用 make_write —— 那是逐条 fire-and-forget, 无错误反馈;
    // 这里自包含地顺序 await 每个 key 的状态写回: 进入时快照选中集合(空则直接
    // return), 成功者就地更新本地 groups 对应项(与单发路径同为 with_mut 原位改),
    // 收集失败 key, 结束后一次性汇总通知, 最后无论成败清空选中。
    let make_bulk_toggle = |target: i16| {
        move |_| {
            let keys = selected.peek().clone();
            if keys.is_empty() {
                return;
            }
            let (mut g, mut n, mut sel) = (groups, notice, selected);
            spawn(async move {
                let client = ApiClient::shared().clone();
                let mut ok = 0usize;
                let mut failed: Vec<String> = Vec::new();
                for k in &keys {
                    match set_group_status_api(&client, k, target).await {
                        Ok(_) => {
                            ok += 1;
                            g.with_mut(|list| {
                                if let Some(hit) = list.iter_mut().find(|x| &x.key == k) {
                                    hit.status = target;
                                }
                            });
                        }
                        Err(_) => failed.push(k.clone()),
                    }
                }
                let msg = if failed.is_empty() {
                    MSG_BULK_OK.to_string()
                } else {
                    // 失败 key 截断到前 3 个, 更长以 … 结尾提示
                    let shown = failed
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                    let more = if failed.len() > 3 { "…" } else { "" };
                    format!(
                        "{MSG_BULK_PREFIX}{ok}{MSG_BULK_OK_SUFFIX}{}{MSG_BULK_FAIL_SUFFIX}{shown}{more}{MSG_BULK_TAIL}",
                        failed.len()
                    )
                };
                n.set(Some(msg));
                sel.set(Vec::new());
            });
        }
    };
    // 两个批量按钮各自持有独立执行闭包实例(工厂按 target 生成的两份)
    let bulk_enable = make_bulk_toggle(1);
    let bulk_disable = make_bulk_toggle(2);

    // 弹窗关闭并触发重拉
    let close_and_reload = move |_| {
        modal_state.set(ModalState::Closed);
        reload.set(reload() + 1);
    };

    // 卡片写回统一入口: 删除走 write_delete, 启停/倍率走 write_toggle
    // (两个闭包由同一工厂生成, 行为一致; 拆开保持调用语义清晰)。
    let on_write = move |(key, op): (String, WriteOp)| match op {
        WriteOp::Delete => write_delete(key, WriteOp::Delete),
        WriteOp::SetRatio(v) => write_toggle(key, WriteOp::SetRatio(v)),
        WriteOp::ToggleStatus(s) => write_toggle(key, WriteOp::ToggleStatus(s)),
    };

    rsx! {
        div { class: "flex flex-col gap-6",
            role: "region",
            "aria-label": SEC_PAGE_ARIA,
            // 通知条(成功/错误/进行中)
            if let Some(msg) = notice() {
                div { class: "rounded-xl border {ui::T_border_zinc_700} {ui::T_bg_zinc_900} px-4 py-2 {ui::T_text_xs} {ui::T_text_zinc_300}",
                    "{msg}"
                    if busy() { " ···" }
                }
            }

            // 统计区(编号段 1):五张概览卡(总数/启用/停用/白名单/默认分组)。
            // 纯渲染,stats 由上方派生块算好传入;组件零状态,见 stats.rs。
            GroupsStatsSection { stats: stats.to_vec() }

            // 筛选与操作区(编号段 2):搜索 / 分级胶囊 / 批量点选与动作条。
            // search/filter_tier/selected 以 Signal 绑定(页面要算 filtered 并批量写);
            // on_refresh/on_new/on_bulk_* 都是跨组件交互,由页面闭包处理。
            GroupsToolbar {
                groups: groups(),
                filter_options,
                search,
                filter_tier,
                selected,
                on_refresh: move |_| reload.set(reload() + 1),
                on_new: open_new,
                on_bulk_enable: bulk_enable,
                on_bulk_disable: bulk_disable,
                on_bulk_clear: move |_| selected.set(Vec::new()),
            }

            // 卡片网格区(编号段 3):四态(错误/加载/空/网格)+ 首卡示例 + GroupCard 网格。
            // 数据以值传入(filtered 已在上方筛好);on_write 把卡片操作落成
            // WriteOp 走 API,on_edit 开编辑弹窗,均为跨组件交互。
            GroupsList {
                filtered,
                loading: loading(),
                err: err(),
                on_edit: move |key: String| open_edit(key),
                on_write,
                on_retry: move |_| reload.set(reload() + 1),
            }
        }

        // 新建 / 编辑弹窗
        if matches!(modal_state(), ModalState::New | ModalState::Edit(_)) {
            GroupFormModal {
                editing: matches!(modal_state(), ModalState::Edit(_)),
                group_key: match modal_state() {
                    ModalState::Edit(k) => Some(k),
                    _ => None,
                },
                name: f_name,
                ratio: f_ratio,
                remark: f_remark,
                whitelist: f_whitelist,
                alias_options: alias_options(),
                f_alias,
                on_cancel: move |_| modal_state.set(ModalState::Closed),
                on_submit: close_and_reload,
            }
        }
    }
}
