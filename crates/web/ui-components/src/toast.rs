//! Toast — sonner 视觉对齐的轻量通知堆栈（PR2 adopting）。
//!
//! 队列是全局的（[`TOASTS`]）：任意组件在事件回调里调用 [`toast`] /
//! [`toast_with`] 入队，根部挂一个 [`Toaster`] 负责渲染、每秒清扫过期条目、
//! 点击立即关闭。
//!
//! class 契约（视觉来源 [shadcn new-york-v4 ui/sonner.tsx] 的 normal 配色映射：
//! `--normal-bg: var(--popover)`、`--normal-text: var(--popover-foreground)`、
//! `--normal-border: var(--border)`）：
//! - 堆栈容器：右下 `fixed bottom-4 right-4 z-50 flex flex-col gap-2`。
//! - 单条：`rounded-lg border bg-popover px-4 py-3 text-sm shadow-lg`，
//!   标题 `font-medium`，描述 `text-muted-foreground`。
//! - destructive 变体：边框/文字用 destructive token（见 [`variant_parts`]）。
//!
//! 定时器：`Toaster` 挂载后 spawn 一个每秒一轮的清扫任务（wasm32 下用
//! `gloo_timers::future::TimeoutFuture` 循环 await），任务归属 Toaster 的
//! component scope，组件卸载时被自动 drop，另有 `use_drop(task.cancel())`
//! 双保险，不泄漏 interval。
//!
//! [shadcn new-york-v4 ui/sonner.tsx]: https://github.com/shadcn-ui/ui

use dioxus::prelude::*;

/// 单条 toast 的默认存活秒数（每秒清扫一轮，满 4 秒移除）。
pub const TOAST_TTL_SECS: u64 = 4;

/// 单条 toast 的基础 class（sonner normal 配色，逐字契约）。
const TOAST_ITEM_BASE_CLASS: &str =
    "pointer-events-auto cursor-pointer rounded-lg border bg-popover px-4 py-3 text-sm shadow-lg";

/// 通知变体。
///
/// 派生 `Clone`/`PartialEq`：`ToastItem` 需要整体比较与入队克隆。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastVariant {
    /// 默认（sonner normal 配色）。
    #[default]
    Default,
    /// 成功：与 Default 同配色（sonner 仅靠图标区分，本库暂无图标位）。
    Success,
    /// 危险：边框/文字用 destructive token。
    Destructive,
}

/// 一条通知。
///
/// `ttl_ticks` 是剩余存活秒数：[`Toaster`] 每秒清扫一轮，每轮减一，归零移除
/// （用倒计时而非时间戳，纯函数可测、跨目标无时钟依赖）。
#[derive(Clone, Debug, PartialEq)]
pub struct ToastItem {
    /// 唯一标识，由队列内 max(id)+1 递增生成，点击关闭时定位条目。
    pub id: u64,
    /// 标题（`font-medium`）。
    pub title: String,
    /// 可选描述（`text-muted-foreground`）。
    pub description: Option<String>,
    /// 变体，决定 `data-variant` 与叠加 class。
    pub variant: ToastVariant,
    /// 剩余存活秒数（清扫倒计时）。
    pub ttl_ticks: u64,
}

/// 全局通知队列：任意位置 [`toast`] 入队，[`Toaster`] 渲染并清扫。
pub static TOASTS: GlobalSignal<Vec<ToastItem>> = Signal::global(Vec::new);

/// variant → (data-variant 键名, 叠加 class)。
///
/// default/success 都走 sonner normal 配色（border=--border、文字=
/// popover-foreground），destructive 边框与文字用 destructive token。
/// 抽成纯函数供契约测试锁定视觉契约。
pub fn variant_parts(variant: ToastVariant) -> (&'static str, &'static str) {
    match variant {
        ToastVariant::Default => ("default", "border-border text-popover-foreground"),
        ToastVariant::Success => ("success", "border-border text-popover-foreground"),
        ToastVariant::Destructive => ("destructive", "border-destructive text-destructive"),
    }
}

/// 清扫一轮：所有条目 `ttl_ticks` 减一，归零的条目被移除（1 tick ≈ 1 秒）。
///
/// 纯函数：输入旧队列、输出清扫后的新队列，供 [`Toaster`] 的秒级定时任务与
/// 契约测试复用。
pub fn sweep_toasts(toasts: Vec<ToastItem>) -> Vec<ToastItem> {
    toasts
        .into_iter()
        .filter_map(|mut item| {
            item.ttl_ticks = item.ttl_ticks.saturating_sub(1);
            (item.ttl_ticks > 0).then_some(item)
        })
        .collect()
}

/// 递增生成 id：取队列内最大 id + 1（单线程事件循环下无竞争，且免去跨目标
/// 64 位原子支持差异）。
fn next_toast_id() -> u64 {
    TOASTS.read().iter().map(|t| t.id).max().unwrap_or(0) + 1
}

/// 入队一条通知（内部便捷函数）。
fn push_toast(item: ToastItem) {
    TOASTS.with_mut(|list| list.push(item));
}

/// 弹一条默认变体、无描述的通知（UI 事件回调里调用，需在 Dioxus runtime 内）。
///
/// # 示例
/// ```no_run
/// use ui_components::components::toast::toast;
/// toast("已保存");
/// ```
pub fn toast(title: impl Into<String>) {
    push_toast(ToastItem {
        id: next_toast_id(),
        title: title.into(),
        description: None,
        variant: ToastVariant::Default,
        ttl_ticks: TOAST_TTL_SECS,
    });
}

/// 弹一条带描述与变体的通知（UI 事件回调里调用，需在 Dioxus runtime 内）。
pub fn toast_with(title: impl Into<String>, description: impl Into<String>, variant: ToastVariant) {
    push_toast(ToastItem {
        id: next_toast_id(),
        title: title.into(),
        description: Some(description.into()),
        variant,
        ttl_ticks: TOAST_TTL_SECS,
    });
}

/// 立即移除指定 id 的通知（点击条目时也会调用）。
pub fn dismiss_toast(id: u64) {
    TOASTS.with_mut(|list| list.retain(|t| t.id != id));
}

/// 通知堆栈容器：挂在应用根部，固定右下角渲染 [`TOASTS`] 队列。
///
/// - 每秒一轮 [`sweep_toasts`]：满 [`TOAST_TTL_SECS`] 秒的条目自动消失；
/// - 点击任意条目立即 [`dismiss_toast`]。
#[component]
pub fn Toaster() -> Element {
    // 秒级清扫任务：spawn 归属当前（Toaster）scope，组件卸载时任务随之 drop；
    // use_drop 里的 task.cancel() 是双保险。native 下该组件只参与编译，不起定时器。
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        let task = spawn(async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(1_000).await;
                TOASTS.with_mut(|list| *list = sweep_toasts(std::mem::take(list)));
            }
        });
        use_drop(move || task.cancel());
    });

    let toasts = TOASTS.read();
    rsx! {
        div {
            class: "pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col gap-2",
            "data-slot": "toaster",
            for item in toasts.iter() {
                {
                    let (variant_key, variant_class) = variant_parts(item.variant);
                    let id = item.id;
                    rsx! {
                        div {
                            key: "{id}",
                            class: format!("{TOAST_ITEM_BASE_CLASS} {variant_class}"),
                            "data-slot": "toast",
                            "data-variant": "{variant_key}",
                            "data-state": "open",
                            role: if item.variant == ToastVariant::Destructive { "alert" } else { "status" },
                            onclick: move |_| dismiss_toast(id),
                            div { class: "font-medium", "{item.title}" }
                            if let Some(description) = &item.description {
                                div { class: "mt-0.5 text-muted-foreground", "{description}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
