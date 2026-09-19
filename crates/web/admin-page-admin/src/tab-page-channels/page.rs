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
    delete_channel_api, get_channel_api, list_channels_api, list_groups_api, set_channel_status_api,
};

use super::list::ChannelsListSection;
use super::modal::ChannelFormModal;
use super::shared::{ChannelModalState, WriteOp, filter_channels};
use super::stats::ChannelsStatsSection;
use super::toolbar::ChannelsToolbarSection;

#[component]
pub fn ChannelsPage() -> Element {
    // —— 列表状态 ——
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut channels = use_signal(Vec::<ChannelDto>::new);
    let mut loading = use_signal(|| true);
    // 后台刷新态:与 `loading` 分离,渲染层不清空列表,只在计数徽标上提示。
    // 合并成一个 loading 会让写操作后的重拉把列表换成占位卡 → 视觉闪烁。
    let mut refreshing = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    // —— 弹窗数据状态 ——
    // 弹窗「绑定分组」候选（真实分组列表,失败留空 → 弹窗内只读回退）。
    let mut group_options = use_signal(Vec::<GroupDto>::new);
    let mut group_err = use_signal(|| None::<String>);
    // 弹窗「当前密钥（掩码）」与「拉取模型」面板状态。掩码值只读展示,
    // 永不进入提交请求体;model_pool 为 (模型 id, 勾选) 候选池。
    let mut existing_keys = use_signal(Vec::<String>::new);
    let mut model_pool = use_signal(Vec::<(String, bool)>::new);
    let mut models_touched = use_signal(|| false);
    let mut fetching_models = use_signal(|| false);

    // —— 筛选状态 ——
    let search = use_signal(String::new);
    let filter_tier = use_signal(|| 0usize);

    // —— 弹窗状态与表单 ——
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
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);

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
        (total.to_string(), "总渠道数"),
        (enabled_count.to_string(), "正常启用"),
        (disabled_count.to_string(), "停用/异常"),
        (total_keys.to_string(), "密钥总数"),
        (group_set.len().to_string(), "绑定分组数"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("启用中 ({enabled_count})"),
        format!("已停用 ({disabled_count})"),
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
                        n.set(Some("操作成功".to_string()));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("操作失败:{e}"))),
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

    // 弹窗关闭并触发重拉
    let close_and_reload = move |_| {
        modal_state.set(ChannelModalState::Closed);
        reload.set(reload() + 1);
    };

    rsx! {
        div { class: "flex flex-col gap-6",
            // 通知条(成功/错误/进行中)
            if let Some(msg) = notice() {
                div { class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
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

            // 卡片网格区(编号段 3):四态(错误/加载/空/网格)+ 新卡示例 + ChannelCard 网格。
            // 数据以值传入(filtered 已在上方按 search/filter_tier 筛好);
            // on_toggle 收 (key, 目标状态 1|2),页面落成 WriteOp::Toggle 走 API。
            ChannelsListSection {
                loading: *loading.read(),
                err: err(),
                filtered,
                on_edit: open_edit,
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
