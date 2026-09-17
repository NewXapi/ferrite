//! StatCard — 管理台统计卡基元（值 + 标签两行）。
//!
//! 收敛原先散落在 `admin-page-admin/groups`、`admin-page-users/panel`、
//! `admin-page-account/keys`、`admin-page-account/usage_logs` 四处的重复定义
//! （前三处标记逐字节相同，第四处仅 padding/字号不同）。不同面板的视觉差异
//! 由 [`StatSize`] 变体区分，三段 class 串逐字取自各处原定义——不做「顺手
//! 美化」，任何一串漂移都是用户可见的样式回归，钉死见
//! `tests/stat_card_props.rs`。
//!
//! 未纳入 `admin-page-overview` 的同名组件：那一处是 `Card` 包裹 + 主题 token
//! （`text-foreground`/`text-muted-foreground`）+ 内嵌 Sparkline 的另一套设计，
//! 且 Sparkline 依赖 overview 私有的 `api::sparkline_svg_paths`；强行并入会
//! 引入第三种变体或把数据整形逻辑下沉到组件库，保留在原地更内聚。

use dioxus::prelude::*;

/// 统计卡尺寸变体：对应原先两套并存的历史标记。
///
/// 缺省 [`StatSize::Sm`]——与 groups/users/keys 三处调用点省略 `size` 时的
/// 渲染结果一致；usage_logs 显式传 [`StatSize::Lg`]。
#[derive(Copy, Clone, Debug, PartialEq, Default)]
#[non_exhaustive]
pub enum StatSize {
    /// groups / users / keys 版：`px-4 py-3` + `text-xl` 白色值、`tracking-tight`。
    #[default]
    Sm,
    /// usage_logs 版：`px-5 py-4` + `text-2xl` zinc-100 等宽值（`tabular-nums`）。
    Lg,
}

impl StatSize {
    /// 变体的稳定 slug，供调用方做属性钩子（沿用 button/badge 的 data 键名惯例）。
    pub fn key(&self) -> &'static str {
        match self {
            StatSize::Sm => "sm",
            StatSize::Lg => "lg",
        }
    }
}

/// 变体的 `(容器 class, 值 class, 标签 class)` 三元组，逐字钉死。
///
/// 公开为组件视觉契约的一部分，供 `tests/` 契约测试使用（同
/// `components::button::size_parts` / `components::badge::variant_parts`）。
/// Sm 串取自原 `groups.rs`（= `panel.rs` = `keys.rs`），Lg 串取自原
/// `usage_logs.rs`；两套容器串仅 padding 与 hover/transition 顺序不同，
/// 保留原序以保持逐字一致。
pub fn size_parts(size: StatSize) -> (&'static str, &'static str, &'static str) {
    match size {
        StatSize::Sm => (
            "rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 transition-colors hover:border-zinc-600",
            "text-xl font-semibold tracking-tight text-white",
            "mt-0.5 text-xs text-zinc-500",
        ),
        StatSize::Lg => (
            "rounded-xl border border-zinc-800 bg-zinc-900/60 px-5 py-4 hover:border-zinc-600 transition-colors",
            "text-2xl font-semibold text-zinc-100 tabular-nums",
            "mt-1 text-xs text-zinc-500",
        ),
    }
}

/// 管理台统计卡：大数字值 + 小字标签，暗色卡面，hover 时边框变亮。
///
/// - `value`：已由调用方格式化好的展示串（计数 / 额度 / 百分比都在调用方格式化，
///   组件不做数字解析）。
/// - `label`：取 `&'static str` 对齐四处历史签名，调用点传字面量无需改动。
/// - `size`：缺省 [`StatSize::Sm`]；usage_logs 等大字版传 [`StatSize::Lg`]。
#[component]
pub fn StatCard(value: String, label: &'static str, #[props(default)] size: StatSize) -> Element {
    let (container, value_class, label_class) = size_parts(size);
    rsx! {
        div { class: "{container}",
            p { class: "{value_class}", "{value}" }
            p { class: "{label_class}", "{label}" }
        }
    }
}
