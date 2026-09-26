//! 管理页 tab 内容区的共享壳组件。
//!
//! 本模块把 `admin-page-admin` 各 tab 目录里被反复复制粘贴的「语义完整结构块」
//! class 串收敛成组件，让页面与区段组件（`list` / `stats` / `toolbar`）的 rsx
//! 只做组装，不再出现长 class 字面量。
//!
//! 收敛范围仅限**硬编码 zinc 色**的结构块，与本 crate 既有的 shadcn token
//! 体系（`components/card`）互不影响：那套 token 的迁移是独立的换肤工作面，
//! 混做会让「样式零变化」无法验收。
//!
//! 各组件只固定外壳几何与配色，内容与交互由调用方以 children / props 传入。

use dioxus::prelude::*;

/// 卡片外壳的 class 串。
///
/// 原 `admin-page-admin` 的 aliases / channels / redemptions 三张实体卡与
/// groups 弹窗里各复制了一份同样的串；`AdminCard` 里还多了一份（少了
/// `justify-between`，导致单 panel 内容时底部留白与旧卡不一致）。统一到这里
/// 后，四类卡与 `AdminCard` 共用同一条几何定义。
pub const CARD_SHELL_CLASS: &str = "group flex flex-col justify-between rounded-xl border border-border bg-card/60 p-4 transition-all duration-200 hover:border-border hover:bg-card/80";

/// 卡片外壳组件：替代各实体卡里手写的 `div { class: CARD_SHELL_CLASS }`。
///
/// `title` 用于 `aria-label`；`testid` 透传到 `data-testid`。内容由 children
/// 传入，组件不改动内部布局——布局差异（是否 `justify-between`）由外壳统一决定。
#[component]
pub fn CardShell(
    /// 无障碍标签（通常为卡片标题）
    title: String,
    /// 可选测试标识
    #[props(default)]
    testid: Option<String>,
    /// 卡片内容
    children: Element,
) -> Element {
    rsx! {
        div {
            class: CARD_SHELL_CLASS,
            role: "region",
            "aria-label": "{title}",
            "data-testid": testid.as_deref(),
            {children}
        }
    }
}

/// 区段外壳：`scroll-mt-8` + 圆角描边面板 + 纵向间距。
///
/// 原 aliases / channels / groups / redemptions 的 `toolbar.rs` 与 system 的
/// `options.rs` / `overview.rs` / `proxy-nodes.rs` 各写了一份同样的串。
/// 通过 `id` 支持 ScrollSpy 锚点。
#[component]
pub fn AdminSection(
    /// 滚动锚点 id（ScrollSpy 用）
    #[props(default)]
    id: Option<String>,
    /// 额外的布局 class（如 `space-y-4`）；追加在外壳 class 之后
    #[props(default)]
    class: Option<String>,
    children: Element,
) -> Element {
    let extra = class.unwrap_or_default();
    rsx! {
        section {
            id: id,
            class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-border bg-card p-5 {extra}",
            {children}
        }
    }
}

/// 区段标题行：左标题 + 右侧计数/状态胶囊 + 可选分页器插槽。
#[component]
pub fn SectionHeader(
    /// 区段标题文案
    title: String,
    /// 右侧胶囊文案（计数或加载状态）
    badge: String,
    /// 标题行右侧、计数胶囊之后的可选插槽（分页器等）
    #[props(default)]
    trailing: Option<Element>,
) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-2",
            h2 { class: "{crate::TYPE_TITLE}", "{title}" }
            div { class: "flex items-center gap-2",
                span { class: "rounded-full bg-secondary px-3 py-1 {crate::TYPE_DESC}", "{badge}" }
                if let Some(extra) = trailing {
                    {extra}
                }
            }
        }
    }
}

/// 实体卡网格容器：1 列（手机）/ 3 列（中屏）/ 5 列（大屏）。
#[component]
pub fn CardGrid(
    /// 网格的无障碍标签
    aria_label: String,
    /// 可选测试标识
    #[props(default)]
    testid: Option<String>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
            role: "list",
            "aria-label": "{aria_label}",
            "data-testid": testid.as_deref(),
            {children}
        }
    }
}

/// 空态/加载态占位块（虚线描边）。
///
/// 原 aliases / channels / groups / redemptions 的 `list.rs` 各写一份。
/// `py` 与 `dashed` 允许保留各处微差（16 档 vs 10 档、虚线 vs 实线），
/// 避免为了统一而改动视觉。
#[component]
pub fn PlaceholderBlock(
    /// 文案
    message: String,
    /// 是否用虚线描边（列表空态用虚线，加载态部分用虚线）
    #[props(default = true)]
    dashed: bool,
    /// 纵向 padding 档：`lg` = py-16、`md` = py-10
    #[props(default = true)]
    large: bool,
    children: Element,
) -> Element {
    let border = if dashed { "border-dashed" } else { "" };
    let pad = if large { "py-16" } else { "py-10" };
    rsx! {
        div {
            class: "rounded-2xl border {border} border-border bg-card/50 {pad} text-center",
            p { class: "{crate::TYPE_BODY}", "{message}" }
            {children}
        }
    }
}

/// 危险态块：红底描边的错误区（列表拉取失败 / 系统告警）。
#[component]
pub fn DangerBlock(
    /// 主文案
    title: String,
    /// 次要说明（错误详情）
    #[props(default)]
    detail: Option<String>,
    /// 可选图标/自定义头部内容
    #[props(default)]
    children: Option<Element>,
) -> Element {
    rsx! {
        div { class: "rounded-2xl border border-destructive bg-destructive py-10 text-center",
            p { class: "text-sm {crate::C_DANGER}", "{title}" }
            if let Some(detail) = detail {
                p { class: "mt-1 text-xs {crate::C_DANGER}", "{detail}" }
            }
            if let Some(children) = children {
                {children}
            }
        }
    }
}
