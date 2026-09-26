//! 卡片式操作按钮组 (ActionButtonGroup)
//!
//! 实体卡片(分组/渠道/别名/兑换码等)底部统一的操作按钮行:
//! 按 tone 着色的等宽按钮, 支持禁用占位与可选 testid(None = 不挂),
//! 与 `card`/`form` 同族, 供各实体页复用, 避免每页手抄按钮 class。
//!
//! 用法: 传一组 `ActionSpec` (文本 + 着色 + 是否禁用 + 可选 testid,
//! None = 不挂), 组件渲染为 `flex gap-1.5 border-t` 等宽按钮行,
//! 每个按钮回调 `on_press(index)`; 禁用按钮不触发回调。

use dioxus::prelude::*;

/// 按钮着色基调: 与实体卡底部动作语义对齐。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionTone {
    /// 中性 (编辑/停用 等常规操作)
    Neutral,
    /// 成功 (启用 等)
    Success,
    /// 柔和成功 (锌底 + 翠绿文字,如「充值」:正向但非主操作)
    SuccessSoft,
    /// 警示 (琥珀色,如「停用/启用」切换类动作)
    Warning,
    /// 危险 (删除 等)
    Danger,
    /// 禁用占位 (内置 等不可操作项, 灰显 + not-allowed)
    Disabled,
}

/// 单个操作按钮规格。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ActionSpec {
    /// 按钮文案
    pub label: String,
    /// 着色基调
    pub tone: ActionTone,
    /// 禁用占位 (tone=Disabled 时渲染灰显 not-allowed; 禁用时不触发回调)
    pub disabled: bool,
    /// 可选 data-testid (agent 验证用; None = 不挂)
    pub testid: Option<String>,
}

/// 按 tone 返回按钮 class (模块级自由函数, 便于 rsx 外复用与排查)。
fn class_for(tone: &ActionTone) -> &'static str {
    match tone {
        ActionTone::Neutral => {
            "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white"
        }
        ActionTone::Success => {
            "flex-1 rounded-lg border border-emerald-700/50 bg-emerald-900/30 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-emerald-800/50 hover:text-emerald-300"
        }
        ActionTone::SuccessSoft => {
            "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300"
        }
        ActionTone::Warning => {
            "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300"
        }
        ActionTone::Danger => {
            "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-zinc-700 hover:text-red-300"
        }
        ActionTone::Disabled => {
            "flex-1 rounded-lg border border-zinc-800 py-1 text-[11px] text-zinc-600 cursor-not-allowed"
        }
    }
}

/// 单颗操作按钮: 按 tone 着色的等宽按钮, 禁用占位不触发回调。
///
/// 是 `ActionButtonGroup` 循环渲染的最小条目组件: 按钮组把「一颗按钮」
/// 抽象成组件复用, 组内 N 颗 = N 次实例化(一对一多)。
#[component]
pub fn ActionButton(
    /// 按钮文案
    label: String,
    /// 着色基调
    tone: ActionTone,
    /// 禁用占位 (灰显 + not-allowed, 不触发回调)
    #[props(default)]
    disabled: bool,
    /// 可选 data-testid (None = 不挂)
    #[props(default)]
    testid: Option<String>,
    /// 点击回调 (禁用占位不触发)
    on_press: EventHandler<()>,
) -> Element {
    let tid = testid.clone();
    rsx! {
        button {
            class: "{class_for(&tone)}",
            "data-testid": tid,
            disabled,
            onclick: move |_| {
                // 禁用占位不得触发回调 (除 disabled 属性外的兜底守卫)
                if disabled {
                    return;
                }
                on_press.call(());
            },
            "{label}"
        }
    }
}

/// 卡片底部操作按钮组 (等宽, 顶部分隔线)。
///
/// `actions` 顺序即渲染顺序; `on_press` 携带被按下按钮的下标
/// (禁用按钮不触发回调, `disabled` 视觉属性保留)。容器挂 `data-testid`
/// 供快照定位; 按钮 testid 取自 `ActionSpec.testid`, None = 不挂。
///
/// 循环体内复用 [`ActionButton`] 渲染每颗按钮(一对一多: 按钮组 → 单按钮组件)。
#[component]
pub fn ActionButtonGroup(
    /// 按钮规格列表 (空列表 = 不渲染)
    actions: Vec<ActionSpec>,
    /// 按下回调, 参数为被按按钮下标
    on_press: EventHandler<usize>,
    /// 容器 data-testid (快照定位按钮组整体; 按钮 testid 见 `ActionSpec`)
    #[props(default)]
    testid_prefix: String,
) -> Element {
    let specs = actions.clone();
    // 每颗按钮一个按下闭包(捕获自身下标), 循环体内直接实例化 `ActionButton`
    // (一对一多: 按钮组 → 单按钮组件), 闭包不进 rsx 推导。
    let press: Vec<EventHandler<()>> = specs
        .iter()
        .enumerate()
        .map(|(i, _)| EventHandler::new(move |_| on_press.call(i)))
        .collect();
    rsx! {
        div { class: "mt-4 flex gap-1.5 border-t {crate::T_border_zinc_800} pt-3",
            "data-testid": "{testid_prefix}",
            for (i, spec) in specs.iter().enumerate() {
                ActionButton {
                    label: spec.label.clone(),
                    tone: spec.tone,
                    disabled: spec.disabled,
                    testid: spec.testid.clone(),
                    on_press: press[i],
                }
            }
        }
    }
}
