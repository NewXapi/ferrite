//! 分组管理页:卡片式设计,对齐用户管理面板 (UsersPanel) 视觉规范。
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_groups_api`,写入本地
//! `groups` signal;删除走 `delete_group_api`,新建/编辑走
//! `create_group_api` / `update_group_api`。
//!
use dioxus::prelude::*;

use client::ApiClient;
use contract::api::admin::GroupDto;

use super::list::GroupsList;
use super::modal::GroupFormModal;
use super::shared::{ModalState, WriteOp, parse_whitelist};
use super::stats::GroupsStatsSection;
use super::toolbar::GroupsToolbar;
use crate::api::{delete_group_api, list_groups_api, set_group_status_api, update_group_ratio_api};

#[component]
pub fn GroupsPage() -> Element {
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut groups = use_signal(Vec::<GroupDto>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    let search = use_signal(String::new);
    let filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| ModalState::Closed);
    // 多选集合 (卡片勾选框); 批量动作只在选中数>0 时可用
    let mut selected = use_signal(Vec::<String>::new);

    // 表单状态
    let mut f_name = use_signal(String::new);
    let mut f_ratio = use_signal(|| "1.0".to_string());
    let mut f_remark = use_signal(String::new);
    // 模型白名单:逗号分隔输入,提交时拆分;后端校验非空字符串数组
    let mut f_whitelist = use_signal(String::new);
    // 映射别名(编辑弹窗 tab2): 逗号分隔; MVP 仅登记
    let mut f_alias = use_signal(String::new);

    // 映射别名候选 (models 域 name 列表); 与分组列表并行拉取, 失败留空
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
        (total.to_string(), "总分组数"),
        (enabled_count.to_string(), "启用中"),
        (disabled_count.to_string(), "已停用"),
        (format!("{:.2}×", avg_ratio), "平均倍率"),
        (custom_count.to_string(), "非基准倍率"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("启用中 ({enabled_count})"),
        format!("已停用 ({disabled_count})"),
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
                        n.set(Some("操作成功".to_string()));
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
                        n.set(Some(format!("操作失败:{e}")));
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
                    "批量操作成功".to_string()
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
                        "批量操作：{ok} 成功，{} 失败（key: {shown}{more}）",
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
            "aria-label": "分组管理",
            // 通知条(成功/错误/进行中)
            if let Some(msg) = notice() {
                div { class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
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
