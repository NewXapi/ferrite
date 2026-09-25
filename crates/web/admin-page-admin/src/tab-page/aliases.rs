//! 模型别名管理面板 —— 统计 / 筛选 / 别名卡片三区,组合式编排。
//! 后端接口:GET /api/models?size=100, PUT /api/models/{key}, DELETE /api/models/{key}
//!
//! 本文件只放「状态 + 拉取 effect + 区段组件组合」(spec R1: 页面 rsx 元素嵌套 ≤1 层):
//! - 区段: `crate::components::{AliasesStatsSection, AliasesToolbarSection, AliasesListSection}`
//! - 浮层: `crate::components::AliasModal` (字段状态已按 R2 沉入组件内)
//! - 文案常量在根级 `crate::shared`,格式化助手在根级 `crate::format`
//!
//! 状态归属约定(页面层持有的都是跨组件交互的):
//! - 列表状态(rows/loading/err/reload/groups):effect 拉取 + 子组件共享
//! - 筛选状态(search/filter_tier):页面算 filtered,toolbar 就地读写
//! - 弹窗状态(modal_state):由页面闭包控制,子组件根据 prop 只读
//! - 写回状态(busy/notice):通知条与子组件共享
//! - 弹窗表单状态:组件内部处理,由页面闭包控制表单重置
//!
//! 刷新时机：首次加载、create/update/delete 成功后统一调用 load_aliases()。
//!
//! 边界:本目录只服务别名这一个 tab;别名卡片的**视觉外壳**定义在
//! ui-components 的 `admin_card`,这里不复制卡片样式。
//! 表格编辑与验证逻辑留给组件内部处理；网络写回全部留在本文件，
//! 子组件只暴露 `EventHandler`;组件内不出现 `spawn` 请求。

use client::ApiClient;
use contract::api::admin::{GroupDto, ModelAliasView, UpdateModelRequest};
use dioxus::prelude::*;

use crate::api::{list_model_aliases_api, update_model_alias_api, delete_model_alias_api};
use crate::components::{AliasModal, AliasesListSection, AliasesStatsSection, AliasesToolbarSection};
use crate::format::fmt_cny;
use crate::shared::{BTN_REFRESH, MSG_LOAD_FAILED, MSG_LOADING_LIST, MSG_EMPTY, MSG_DELETED, MSG_SAVE_FAILED, MSG_DELETE_FAILED, MSG_CREATE_REJECTED};
use crate::state::{parse_groups_whitelist, parse_keys_input};

/// 拉取当前用户的模型别名列表 (GET /api/models?size=100) 并写回状态。
/// 首次加载与 create/update/delete 成功后刷新共用此入口。
fn load_aliases(
    mut rows: Signal<Vec<crate::state::AliasItem>>, 
    mut loaded: Signal<bool>, 
    mut err: Signal<String>, 
    mut reload: Signal<u32>
) {
    // 列表数据与错误：子组件共享,故放页面层
    let client = ApiClient::shared().clone();
    let rows_tx = rows.clone();
    let loaded_tx = loaded.clone();
    let err_tx = err.clone();

    spawn(async move {
        match list_model_aliases_api(&client).await {
            Ok(data) => {
                // 转换为页面专用的 AliasItem
                let items: Vec<crate::state::AliasItem> = data.into_iter().map(|dto| {
                    crate::state::AliasItem {
                        key: dto.key,
                        name: dto.name,
                        display_name: dto.display_name.unwrap_or_default(),
                        multiplier: dto.multiplier,
                        input_price: 0, // 后端无此列,设 0
                        output_price: 0, // 后端无此列,设 0
                        cache_read_price: 0,
                        cache_write_price: 0,
                        completion_price: 0,
                        price_mode: crate::state::PriceMode::Standard,
                        enabled: dto.enabled.unwrap_or(true),
                    }
                }).collect();
                rows_tx.set(items);
                loaded_tx.set(true);
            }
            Err(e) => {
                err_tx.set(format!("加载失败: {}", e));
            }
        }
    });
}

#[derive(Clone, PartialEq)]
enum ModalState {
    Closed,
    New,
    Edit(String), // 编辑的 key
}

#[component]
pub fn AliasesPanel() -> Element {
    // 别名管理页专有状态:跨组件交互
    let mut rows = use_signal(Vec::<crate::state::AliasItem>::new);
    let mut rows_loaded = use_signal(|| false);
    let mut rows_err = use_signal(String::new);
    let mut reload_counter = use_signal(|| 0u32);
    
    let mut modal_state = use_signal(|| ModalState::Closed);
    let mut notice = use_signal(String::new);
    let mut busy = use_signal(|| false);
    
    // 列表数据与错误：子组件共享,故放页面层
    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize); // 0=全部, 1=标准, 2=自定, 3=免费
    
    // 初始化加载
    use_effect(move || {
        load_aliases(rows, rows_loaded, rows_err, reload_counter);
    });
    
    // 计算过滤后的列表
    let filtered_rows = use_memo(move || {
        let rows = rows();
        let search_term = search().to_lowercase();
        let tier = filter_tier();
        
        rows.into_iter().filter(|item| {
            if search_term.is_empty() {
                true
            } else {
                item.name.to_lowercase().contains(&search_term) || 
                item.display_name.to_lowercase().contains(&search_term)
            }
        }).collect()
    });
    
    // 统计数据派生
    let stats = use_memo(move || {
        let rows = rows();
        let mut total = 0;
        let mut standard = 0;
        let mut custom = 0;
        let mut free = 0;
        let mut avg_multiplier = 0.0;
        
        if !rows.is_empty() {
            total = rows.len() as i64;
            for item in &rows {
                match item.price_mode {
                    crate::state::PriceMode::Standard => standard += 1,
                    crate::state::PriceMode::PerToken => custom += 1,
                    crate::state::PriceMode::PerCall => free += 1,
                }
                avg_multiplier += item.multiplier;
            }
            avg_multiplier /= rows.len() as f64;
        }
        
        vec![
            (format!("{}（共 {} 个）", crate::shared::LBL_STAT_TOTAL, total), crate::shared::LBL_STAT_TOTAL),
            (format!("{}（{} 个）", crate::shared::LBL_STAT_STANDARD, standard), crate::shared::LBL_STAT_STANDARD),
            (format!("{}（{} 个）", crate::shared::LBL_STAT_CUSTOM, custom), crate::shared::LBL_STAT_CUSTOM),
            (format!("{}（{} 个）", crate::shared::LBL_STAT_FREE, free), crate::shared::LBL_STAT_FREE),
            (format!("{}：{:.2}×", crate::shared::LBL_STAT_AVG, avg_multiplier), crate::shared::LBL_STAT_AVG),
        ]
    });
    
    // 筛选胶囊文案派生
    let filter_options = use_memo(move || {
        let rows = rows();
        let mut total = rows.len();
        let mut standard = 0;
        let mut custom = 0;
        let mut free = 0;
        
        for item in &rows {
            match item.price_mode {
                crate::state::PriceMode::Standard => standard += 1,
                crate::state::PriceMode::PerToken => custom += 1,
                crate::state::PriceMode::PerCall => free += 1,
            }
        }
        
        let mut options = Vec::new();
        options.push(format!("{}（{}）", crate::shared::OPT_ALL, total));
        options.push(format!("{}（{}）", crate::shared::OPT_STANDARD, standard));
        options.push(format!("{}（{}）", crate::shared::OPT_CUSTOM, custom));
        options.push(format!("{}（{}）", crate::shared::OPT_FREE, free));
        options
    });
    
    // Modal 操作辅助函数
    fn open_new(mut modal_state: Signal<ModalState>) {
        modal_state.set(ModalState::New);
    }
    
    fn close_modal(mut modal_state: Signal<ModalState>) {
        modal_state.set(ModalState::Closed);
    }
    
    rsx! {
        section { class: "flex flex-col gap-6",
            // 通知条
            if !notice().is_empty() {
                div { class: "rounded-xl border-zinc-700 bg-zinc-900 p-4 text-white",
                    "{notice()}"
                }
            }
            
            // 别名概览统计区
            AliasesStatsSection { stats: stats() }
            
            // 筛选与操作区
            AliasesToolbarSection {
                search,
                filter_tier,
                reload_counter,
                filter_options: filter_options(),
                on_new: move |_| open_new(modal_state),
            }
            
            // 别名列表
            if rows_loaded() {
                if rows_err().is_empty() {
                    if filtered_rows().is_empty() {
                        div { class: "text-center py-8 text-zinc-400", "{crate::shared::MSG_EMPTY}" }
                    } else {
                        AliasesListSection { rows: filtered_rows() }
                    }
                } else {
                    div { class: "text-center py-4 text-red-400", "{rows_err()}" }
                }
            } else {
                div { class: "text-center py-4 text-zinc-400", "{crate::shared::MSG_LOADING_LIST}" }
            }
            
            // 新建/编辑弹窗
            match modal_state() {
                ModalState::Closed => rsx! {},
                ModalState::New => rsx! {
                    AliasModal { 
                        mode: crate::state::ModalMode::New,
                        on_close: move |_| close_modal(modal_state),
                    }
                },
                ModalState::Edit(key) => rsx! {
                    AliasModal {
                        mode: crate::state::ModalMode::Edit(key),
                        on_close: move |_| close_modal(modal_state),
                    }
                },
            }
        }
    }
}