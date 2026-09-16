//! 卡片式操作按钮组 (ActionButtonGroup)
//!
//! 实体卡片(分组/渠道/别名/兑换码等)底部统一的操作按钮行:
//! 按 tone 着色的等宽按钮, 支持禁用占位与 testid, 与
//! `card`/`form` 同族, 供各实体页复用, 避免每页手抄按钮 class。
//!
//! 用法: 传一组 `ActionSpec` (文本 + 着色 + 是否禁用 + testid),
//! 组件渲染为 `flex gap-1.5 border-t` 等宽按钮行, 每个按钮回调 `on_press(index)`。

use dioxus::prelude::*;

/// 按钮着色基调: 与实体卡底部动作语义对齐。
#[derive(Clone, Copy, PartialEq)]
pub enum ActionTone {
    /// 中性 (编辑/停用 等常规操作)
    Neutral,
    /// 成功 (启用 等)
    Success,
    /// 危险 (删除 等)
    Danger,
    /// 禁用占位 (内置 等不可操作项, 灰显 + not-allowed)
    Disabled,
}

/// 单个操作按钮规格。
#[derive(Clone, PartialEq)]
pub struct ActionSpec {
    /// 按钮文案
    pub label: String,
    /// 着色基调
    pub tone: ActionTone,
    /// 禁用占位 (tone=Disabled 时渲染灰显 not-allowed, 不触发回调)
    pub disabled: bool,
    /// 可选 data-testid (agent 验证用; 空 = 不挂)
    pub testid: String,
}

/// 卡片底部操作按钮组 (等宽, 顶部分隔线)。
///
/// `actions` 顺序即渲染顺序; `on_press` 携带被按下按钮的下标
/// (禁用按钮不触发)。容器挂 `data-testid` 供快照定位。
#[component]
pub fn ActionButtonGroup(
    /// 按钮规格列表 (空列表 = 不渲染)
    actions: Vec<ActionSpec>,
    /// 按下回调, 参数为被按按钮下标
    on_press: EventHandler<usize>,
    /// 容器 data-testid 前缀 (每个按钮挂 `{prefix}-{i}`)
    #[props(default)]
    testid_prefix: String,
) -> Element {
    let class_for = |tone: &ActionTone| match tone {
        ActionTone::Neutral => {
            "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white"
        }
        ActionTone::Success => {
            "flex-1 rounded-lg border border-emerald-700/50 bg-emerald-900/30 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-emerald-800/50 hover:text-emerald-300"
        }
        ActionTone::Danger => {
            "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-zinc-700 hover:text-red-300"
        }
        ActionTone::Disabled => {
            "flex-1 rounded-lg border border-zinc-800 py-1 text-[11px] text-zinc-600 cursor-not-allowed"
        }
    };

    let specs = actions.clone();
    rsx! {
        div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
            "data-testid": "{testid_prefix}",
            for (i, spec) in specs.iter().enumerate() {
                {
                    let idx = i;
                    let disabled = spec.disabled;
                    let tid = if spec.testid.is_empty() {
                        format!("{testid_prefix}-{i}")
                    } else {
                        spec.testid.clone()
                    };
                    rsx! {
                        button {
                            class: "{class_for(&spec.tone)}",
                            "data-testid": "{tid}",
                            disabled,
                            onclick: move |_| on_press.call(idx),
                            "{spec.label}"
                        }
                    }
                }
            }
        }
    }
}
