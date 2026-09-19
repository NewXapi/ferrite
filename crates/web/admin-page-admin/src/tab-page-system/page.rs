//! 系统页:组合系统诊断区与代理节点区。
//! - 拉取 `/api/system-info`(admin-ops system_info),派生三组渲染数据:
//!   概览统计卡、实体统计卡、运行环境明细行
//! - 挂载三个面板:`SystemOptionsPanel`(站点选项)、`ProxyNodesPanel`
//!   (出口代理节点导入)、`ProxyRuntimePanel`(代理节点运行态)
//! - 所有数据来自后端实时采集;后端不提供的站点配置/功能开关字段已移除,
//!   不再使用 EntityStore mock 假数据。
//!
//! 编号段 1-3(概览统计/实体统计/运行环境)共享同一数据源,整体抽成
//! `overview` 组件,stats/count_cards/env_rows 派生随渲染搬入组件内部;
//! DTO 与格式化辅助见 `shared`,面板实现见各自文件。

use dioxus::prelude::*;

use super::options::SystemOptionsPanel;
use super::overview::SystemOverview;
use super::proxy_nodes::ProxyNodesPanel;
use super::proxy_runtime::ProxyRuntimePanel;
use super::shared::SystemInfoView;
use client::ApiClient;

/// 系统页:顶部运行指标 + 实体统计 + 运行环境明细,数据来自 `/api/system-info`。
#[component]
pub fn SystemPage() -> Element {
    // —— 列表状态(data/loading/err 被 overview 组件消费,reload 跨组件)——
    let mut info = use_signal(|| None::<SystemInfoView>);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        // reload 变化(首帧或点击刷新)触发重新拉取
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared();
            match client.get::<SystemInfoView>("/api/system-info").await {
                Ok(v) => {
                    info.set(Some(v));
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "flex flex-col gap-6",

            // 系统概览区(编号段 1-3):概览统计卡 + 实体统计卡 + 运行环境明细行。
            // 三段共享同一数据源(SystemInfoView),抽成一个组件;
            // data/loading/err 由页面持有(拉取 effect + 刷新跨组件),值 + 事件传入。
            SystemOverview {
                data: info(),
                loading: *loading.read(),
                err: err(),
                on_refresh: move |_| reload.set(reload() + 1),
            }

            // 4. 站点选项(key/value 平表,真实 /api/option 读写)
            SystemOptionsPanel {}

            // 5. 出口代理节点导入面板 (M2-C)
            ProxyNodesPanel {}

            // 5. 代理节点运行态面板 (M3,消费 GET /api/proxy_nodes/report)
            ProxyRuntimePanel {}
        }
    }
}
