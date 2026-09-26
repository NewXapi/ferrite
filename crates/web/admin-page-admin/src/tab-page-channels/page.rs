//! 渠道管理页:卡片式网格,对齐 GroupsPage / UsersPanel 规范。
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_channels_api`,写入本地
//! `channels` signal;启用/停用走 `set_channel_status_api`,删除走
//! `delete_channel_api`,新建/编辑走 `create_channel_api` / `update_channel_api`。
//! 编辑保存用最小 diff 体(`UpdateChannelBody`):密钥框留空 = 不触碰现有密钥,
//! `models` 仅在用户动过「拉取模型」面板时携带,priority/weight 不随请求发出、
//! 由后端 COALESCE 保持。
//! 拓扑测速、批量分组、快速导入是 mock 期的纯前端特性,已移除(后端暂无对应接口)。
//!
//! 本文件只保留状态、拉取与写回逻辑(submit / open_edit / toggle / delete)
//! 以及组件组合;渲染拆成 `stats`(概览统计)、`toolbar`(筛选与操作)、
//! `list`(卡片网格)、`card`(单卡)、`modal`(弹窗),共享类型见 `shared`。
//!
//! 状态归属约定(页面层持有的都是跨组件交互的):
//! - 列表状态(channels/loading/refreshing/err/reload):effect 拉取 + 三组件共享
//! - 筛选状态(search/filter_tier):页面算 filtered,toolbar 就地读写
//! - 弹窗表单状态(f_*/existing_keys/model_pool/models_touched/fetching_models/
//!   group_options/group_err):open_new/open_edit 重置 → 弹窗读写,跨组件
//! - 写回状态(busy/notice):通知条与 make_write 闭包共享

use dioxus::prelude::*;

use client::ApiClient;
use contract::api::admin::{ChannelDto, GroupDto};

use crate::api::{
    UpdateChannelBody, delete_channel_api, get_channel_api, list_channels_api, list_groups_api,
    set_channel_status_api, update_channel_api,
};

use super::list::ChannelsListSection;
use super::modal::ChannelFormModal;
use super::shared::{
    ChannelModalState, LBL_STAT_DISABLED, LBL_STAT_ENABLED, LBL_STAT_GROUPS, LBL_STAT_KEYS,
    LBL_STAT_TOTAL, MSG_NAME_REQUIRED, MSG_OP_FAILED, MSG_OP_OK, MSG_SAVE_FAILED, OPT_ALL,
    OPT_DISABLED, OPT_ENABLED, WriteOp, filter_channels,
};
use super::stats::ChannelsStatsSection;
use super::toolbar::ChannelsToolbarSection;

/// 渠道管理页
///
/// 【是什么】渠道 tab 的页面入口组件,持有跨组件状态并薄组装三段区(统计/筛选/列表)与弹窗。
///
/// 【做什么】负责渠道列表与分组候选的拉取、按条件派生 filtered 与统计、以及全部写回
/// (启停 / 删除 / 弹窗提交的落点);不负责任何视觉细节 —— 渲染交给 `stats` / `toolbar` /
/// `list` / `card` / `modal`(本文件的 rsx 只有组装)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 首屏/`reload` 变化 → `use_effect`:`channels.peek()` 判空决定 `loading`(首屏,
///   清列表渲染占位)还是 `refreshing`(已有数据,旧列表留屏避免闪烁,用 `peek` 是为了
///   不让本 effect 订阅自己写入的信号而重拉死循环),随后 `list_channels_api`。
/// - 同一 `reload` 还触发第二个 effect 拉分组候选(`list_groups_api`),失败只记
///   `group_err`,弹窗回退为只读展示当前分组。
/// - 搜索/切档 → toolbar 就地写 `search` / `filter_tier`,本文件调 `filter_channels` 重算。
/// - 卡片启停 / 删除 → 走 `make_write` 工厂生成的闭包:`set_channel_status_api` /
///   `delete_channel_api`,成功后置 `MSG_OP_OK` 并 `reload + 1`。
/// - 点卡片「编辑」→ `open_edit` 回填表单并按需拉掩码密钥(`get_channel_api`)。
/// - 弹窗保存 → 由弹窗自己发创建/更新请求,成功回到 `close_and_reload`(关弹窗 + 重拉)。
///
/// 【样式】顶层 `div.flex flex-col gap-6`;通知条 `rounded-xl border-border
/// bg-card`。页面自身不写卡片/网格样式。
///
/// 【子组件组成】`ChannelsStatsSection` / `ChannelsToolbarSection` / `ChannelsListSection`,
/// 条件渲染时挂载 `ChannelFormModal`;`ChannelCard` 由 list 区内部使用。
///
/// 【数据流】
/// - 对内(入):无 prop —— 页面组件不接收外部参数。
/// - 对外(出):把上面的 Signal 与派生值以 prop 下发;子组件则通过 Signal 就地读写或
///   `EventHandler`(开弹窗 / 编辑 / 启停 / 删除 / 重试)把意图抛回本文件的写回闭包,
///   由本文件统一落成 API 调用与 `channels` / `notice` 更新。
#[component]
pub fn ChannelsPage() -> Element {
    // 列表状态:被 effect、list 区与 count 徽标共用,故放页面层。
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut channels = use_signal(Vec::<ChannelDto>::new);
    let mut loading = use_signal(|| true);
    // 后台刷新态:与 `loading` 分离,渲染层不清空列表,只在计数徽标上提示。
    // 合并成一个 loading 会让写操作后的重拉把列表换成占位卡 → 视觉闪烁。
    let mut refreshing = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    // 弹窗数据状态:只被弹窗消费,但由 `open_edit` 与列表 effect 写入 ——
    // 写入方在页面层,故随之提升到这里,以 Signal 注入弹窗(组件内不自持)。
    // 弹窗「绑定分组」候选（真实分组列表,失败留空 → 弹窗内只读回退）。
    let mut group_options = use_signal(Vec::<GroupDto>::new);
    let mut group_err = use_signal(|| None::<String>);
    // 弹窗「当前密钥（掩码）」与「拉取模型」面板状态。掩码值只读展示,
    // 永不进入提交请求体;model_pool 为 (模型 id, 勾选) 候选池。
    let mut existing_keys = use_signal(Vec::<String>::new);
    let mut model_pool = use_signal(Vec::<(String, bool)>::new);
    let mut models_touched = use_signal(|| false);
    let mut fetching_models = use_signal(|| false);

    // 筛选状态:toolbar 就地读写,但 filtered 由页面调用 `filter_channels` 计算,
    // 所以状态提升到这一层、以 Signal 传入 toolbar。
    let search = use_signal(String::new);
    let filter_tier = use_signal(|| 0usize);

    // 弹窗开关与表单状态:弹窗状态决定挂载与否;`f_*` 由 `open_new` / `open_edit`
    // 成组重置、弹窗就地读写 —— 初值写入方在页面,故整组放页面层。
    let mut modal_state = use_signal(|| ChannelModalState::Closed);

    // 编辑/新建表单状态
    let mut f_name = use_signal(String::new);
    let mut f_ctype = use_signal(|| "openai".to_string());
    let mut f_url = use_signal(String::new);
    let mut f_keys = use_signal(String::new);
    let mut f_group = use_signal(Vec::<String>::new);
    let mut f_remark = use_signal(String::new);
    // 测速模型：弹窗无编辑控件，但后端 test_model 列是 SQL 直绑（无 COALESCE），
    // 编辑保存必须原样回传现值，缺席即被清成 NULL。编辑打开时从列表行带入。
    let mut f_test_model = use_signal(|| None::<String>);

    // —— 写回状态 ——
    // 写操作进行中 / 成功提示:由 `make_write` 生成的两个闭包共同写、被通知条读取,
    // 跨组件共享,故放页面层。
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<String>);

    // 挂载即拉取真实列表;reload 变化时重拉。
    // 首屏(列表为空)走 `loading` → 渲染占位卡;已有数据的重拉走 `refreshing`
    // → 旧列表留在屏上,避免写操作后整块消失再出现的闪烁。
    // `peek()` 读列表长度:普通 `channels()` 会让本 effect 订阅自己写入的信号 → 重拉死循环。
    use_effect(move || {
        let _ = reload();
        let first_load = channels.peek().is_empty();
        if first_load {
            loading.set(true);
        } else {
            refreshing.set(true);
        }
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_channels_api(&client).await {
                Ok(list) => channels.set(list),
                // 后台刷新失败也走 err:错误态优先于陈旧列表,避免用户对着过期数据操作
                Err(e) => err.set(Some(e.to_string())),
            }
            loading.set(false);
            refreshing.set(false);
        });
    });

    // 分组候选:弹窗「绑定分组」多选用。与列表并行拉取,失败只记 err 文案,
    // 不阻断弹窗——弹窗在候选为空时回退为只读展示当前分组值(见 ChannelFormModal)。
    use_effect(move || {
        let _ = reload();
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_groups_api(&client).await {
                Ok(list) => {
                    group_options.set(list);
                    group_err.set(None);
                }
                Err(e) => group_err.set(Some(e.to_string())),
            }
        });
    });

    // —— 派生:统计与筛选(filtered 被 list 组件消费,计算留在页面)——
    let list = channels();
    let total = list.len();
    let enabled_count = list.iter().filter(|c| c.status == 1).count();
    let disabled_count = list.iter().filter(|c| c.status != 1).count();
    let total_keys: i64 = list.iter().map(|c| c.key_count).sum();
    let group_set: std::collections::BTreeSet<String> =
        list.iter().flat_map(|c| c.groups.iter().cloned()).collect();

    let stats: [(String, &str); 5] = [
        (total.to_string(), LBL_STAT_TOTAL),
        (enabled_count.to_string(), LBL_STAT_ENABLED),
        (disabled_count.to_string(), LBL_STAT_DISABLED),
        (total_keys.to_string(), LBL_STAT_KEYS),
        (group_set.len().to_string(), LBL_STAT_GROUPS),
    ];

    let filter_options = vec![
        format!("{OPT_ALL} ({total})"),
        format!("{OPT_ENABLED} ({enabled_count})"),
        format!("{OPT_DISABLED} ({disabled_count})"),
    ];

    // 筛选纯函数:抽取为模块级 `filter_channels`,便于纯函数单测
    let filtered = filter_channels(&list, &search(), filter_tier());

    // —— 写回闭包 ——
    let open_new = move |_| {
        f_name.set(String::new());
        f_ctype.set("openai".to_string());
        f_url.set("https://api.openai.com/v1".to_string());
        f_keys.set(String::new());
        f_group.set(vec!["default".to_string()]);
        existing_keys.set(Vec::new());
        model_pool.set(Vec::new());
        models_touched.set(false);
        fetching_models.set(false);
        f_remark.set(String::new());
        f_test_model.set(None);
        modal_state.set(ChannelModalState::New);
    };

    let open_edit = move |key: String| {
        if let Some(c) = channels().iter().find(|c| c.key == key) {
            f_name.set(c.name.clone());
            f_ctype.set(c.channel_type.clone());
            f_url.set(c.base_url.clone());
            // 密钥编辑框留空：不回显掩码，最小 diff 语义是「留空 = 不改动现有密钥」。
            // 当前密钥的掩码值单独只读展示（单查接口带回），让维护者知道配置了什么。
            f_keys.set(String::new());
            f_group.set(c.groups.clone());
            f_remark.set(c.remark.clone());
            f_test_model.set(c.test_model.clone());
            // 模型候选池预填渠道现有 models（字符串或 {alias} 对象，取别名），
            // 全部勾选 = 现状；未动面板则 models 字段缺席（COALESCE 保持）。
            let existing: Vec<String> = c
                .models
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| {
                            m.as_str().map(|s| s.to_string()).or_else(|| {
                                m.get("alias")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            model_pool.set(existing.iter().cloned().map(|id| (id, true)).collect());
            models_touched.set(false);
            fetching_models.set(false);
            existing_keys.set(Vec::new());
            modal_state.set(ChannelModalState::Edit(key.clone()));
            // 掩码密钥与分组候选并行拉取（失败静默：掩码区隐藏/候选区只读回退）
            spawn(async move {
                if let Ok(dto) = get_channel_api(&ApiClient::shared().clone(), &key).await
                    && let Some(ks) = dto.keys
                {
                    existing_keys.set(ks);
                }
            });
        }
    };

    // 写操作助手工厂:返回独立闭包,分别交给卡片(启停/删除)。
    // Signal 是 Copy;每次调用先复制一份再 move 进 async,避免把闭包捕获的
    // signal 移动出去(FnMut 不允许)。
    let make_write = || {
        let busy_sig = busy;
        let notice_sig = notice;
        let reload_sig = reload;
        move |key: String, op: WriteOp| {
            let (mut b, mut n, mut r) = (busy_sig, notice_sig, reload_sig);
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                let res = match op {
                    WriteOp::Toggle(status) => set_channel_status_api(&client, &key, status).await,
                    WriteOp::Delete => delete_channel_api(&client, &key).await,
                };
                match res {
                    Ok(_) => {
                        n.set(Some(MSG_OP_OK.to_string()));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("{MSG_OP_FAILED}{e}"))),
                }
                b.set(false);
            });
        }
    };
    let write_toggle = make_write();
    let write_delete = make_write();

    // 卡片启停/删除:把 (key, 目标状态) 落成 WriteOp 走 make_write 通道
    let on_toggle_card = move |pair: (String, i16)| write_toggle(pair.0, WriteOp::Toggle(pair.1));
    let on_delete_card = move |key: String| write_delete(key, WriteOp::Delete);

    // 行内 Popover 保存(UI 决策记录 §2.2):单字段走 UpdateChannelBody 最小 diff
    // PUT(name/channel_type/base_url/groups/remark/test_model 恒带现值,被改字段
    // 换成新值;keys/models 缺席 = 后端保持现值)。priority/weight/status 该 body
    // 不收(COALESCE 保持),启停走专用 status 端点,均不在此处理。
    let commit_channel_field =
        move |(key, field, value): (String, ui::ChannelEditField, String)| {
            let raw = value.trim().to_string();
            let Some(current) = channels().iter().find(|c| c.key == key).cloned() else {
                return;
            };
            // 名称为空 = 放弃这次保存(不发请求,保留旧值)
            if matches!(field, ui::ChannelEditField::Name) && raw.is_empty() {
                notice.set(Some(MSG_NAME_REQUIRED.to_string()));
                return;
            }
            let body = UpdateChannelBody {
                name: match field {
                    ui::ChannelEditField::Name if !raw.is_empty() => raw.clone(),
                    _ => current.name.clone(),
                },
                channel_type: current.channel_type.clone(),
                base_url: match field {
                    ui::ChannelEditField::BaseUrl => raw.clone(),
                    _ => current.base_url.clone(),
                },
                groups: current.groups.clone(),
                remark: match field {
                    ui::ChannelEditField::Remark => raw.clone(),
                    _ => current.remark.clone(),
                },
                test_model: match field {
                    ui::ChannelEditField::TestModel => {
                        if raw.is_empty() {
                            None
                        } else {
                            Some(raw.clone())
                        }
                    }
                    _ => current.test_model.clone(),
                },
                keys: None,
                models: None,
            };
            let (mut b, mut n) = (busy, notice);
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                match update_channel_api(&client, &key, &body).await {
                    Ok(updated) => {
                        // 就地替换该行(后端返回更新后的完整 DTO),不整体重拉
                        let mut list = channels().to_vec();
                        if let Some(slot) = list.iter_mut().find(|c| c.key == key) {
                            *slot = updated;
                        }
                        channels.set(list);
                    }
                    Err(e) => n.set(Some(format!("{MSG_SAVE_FAILED}{e}"))),
                }
                b.set(false);
            });
        };

    // 弹窗关闭并触发重拉
    let close_and_reload = move |_| {
        modal_state.set(ChannelModalState::Closed);
        reload.set(reload() + 1);
    };

    rsx! {
        div { class: "flex flex-col gap-6",
            // 通知条(成功/错误/进行中)
            if let Some(msg) = notice() {
                div { class: "rounded-xl border border-border bg-card px-4 py-2 {ui::TYPE_DESC}",
                    "{msg}"
                    if busy() { " ···" }
                }
            }

            // 统计区(编号段 1):五张概览卡(总数/启用/停用/密钥/分组)。
            // 纯渲染,stats 由上方派生块算好传入;组件零状态,见 stats.rs。
            ChannelsStatsSection { stats: stats.to_vec() }

            // 筛选与操作区(编号段 2):刷新/新建按钮 + 搜索框 + 状态胶囊。
            // search/filter_tier/reload 以 Signal 绑定 —— 页面要拿它们算 filtered
            // 并触发重拉,组件就地读写同一份状态;on_new 开弹窗属跨组件交互,页面闭包。
            ChannelsToolbarSection {
                search,
                filter_tier,
                reload,
                filter_options,
                on_new: open_new,
            }

            // 卡片网格区(编号段 3):四态(错误/加载/空/网格)+ 新卡牌网格。
            // 数据以值传入(filtered 已在上方按 search/filter_tier 筛好);
            // on_edit 收 (key, 字段, 原始值) 走最小 diff PUT;on_full_edit 开弹窗
            // (密钥/模型/分组等多字段能力只在弹窗);on_toggle 收 (key, 目标状态 1|2)。
            ChannelsListSection {
                loading: *loading.read(),
                err: err(),
                filtered,
                on_edit: commit_channel_field,
                on_full_edit: open_edit,
                on_toggle: on_toggle_card,
                on_delete: on_delete_card,
                on_retry: move |_| reload.set(reload() + 1),
            }
        }

        // 新建 / 编辑弹窗
        if matches!(modal_state(), ChannelModalState::New | ChannelModalState::Edit(_)) {
            ChannelFormModal {
                editing: matches!(modal_state(), ChannelModalState::Edit(_)),
                channel_key: match modal_state() {
                    ChannelModalState::Edit(k) => Some(k),
                    _ => None,
                },
                name: f_name,
                ctype: f_ctype,
                url: f_url,
                keys: f_keys,
                group: f_group,
                group_options: group_options,
                group_err: group_err,
                existing_keys: existing_keys,
                model_pool: model_pool,
                models_touched: models_touched,
                fetching_models: fetching_models,
                remark: f_remark,
                test_model: f_test_model,
                on_cancel: move |_| modal_state.set(ChannelModalState::Closed),
                on_submit: close_and_reload,
            }
        }
    }
}
