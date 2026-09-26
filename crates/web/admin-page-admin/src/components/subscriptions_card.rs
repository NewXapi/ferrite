//! 订阅套餐卡片:单张套餐的概览(标识徽标 + 标题 + 状态/分组徽标 + 开关 +
//! 编辑/删除 + 五格指标条)。
//!
//! 纯展示组件:数据以值传入(`PlanRow`),编辑 / 启停 / 删除事件通过
//! `on_edit` / `on_toggle` / `on_delete` 抛回页面(传入 plans 列表中的下标),
//! 写回逻辑(真实后端调用)在 `page` 的 `SubscriptionsPage` 里。

use dioxus::prelude::*;

use crate::shared::{
    BTN_DELETE, BTN_EDIT, LBL_DISABLED, LBL_ENABLED, LBL_GROUP_PREFIX, LBL_NO_LIMIT, LBL_PERIOD,
    LBL_PRICE, LBL_QUOTA, LBL_UNLIMITED, ToggleSwitch,
};
use crate::state::PlanRow;

/// 订阅套餐卡片
///
/// 【是什么】单张订阅套餐的概览卡:标识徽标 + 标题 + 状态/分组徽标 + 开关 +
/// 编辑/删除 + 五格指标条(价格 / 有效期 / 套餐额度 / 升级分组 / 限购)。
///
/// 【做什么】按传入的 `PlanRow` 值渲染一张卡;不负责写回(启停/编辑/删除
/// 均由回调抛回页面)、不负责列表容器与弹窗。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点开关 `ToggleSwitch` → `on_toggle.call(index)`;点「编辑」→
///   `on_edit.call(index)`;点「✕」→ `on_delete.call(index)`。
///   三个回调都只带本卡下标,由页面决定改 `plans` 哪一行(页面按下标取行,
///   再以行的 `key` 调真实后端端点)。
/// - 价格/额度/有效期等派生串(如 `quota <= 0` 时显示「无限制」)在渲染前
///   算好,纯展示。
/// 数据交互:本组件**不发任何网络请求**。
///
/// 【标识徽标】后端无 new-api 数字 id 列(`PlanRow.id` 恒为 `None`),徽标
/// 改用 UUID 前缀(行 `key` 的前 8 字符),保证每行有稳定可辨识的标识;
/// hover 时 `title` 显示完整 key。
///
/// 【样式】外壳用共用样式壳 `ui::CardShell`(`CARD_SHELL_CLASS`:根节点
/// `role="region"` + `aria-label="{套餐标题}"` + `data-testid="subscription-card"`);
/// 头部 `flex flex-wrap items-start justify-between gap-2.5`;状态徽标按 `enabled`
/// 切绿/灰两套圆角 pill;
/// 指标条 `grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-3 pt-3
/// border-t border-border/70`(手机 2 / sm 3 / md 5 列)。
///
/// 【子组件组成】`ToggleSwitch`(启停开关,来自 `shared.rs`);其余为原生元素。
///
/// 【数据流】
/// - 对内(入):`plan`(该行 `PlanRow` 全量数据,页面 `plans` signal 中一行
///   的克隆)、`index`(在页面 `plans` 列表中的下标,回调原样回传)。
/// - 对外(出):`on_edit(index)` → 页面 `open_edit`(开弹窗回填);
///   `on_toggle(index)` → 页面以同一 `name` 重建 upsert 体并取反 `enabled`;
///   `on_delete(index)` → 页面按行 `key` 调 DELETE 端点。
#[component]
pub fn PlanCard(
    plan: PlanRow,
    /// 在页面 plans 列表中的下标,回调原样回传
    index: usize,
    on_edit: EventHandler<usize>,
    on_toggle: EventHandler<usize>,
    on_delete: EventHandler<usize>,
) -> Element {
    // 预构建每行所需 owned 值:onclick 闭包在 rsx 构造之后才触发,
    // 那时 plans.read() 的借用 guard 已释放,闭包只能捕获 owned 数据。
    let title_txt = plan.title.clone();
    let price_sym = if plan.currency == "USD" { "$" } else { "¥" };
    let price_str = format!("{price_sym}{:.2}", plan.price);
    // 后端无 new-api 数字 id 列(恒 None):徽标改用 UUID 前缀。
    let badge_txt = plan
        .id
        .map(|n| format!("#{n}"))
        .unwrap_or_else(|| plan.key.chars().take(8).collect());
    let period_str = format!("{} 天", plan.period_val);
    let quota_str = if plan.quota <= 0.0 {
        LBL_UNLIMITED.to_string()
    } else {
        format!("{}", plan.quota)
    };
    let group_txt = if plan.group.is_empty() {
        "不升级".to_string()
    } else {
        plan.group.clone()
    };
    let limit_txt = if plan.max_per_user > 0 {
        format!("{}", plan.max_per_user)
    } else {
        LBL_NO_LIMIT.to_string()
    };
    let cur_enabled = plan.enabled;
    let row_key = plan.key.clone();
    let edit_id = format!("subscriptions-edit-{}", plan.key);
    let del_id = format!("subscriptions-delete-{}", plan.key);

    rsx! {
        ui::CardShell {
            title: title_txt.clone(),
            testid: Some("subscription-card".to_string()),

            // 卡片头部行: 标识 + 标题 + 状态/分组徽标 + 操作按钮
            div { class: "flex flex-wrap items-start justify-between gap-2.5",
                div { class: "flex items-center gap-2.5 min-w-0 flex-1",
                    span { class: "shrink-0 rounded-md border border-border/80 bg-secondary px-2 py-0.5 text-xs font-mono font-bold text-foreground",
                        title: "{row_key}",
                        "{badge_txt}"
                    }
                    h3 { class: "truncate {ui::TYPE_TITLE}", "{title_txt}" }
                    span {
                        class: if cur_enabled { "rounded-full border border-emerald-500/30 bg-success px-2.5 py-0.5 text-[11px] font-medium text-success-foreground" } else { "rounded-full border border-border bg-secondary/80 px-2.5 py-0.5 text-[11px] font-medium text-muted-foreground" },
                        if cur_enabled { "{LBL_ENABLED}" } else { "{LBL_DISABLED}" }
                    }
                    if !plan.group.is_empty() && plan.group != "不升级" {
                        span { class: "rounded-full border border-sky-500/30 bg-info px-2.5 py-0.5 text-[11px] font-medium {ui::C_INFO} uppercase",
                            "{LBL_GROUP_PREFIX}{plan.group}"
                        }
                    }
                }
                div { class: "flex items-center gap-2 shrink-0",
                    ToggleSwitch {
                        on: cur_enabled,
                        on_toggle: move |_| on_toggle.call(index),
                    }
                    button {
                        class: "rounded-lg border border-border bg-secondary px-2.5 py-1 {ui::TYPE_DESC} transition-colors hover:bg-secondary hover:text-foreground",
                        "data-testid": edit_id,
                        onclick: move |_| on_edit.call(index),
                        "{BTN_EDIT}"
                    }
                    button {
                        class: "rounded-lg border border-destructive bg-destructive px-2 py-1 {ui::TYPE_DESC} {ui::C_DANGER} transition-colors hover:bg-destructive hover:text-destructive",
                        "data-testid": del_id,
                        onclick: move |_| on_delete.call(index),
                        "{BTN_DELETE}"
                    }
                }
            }

            // 关键指标条 — 只展示后端实际返回的字段
            div { class: "mt-3.5 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-3 pt-3 border-t border-border/70 {ui::TYPE_DESC}",
                div {
                    span { class: "{ui::TYPE_LABEL} block", "{LBL_PRICE}" }
                    span { class: "font-mono font-bold text-sm {ui::C_SUCCESS}", "{price_str}" }
                }
                div {
                    span { class: "{ui::TYPE_LABEL} block", "{LBL_PERIOD}" }
                    span { class: "font-medium text-foreground", "{period_str}" }
                }
                div {
                    span { class: "{ui::TYPE_LABEL} block", "{LBL_QUOTA}" }
                    span { class: "font-mono font-semibold text-warning-foreground", "{quota_str}" }
                }
                div {
                    span { class: "{ui::TYPE_LABEL} block", "升级分组" }
                    span { class: "text-foreground font-medium", "{group_txt}" }
                }
                div {
                    span { class: "{ui::TYPE_LABEL} block", "限购" }
                    span { class: "{ui::C_MUTED}", "{limit_txt}" }
                }
            }
        }
    }
}
