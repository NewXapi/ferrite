//! 分组 tab 的通用小件与编辑弹窗:
//! - `StatCard` / `Badge` —— 概览卡与状态徽标,被 stats / list / card 复用;
//! - `GroupCard` —— 单张可操作分组卡(倍率滑条 + 操作按钮组);
//! - `Modal` —— 通用弹窗外壳(遮罩 + 标题 + 关闭 + slot);
//! - `GroupFormModal` —— 新建 / 编辑分组表单(双页签,自带提交请求)。
//!
//! 边界:本文件不含列表四态与网格排版(在 `list.rs`);`Modal` 是跨 tab 复用的
//! 通用外壳(redemptions 的弹窗也 import 它),但暂留在此未上移。
//! `GroupFormModal` 是**唯一自带网络请求的组件**(提交时直接调
//! `create_group_api` / `update_group_api`),与 `list.rs` 的纯展示约定不同。

use super::shared::{
    BTN_BUILTIN, BTN_CANCEL, BTN_CREATE_GROUP, BTN_DELETE, BTN_DISABLE, BTN_EDIT, BTN_ENABLE,
    BTN_SAVE_CHANGES, FIELD_ALIAS, FIELD_GROUP_NAME, FIELD_RATIO, FIELD_REMARK, FIELD_WHITELIST,
    LBL_ACTUAL_COST, LBL_BUILTIN, LBL_CLOSE, LBL_DEFAULT, LBL_EXAMPLE_COST,
    LBL_MULT_BASELINE_PREFIX, LBL_MULT_DISCOUNT_PREFIX, LBL_MULT_MARKUP_PREFIX, LBL_RATIO,
    LBL_SCOPE, LBL_SCOPE_VALUE, MSG_ALIAS_HINT, MSG_DEFAULT_LOCKED, MSG_NO_ALIAS_OPTIONS,
    MSG_PH_ALIAS, MSG_PH_GROUP_NAME, MSG_PH_REMARK, MSG_PH_WHITELIST, MSG_WHITELIST_HINT,
    OPT_DISABLED, OPT_ENABLED, OPT_RATIO_BASELINE, OPT_RATIO_DISCOUNT, OPT_RATIO_DOUBLE,
    OPT_RATIO_HALF, OPT_RATIO_HIGH, OPT_RATIO_MARKUP, OPT_RATIO_PRESETS, TAB_ALIAS, TAB_BASIC,
    TTL_EDIT, TTL_NEW, parse_whitelist_raw,
};
use crate::api::{create_group_api, update_group_api};
use client::ApiClient;
use contract::api::admin::{GroupDto, GroupUpsertRequest};
use dioxus::prelude::*;
use serde_json::json;
use ui::ActionButtonGroup;
use ui::ActionSpec;
use ui::ActionTone;
use ui::SegmentedCapsule;
// ============ 组件 ============

/// 概览统计卡(单张:大号数值 + 小号标签)。
///
/// 【是什么】一张只读的数值概览卡,用于顶部统计区的五个指标。
///
/// 【做什么】展示 `value` 与 `label` 两行文本。不负责取值计算、不负责点击、
/// 不负责单位换算(数值已由调用方格式化成字符串)。
///
/// 【交互逻辑】纯展示,无交互:无 `EventHandler`,无网络请求,无内部状态。
///
/// 【样式】外壳 `rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3`,
/// 悬停 `hover:border-zinc-600` 且 `transition-colors`;数值行
/// `text-xl font-semibold tracking-tight text-white`;标签行 `mt-0.5 text-xs text-zinc-500`。
///
/// 【子组件组成】无(两个原生 `p`)。
///
/// 【数据流】
/// - 对内(入):`value` 已格式化的展示数值(如 `"5"` / `"1.20×"`)、
///   `label` 静态标签(`shared.rs` 的 `LBL_STAT_*` 常量)。
/// - 对外(出):无。
#[component]
pub fn StatCard(value: String, label: &'static str) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 transition-colors hover:border-zinc-600",
            p { class: "text-xl font-semibold tracking-tight text-white", "{value}" }
            p { class: "mt-0.5 text-xs text-zinc-500", "{label}" }
        }
    }
}

/// 状态/属性小徽标(圆角胶囊,配色由调用方按语义传入)。
///
/// 【是什么】一枚 `rounded-full` 的胶囊小标签,用于卡片徽标行(倍率 / 内置 / 状态 /
/// 面值)与兑换码卡片。
///
/// 【做什么】渲染 `text` 并套用调用方给的 `tone` 配色串。不负责决定语义配色
/// (由调用方按 status 判定)、不负责点击、不含任何状态。
///
/// 【交互逻辑】纯展示,无交互:无 `EventHandler`,无网络请求。
///
/// 【样式】`rounded-full border px-2 py-0.5 text-[11px] font-medium` 为固定部分,
/// 具体色系(emerald 折扣 / amber 溢价 / zinc 基准 / blue 内置)完全由 `tone`
/// 注入,本组件不自带任何色彩倾向。
///
/// 【子组件组成】无(单个原生 `span`)。
///
/// 【数据流】
/// - 对内(入):`text` 徽标文案(状态名、`面值 ¥N` 等)、`tone` 语义配色 class 串
///   (调用方在 `GroupCard` / `RedemptionCard` 内按 status 算好)。
/// - 对外(出):无。
#[component]
pub fn Badge(text: String, tone: &'static str) -> Element {
    rsx! {
        span { class: "rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}",
            "{text}"
        }
    }
}

/// 单个分组卡片(对齐 UserCard 风格)。
///
/// 【是什么】一张可操作的分组卡:分组名 + 默认/内置/状态徽标 + 两段式倍率滑条 +
/// 指标行(100 额度实扣 / 调度作用域)+ 底部 [编辑][启停][删除/内置] 按钮组。
///
/// 【做什么】展示单条 `GroupDto` 的全部视觉信息,并在卡内消化倍率滑条的两段式交互;
/// 编辑 / 删除 / 启停 / 倍率写回全部以 `EventHandler` 抛出,不自己做任何网络请求,
/// 也不改动外部状态。不负责网格排版(在 `list.rs`)、不负责默认组判定规则的业务含义
/// (只按 `is_default` 展示)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点倍率滑条轨道(首次)→ 本地 `adjusting = true`,出现 thumb,并按点击位置换算
///   预览值写入 `local_ratio`(纯本地,不发网络);此时指针移动(`onpointermove`,
///   要求按住)会持续改写 `local_ratio` 实时预览,步进 0.05、下限 0.05。
/// - 再次点击滑条 → `on_ratio_drag(local_ratio)` 把当前预览值抛回页面(页面据此调
///   `update_group_ratio_api`),随后 `adjusting = false`、thumb 消失。
/// - 点底部按钮组 → 按下标分派:0→`on_edit`、1→`on_toggle_status`、2→`on_delete`,
///   三者均只抛 `EventHandler`,网络请求由页面闭包完成。
/// 数据交互:本组件自身**不发任何网络请求**;滑条拖动全程只改本地 signal。
///
/// 【样式】外壳 `group flex flex-col justify-between rounded-xl border border-zinc-800
/// bg-zinc-900/60 p-4`,悬停 `hover:border-zinc-600 hover:bg-zinc-900/80` 且
/// `transition-all duration-200`,`data-testid="group-card"`;默认标签为蓝底
/// `bg-blue-950/60 border-blue-800/60 text-blue-300`;滑条轨道 `h-4 w-full`,
/// 灰底 `h-1.5 ... bg-zinc-800` 上叠彩色进度条,调整态才渲染 thumb
/// (`h-3.5 w-3.5 rounded-full border-2 border-zinc-100 bg-zinc-900 shadow`,
/// `data-testid="ratio-thumb"`)。
///
/// 【子组件组成】`Badge`(倍率 / 内置 / 状态三枚)、`ui::ActionButtonGroup`(底部按钮组)。
///
/// 【数据流】
/// - 对内(入):`group`(单条 `GroupDto`,提供 name / ratio / status / key)、
///   `is_default`(页面按 `name == "default"` 判定,决定默认标签与删除位占位)、
///   `on_edit` / `on_delete` / `on_toggle_status` / `on_ratio_drag`。
/// - 对外(出):三个无参 `EventHandler` → 页面构造 `WriteOp::Delete` /
///   `WriteOp::ToggleStatus` 或 `open_edit`;`on_ratio_drag(f64)` → 页面构造
///   `WriteOp::SetRatio(v)`。写回后由页面就地更新本地列表,卡片随之重渲染。
///
/// 状态块:`adjusting` / `local_ratio` 是本卡独有的临时交互状态(滑条是否处于
/// 可拖预览态、预览值),不跨组件、不参与写库,故留在组件内 `use_signal`;
/// 静态态的显示值始终取后端值 `m`,避免未确认的拖动污染展示。
#[component]
pub fn GroupCard(
    group: GroupDto,
    is_default: bool,
    on_edit: EventHandler<()>,
    on_delete: EventHandler<()>,
    on_toggle_status: EventHandler<()>,
    // 倍率滑条拖动结束时回调新倍率 (卡片本地即时改, 由页面层写回后端)
    on_ratio_drag: EventHandler<f64>,
) -> Element {
    let m = group.ratio;

    // 倍率滑条两段式交互:
    // - 平时只显示静态色块 (无 thumb, 指针划过不吸附)。
    // - 第一次点击轨道 → 进入调整态: 出现 thumb, 拖动即时预览 (不写库)。
    // - 第二次点击 (含拖动后松开再点) → 确认: on_ratio_drag 写回后端, thumb 消失。
    // 选中态存 adjusting signal; local_ratio 只在调整态被拖动改写。
    let mut adjusting = use_signal(|| false);
    let mut local_ratio = use_signal(|| m);
    let show_thumb = adjusting();
    // 调整态预览 live; 静态态显示后端值 m (拖动失败/未确认不污染展示)
    let display_ratio = if show_thumb { local_ratio() } else { m };
    // 滑条域 0.0–2.0 (UI 上限截断; 写回原值可能 >2, 视觉封顶)
    const RATIO_MAX: f64 = 2.0;
    let live_pct = ((display_ratio / RATIO_MAX) * 100.0).clamp(0.0, 100.0);

    // 倍率状态与徽标 (滑条宽度由 live_pct 实时算, 不再固定 bar_width_pct)
    let (mult_badge_text, mult_badge_tone, bar_tone) = if (m - 1.0).abs() < 0.001 {
        (
            format!("{LBL_MULT_BASELINE_PREFIX}{m:.2}×"),
            "border-zinc-700 bg-zinc-800/80 text-zinc-300",
            "bg-zinc-200",
        )
    } else if m < 1.0 {
        let discount = ((1.0 - m) * 100.0).round() as i64;
        (
            format!("{LBL_MULT_DISCOUNT_PREFIX}{m:.2}× (-{discount}%)"),
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
        )
    } else {
        let markup = ((m - 1.0) * 100.0).round() as i64;
        (
            format!("{LBL_MULT_MARKUP_PREFIX}{m:.2}× (+{markup}%)"),
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
        )
    };

    let status_text = if group.status == 1 {
        OPT_ENABLED
    } else {
        OPT_DISABLED
    };
    let status_tone = if group.status == 1 {
        "border-emerald-500/30 bg-emerald-500/20 text-emerald-400"
    } else {
        "border-zinc-700 bg-zinc-800/80 text-zinc-400"
    };

    let example_cost = (100.0 * m).round() as i64;

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            "data-testid": "group-card",

            div { class: "space-y-3",
                // 头部:分组名 + 默认标签 (卡内勾选框已移除, 多选改到列表外的 chips 区)
                div { class: "min-w-0",
                    div { class: "flex items-center justify-between gap-2",
                        h3 { class: "truncate text-sm font-medium text-zinc-100", "{group.name}" }
                        if is_default {
                            span { class: "min-w-0 max-w-[140px] truncate rounded bg-blue-950/60 border border-blue-800/60 px-1.5 py-0.5 text-[10px] font-mono text-blue-300 shrink-0",
                                title: LBL_DEFAULT,
                                "{LBL_DEFAULT}"
                            }
                        }
                    }
                }

                // 徽标行: 倍率徽标 + (default) 系统内置 + 状态; 非默认组不再单独挂「自定义分组」
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: mult_badge_text, tone: mult_badge_tone }
                    if is_default {
                        Badge { text: LBL_BUILTIN.to_string(), tone: "border-blue-500/30 bg-blue-500/20 text-blue-300" }
                    }
                    Badge { text: status_text.to_string(), tone: status_tone }
                }

                // 倍率滑条 (两段式): 静态时只有色块无 thumb (指针划过不吸附);
                // 第一次点击 → 进入调整态出现 thumb 可拖预览; 第二次点击 → 确认写回 + thumb 消失。
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "{LBL_RATIO}" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200", "×{display_ratio:.2}" }
                    }
                    div {
                        class: "relative h-4 w-full touch-none select-none",
                        class: if show_thumb { "relative h-4 w-full cursor-ew-resize touch-none select-none" } else { "relative h-4 w-full touch-none select-none" },
                        id: "group-ratio-slider",
                        "data-testid": "ratio-slider",
                        onclick: move |e| {
                            if show_thumb {
                                // 第二次点击 = 确认: 写回预览值, 退出调整态
                                on_ratio_drag.call(local_ratio.peek().max(0.05));
                                adjusting.set(false);
                            } else {
                                // 第一次点击 = 进入调整态, 从点击处开始预览
                                local_ratio.set(m);
                                adjusting.set(true);
                                // 立即把预览挪到点击位置 (复用同一换算)
                                let doc = web_sys::window().and_then(|w| w.document());
                                let el = doc
                                    .as_ref()
                                    .and_then(|d| d.get_element_by_id("group-ratio-slider"));
                                let client_x = e.client_coordinates().x;
                                if let Some(el) = el {
                                    let r = el.get_bounding_client_rect();
                                    let frac = ((client_x - r.left()) / r.width().max(1.0)).clamp(0.0, 1.0);
                                    let next = ((frac * RATIO_MAX) / 0.05).round() * 0.05;
                                    local_ratio.set(next.max(0.05));
                                }
                            }
                        },
                        onpointermove: move |e| {
                            // 仅调整态且有按键按住时拖动预览; 静态态划过不动 (不吸附)。
                            // 用 held_buttons 而非 trigger_button: pointermove 在触屏上
                            // 无「触发键」, held_buttons 按住触摸时含 Primary, 兼容触屏拖动。
                            if !show_thumb || e.held_buttons().is_empty() {
                                return;
                            }
                            // 拖动: client_x 相对轨道 rect 换算比例 (w-full 响应式,
                            // 每次从 document 取 bounding rect)
                            let doc = web_sys::window().and_then(|w| w.document());
                            let el = doc
                                .as_ref()
                                .and_then(|d| d.get_element_by_id("group-ratio-slider"));
                            let client_x = e.client_coordinates().x;
                            let frac = if let Some(el) = el {
                                let r = el.get_bounding_client_rect();
                                ((client_x - r.left()) / r.width().max(1.0)).clamp(0.0, 1.0)
                            } else {
                                0.5
                            };
                            let next = ((frac * RATIO_MAX) / 0.05).round() * 0.05;
                            local_ratio.set(next.max(0.05));
                        },
                        // track 灰底 + 左色块
                        div { class: "absolute left-0 top-1/2 h-1.5 w-full -translate-y-1/2 rounded-full bg-zinc-800" }
                        div { class: "absolute left-0 top-1/2 h-1.5 -translate-y-1/2 rounded-full {bar_tone} transition-colors duration-200",
                            style: "width: {live_pct}%;"
                        }
                        // thumb 只在调整态渲染
                        if show_thumb {
                            div {
                                class: "absolute top-1/2 h-3.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-zinc-100 bg-zinc-900 shadow",
                                "data-testid": "ratio-thumb",
                                style: "left: {live_pct}%;"
                            }
                        }
                    }
                }

                // 详情指标行: 保留「100额度实扣」与「调度作用域」;
                // 「费率模式」行与弹窗预览的「标准消耗」同义 (溢价/优惠/标准 已在徽标表达), 删除。
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "{LBL_EXAMPLE_COST}" }
                        span { class: "font-medium text-zinc-200 font-mono", "{example_cost} 点" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "{LBL_SCOPE}" }
                        span { class: "font-medium text-zinc-200", "{LBL_SCOPE_VALUE}" }
                    }
                }
            }

            // 底部操作按钮组: [编辑] [启用/停用] [删除/内置]
            // 用 ui::ActionButtonGroup 统一渲染; 下标语义: 0=编辑 1=启停 2=删除/内置
            div { class: "mt-1",
                ActionButtonGroup {
                    testid_prefix: "group-actions".to_string(),
                    on_press: move |i: usize| {
                        match i {
                            0 => on_edit.call(()),
                            1 => on_toggle_status.call(()),
                            2 => on_delete.call(()),
                            _ => {}
                        }
                    },
                    actions: {
                        let mut acts = vec![ActionSpec {
                            label: BTN_EDIT.to_string(),
                            tone: ActionTone::Neutral,
                            disabled: false,
                            testid: Some("edit-group".into()),
                        }];
                        // 启停位: 启用中→停用(默认组为禁用占位); 已停用→启用
                        if group.status == 1 {
                            acts.push(ActionSpec {
                                label: BTN_DISABLE.to_string(),
                                tone: if is_default { ActionTone::Disabled } else { ActionTone::Neutral },
                                disabled: is_default,
                                testid: Some("disable-group".into()),
                            });
                        } else {
                            acts.push(ActionSpec {
                                label: BTN_ENABLE.to_string(),
                                tone: ActionTone::Success,
                                disabled: false,
                                testid: Some("enable-group".into()),
                            });
                        }
                        // 删除位: 默认组为禁用占位「内置」
                        if is_default {
                            acts.push(ActionSpec {
                                label: BTN_BUILTIN.to_string(),
                                tone: ActionTone::Disabled,
                                disabled: true,
                                testid: None,
                            });
                        } else {
                            acts.push(ActionSpec {
                                label: BTN_DELETE.to_string(),
                                tone: ActionTone::Danger,
                                disabled: false,
                                testid: Some("delete-group".into()),
                            });
                        }
                        acts
                    }
                }
            }
        }
    }
}

// ============ 弹窗 ============

/// 通用弹窗外壳(遮罩 + 标题栏 + 关闭按钮 + slot 内容)。
///
/// 【是什么】一个居中弹窗外壳:半透明遮罩、标题行(标题 + 右上角关闭 ×)、
/// 下方 `children` 插槽区域。
///
/// 【做什么】只提供外壳与关闭交互,不关心内容是什么;`children` 由调用方传入。
/// 不负责表单、不负责提交、不负责任何数据。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点遮罩(弹窗外区域)→ `on_close` 抛回调用方(页面据此关弹窗)。
/// - 点右上角关闭按钮 → 同上。
/// - 点弹窗内部 → `e.stop_propagation()` 阻止冒泡,不会误触发遮罩关闭。
/// 数据交互:本组件自身不发网络请求。
///
/// 【样式】遮罩 `fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4
/// backdrop-blur-sm`;弹窗体 `w-full max-w-md rounded-2xl border border-zinc-800
/// bg-zinc-900 p-5 shadow-xl`;标题 `text-base font-semibold text-zinc-100`;
/// 关闭按钮 `rounded-lg p-1.5 text-zinc-500`,悬停 `hover:bg-zinc-800 hover:text-zinc-200`,
/// 内嵌一个 `h-5 w-5` 的 stroke 风格 × 图标(`aria-label` 取 `LBL_CLOSE`)。
///
/// 【子组件组成】无(原生 `div` / `h3` / `button` / `svg`);`children` 为调用方 slot。
///
/// 【数据流】
/// - 对内(入):`title` 弹窗标题、`on_close`(关闭回调)、`children`(slot 内容)。
/// - 对外(出):`on_close` → 调用方(如 `GroupFormModal` 的 `on_cancel`、
///   `GeneratedCodesModal` 的 `on_close`)关弹窗或清空弹窗状态。
#[component]
pub(crate) fn Modal(title: String, on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "{title}" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": LBL_CLOSE,
                        svg {
                            class: "h-5 w-5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                        }
                    }
                }
                {children}
            }
        }
    }
}

const MODAL_INPUT: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none";

/// 新建 / 编辑分组弹窗(双页签表单)。
///
/// 【是什么】分组的新建/编辑表单弹窗,内含两个页签:0「分组信息」(名称 / 备注 /
/// 白名单 / 倍率 + 预设 + 计费预览),1「映射别名」(输入框 + 候选 chips 多选)。
///
/// 【做什么】渲染表单、就地预览倍率、并在提交时**自己发网络请求**完成
/// 创建或更新。不负责开关弹窗(由页面的 `modal_state` 控制)、不负责回填(页面
/// `open_edit` 已把值写进传入的 Signal)、不负责列表刷新(只回调 `on_submit`)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 输入框 / 白名单 / 倍率 / 别名 → 直接 `set` 对应的页面级 Signal(纯本地)。
/// - 点倍率预设按钮 → `ratio.set(预设值)`(纯本地)。
/// - 点页签胶囊 → 本地 `active_tab.set(i)` 切换两个页签的显隐。
/// - 点别名候选 chip → 在 `f_alias` 里按逗号解析后加入/移出该项,再拼回字符串。
/// - 点「取消」/ `Modal` 的关闭 → `on_cancel` 抛回页面关弹窗。
/// - 点提交 → `do_submit`:名称 trim 后为空则直接返回(不提交);否则 `submitting` 置真,
///   用 `parse_whitelist_raw` 拆白名单,按 `group_key` 有无走
///   `update_group_api`(编辑)或 `create_group_api`(新建),完成后 `on_submit` 通知页面。
/// 数据交互:这是本 tab **唯一自带网络请求**的组件(见模块头)。
///
/// 【样式】经 `Modal` 外壳(`max-w-md`);输入框统一 `MODAL_INPUT`
/// (`w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5`,
/// 聚焦 `focus:border-zinc-500`);倍率输入额外加 `font-mono`;预设/别名 chip 选中态
/// `border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold`;计费预览块
/// `rounded-xl border border-zinc-800 bg-zinc-950`,数值按倍率着色(低于 1
/// 为 `text-emerald-400`,高于 1 为 `text-amber-400`);底部两按钮
/// `flex-1 rounded-xl`,提交为白底 `bg-white text-zinc-900`,禁用时 `disabled:opacity-40`。
///
/// 【子组件组成】`Modal`(外壳)、`SegmentedCapsule`(双页签胶囊)。
///
/// 【数据流】
/// - 对内(入):`editing`(标题与提交文案二选一)、`group_key`(编辑时的后端 key,
///   None 走新建)、`name` / `ratio` / `remark` / `whitelist` / `f_alias`(页面持有的
///   表单 Signal,双向就地读写)、`alias_options`(models 域候选名,页面 effect 拉取,
///   为空时显示 `MSG_NO_ALIAS_OPTIONS`)、`on_cancel` / `on_submit`。
/// - 对外(出):五个 Signal 的写回停留本地表单(仅提交时读取);`on_cancel` → 页面
///   置 `ModalState::Closed`;`on_submit` → 页面 `close_and_reload`(关弹窗 +
///   `reload + 1` 重拉列表)。
///
/// 状态块:`active_tab`(当前页签)与 `submitting`(提交中防重复点击)都是本弹窗独有的
/// UI 临时状态,不跨组件,故留在组件内 `use_signal`;`ratio()` 派生的 `parsed_ratio`
/// 只用于预设高亮与计费预览,不参与写库。
#[component]
pub fn GroupFormModal(
    editing: bool,
    group_key: Option<String>,
    name: Signal<String>,
    ratio: Signal<String>,
    remark: Signal<String>,
    whitelist: Signal<String>,
    // 映射别名候选 (后端 models 域列表的 name); 空 = 拉取失败/无候选, 只读回退。
    alias_options: Vec<String>,
    // 映射别名草稿 (逗号分隔字符串; MVP 仅登记展示, 不随提交落库, 文案见 tab1)
    f_alias: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing { TTL_EDIT } else { TTL_NEW };
    let submit_label = if editing {
        BTN_SAVE_CHANGES
    } else {
        BTN_CREATE_GROUP
    };

    // 编辑弹窗双 tab: 0=分组信息(名称/倍率/备注/白名单), 1=映射别名
    let mut active_tab = use_signal(|| 0usize);
    let tab_labels = vec![TAB_BASIC.to_string(), TAB_ALIAS.to_string()];

    let parsed_ratio = ratio().trim().parse::<f64>().unwrap_or(1.0).max(0.0);

    let submitting = use_signal(|| false);

    // 工厂式复制,避免把原 signal 移动出闭包(供 rsx 中 submitting() 继续读取)
    let submitting2 = submitting;
    let on_submit2 = on_submit;
    let group_key2 = group_key.clone();
    let whitelist2 = whitelist;
    let do_submit = move |_| {
        let key = group_key2.clone();
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let r = ratio.peek().trim().parse::<f64>().unwrap_or(1.0).max(0.0);
        let rm = remark.peek().clone();
        let wl = parse_whitelist_raw(&whitelist2.peek());
        let (mut sub, cb) = (submitting2, on_submit2);
        spawn(async move {
            sub.set(true);
            let client = ApiClient::shared().clone();
            let req = GroupUpsertRequest {
                name: n,
                ratio: r,
                model_whitelist: json!(wl),
                remark: rm,
            };
            let res = match key {
                Some(kk) => update_group_api(&client, &kk, &req).await,
                None => create_group_api(&client, &req).await,
            };
            let _ = res;
            sub.set(false);
            cb.call(());
        });
    };

    let preset_ratios = [
        (OPT_RATIO_HALF, "0.5"),
        (OPT_RATIO_DISCOUNT, "0.8"),
        (OPT_RATIO_BASELINE, "1.0"),
        (OPT_RATIO_MARKUP, "1.2"),
        (OPT_RATIO_HIGH, "1.5"),
        (OPT_RATIO_DOUBLE, "2.0"),
    ];

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            // tab 栏
            div { class: "mb-4",
                SegmentedCapsule {
                    items: tab_labels,
                    active: active_tab(),
                    on_select: move |i| active_tab.set(i),
                    testid_prefix: "group-tab".to_string(),
                }
            }

            div { class: "space-y-4",
                // tab 0: 分组信息
                if active_tab() == 0 {
                    div { class: "space-y-4",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_GROUP_NAME}" }
                            input {
                                class: MODAL_INPUT,
                                "data-testid": "group-name",
                                placeholder: MSG_PH_GROUP_NAME,
                                value: "{name}",
                                disabled: editing && name() == "default",
                                oninput: move |e| name.set(e.value()),
                            }
                            if editing && name() == "default" {
                                p { class: "mt-1 text-xs text-zinc-500", "{MSG_DEFAULT_LOCKED}" }
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_REMARK}" }
                            input {
                                class: MODAL_INPUT,
                                "data-testid": "group-remark",
                                placeholder: MSG_PH_REMARK,
                                value: "{remark}",
                                oninput: move |e| remark.set(e.value()),
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_WHITELIST}" }
                            input {
                                class: MODAL_INPUT,
                                "data-testid": "group-whitelist",
                                placeholder: MSG_PH_WHITELIST,
                                value: "{whitelist}",
                                oninput: move |e| whitelist.set(e.value()),
                            }
                            p { class: "mt-1 text-[11px] text-zinc-500", "{MSG_WHITELIST_HINT}" }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_RATIO}" }
                            input {
                                class: "{MODAL_INPUT} font-mono",
                                r#type: "text",
                                "data-testid": "group-ratio",
                                placeholder: "1.0",
                                value: "{ratio}",
                                oninput: move |e| ratio.set(e.value()),
                            }
                        }

                        // 快捷预设按钮
                        div { class: "space-y-1.5",
                            p { class: "text-[11px] text-zinc-500", "{OPT_RATIO_PRESETS}" }
                            div { class: "flex flex-wrap gap-1.5",
                                for (lbl, val) in preset_ratios {
                                    {
                                        let is_active = (parsed_ratio - val.parse::<f64>().unwrap_or(0.0)).abs() < 0.001;
                                        let btn_tone = if is_active {
                                            "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                        } else {
                                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                        };
                                        rsx! {
                                            button {
                                                class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {btn_tone}",
                                                onclick: move |_| ratio.set(val.to_string()),
                                                "{lbl}"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 计费预览: 仅保留「该分组实际扣费」(「标准消耗」行已删)
                        div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs space-y-1.5",
                            div { class: "flex justify-between font-medium",
                                span { class: "text-zinc-300", "{LBL_ACTUAL_COST}" }
                                span { class: if parsed_ratio < 1.0 { "text-emerald-400" } else if parsed_ratio > 1.0 { "text-amber-400" } else { "text-zinc-200" },
                                    "{(100.0 * parsed_ratio).round() as i64} 点额度"
                                }
                            }
                        }
                    }
                }

                // tab 1: 映射别名 (从后端 models 域候选挑选; 失败留空 → 只读回退)
                if active_tab() == 1 {
                    div { class: "space-y-3",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_ALIAS}" }
                            input {
                                class: MODAL_INPUT,
                                "data-testid": "group-alias",
                                placeholder: MSG_PH_ALIAS,
                                value: "{f_alias}",
                                oninput: move |e| f_alias.set(e.value()),
                            }
                            p { class: "mt-1 text-[11px] text-zinc-500", "{MSG_ALIAS_HINT}" }
                        }
                        if alias_options.is_empty() {
                            p { class: "text-xs text-zinc-500", "{MSG_NO_ALIAS_OPTIONS}" }
                        } else {
                            div { class: "flex flex-wrap gap-1.5",
                                for opt in alias_options.clone() {
                                    {
                                        let picked = parse_whitelist_raw(&f_alias.peek()).iter().any(|a| a == &opt);
                                        let cls = if picked {
                                            "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                        } else {
                                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                        };
                                        rsx! {
                                            button {
                                                class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {cls}",
                                                "data-testid": "group-alias-opt",
                                                onclick: move |_| {
                                                    let mut cur = parse_whitelist_raw(&f_alias.peek());
                                                    if let Some(pos) = cur.iter().position(|a| a == &opt) {
                                                        cur.remove(pos);
                                                    } else {
                                                        cur.push(opt.clone());
                                                    }
                                                    f_alias.set(cur.join(", "));
                                                },
                                                "{opt}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    "data-testid": "group-cancel",
                    onclick: move |_| on_cancel.call(()),
                    "{BTN_CANCEL}"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    "data-testid": "group-submit",
                    disabled: submitting(),
                    onclick: do_submit,
                    "{submit_label}"
                }
            }
        }
    }
}
