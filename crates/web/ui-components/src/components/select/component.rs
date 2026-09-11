//! Select — shadcn new-york-v4 风格下拉选择器（扁平受控 API）。
//!
//! 与 shadcn 原版的差别：原版用 Radix `Select.Root/Trigger/Content/Item` context
//! 组合，这里收敛成单个受控组件——`value` + `options` + `on_value_change`，
//! 由内部 `use_signal` 管理 `open` 状态；data-slot / data-state 属性照抄 Radix。
//!
//! class 契约（逐字来源 [shadcn new-york-v4 ui/select.tsx]）：
//! - Trigger：SelectTrigger class 逐字（含 `data-[placeholder]:text-muted-foreground`、
//!   `data-[size=default]:h-9`）。
//! - Item：SelectItem class 逐字（含 `focus:bg-accent`）。
//! - Content：Radix 专属变量类做了等价替换——`relative` → `absolute top-full left-0
//!   mt-1`（popper bottom 侧定位）、`max-h-(--radix-select-content-available-height)`
//!   → `max-h-96`、去掉 `origin-(--radix-...)` 与 `data-[side=*]` 平移段；
//!   data-state 动画段逐字保留，由 [`state_parts`] 生成。
//!
//! 交互：点击 Trigger 打开 / 点击遮罩（`fixed inset-0` 透明层）关闭；点击条目回调
//! `on_value_change` 并关闭；Escape 关闭（键盘打开后焦点仍在 Trigger 上，由
//! Trigger 的 onkeydown 承接）。
//!
//! [shadcn new-york-v4 ui/select.tsx]: https://github.com/shadcn-ui/ui

use dioxus::prelude::*;

/// shadcn ui/select.tsx SelectTrigger class（`size = "default"` 档，逐字）。
const TRIGGER_BASE_CLASS: &str = "flex w-fit items-center justify-between gap-2 rounded-md border border-input bg-transparent px-3 py-2 text-sm whitespace-nowrap shadow-xs transition-[color,box-shadow] outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 data-[placeholder]:text-muted-foreground data-[size=default]:h-9 data-[size=sm]:h-8 *:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center *:data-[slot=select-value]:gap-2 dark:bg-input/30 dark:hover:bg-input/50 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-muted-foreground";

/// shadcn ui/select.tsx SelectContent class（见模块头：Radix 变量类已做等价替换，
/// data-state 动画段由 [`state_parts`] 追加，逐字）。
const CONTENT_BASE_CLASS: &str = "absolute top-full left-0 z-50 mt-1 min-w-[8rem] max-h-96 overflow-x-hidden overflow-y-auto rounded-md border bg-popover text-popover-foreground shadow-md";

/// shadcn ui/select.tsx SelectContent 的 Viewport 内边距（逐字）。
const VIEWPORT_CLASS: &str = "p-1";

/// shadcn ui/select.tsx SelectItem class（逐字）。
const ITEM_CLASS: &str = "relative flex w-full cursor-default items-center gap-2 rounded-sm py-1.5 pr-8 pl-2 text-sm outline-hidden select-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-muted-foreground *:[span]:last:flex *:[span]:last:items-center *:[span]:last:gap-2";

/// shadcn ui/select.tsx SelectItem 的选中指示器容器 class（逐字）。
const ITEM_INDICATOR_CLASS: &str = "absolute right-2 flex size-3.5 items-center justify-center";

/// content 的 `data-state` 动画 class 串（shadcn ui/select.tsx SelectContent 逐字）：
/// 展开时 open 三段入场动画，收起时 closed 三段退场动画。
///
/// 抽成纯函数供契约测试锁定视觉契约。
pub fn state_parts(open: bool) -> &'static str {
    if open {
        "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95"
    } else {
        "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95"
    }
}

/// lucide `chevron-down`（shadcn Trigger 默认图标），尺寸/透明度对齐上游
/// `size-4 opacity-50`。
fn chevron_down_icon() -> Element {
    rsx! {
        svg {
            class: "size-4 opacity-50",
            "viewBox": "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            "stroke-width": "2",
            "stroke-linecap": "round",
            "stroke-linejoin": "round",
            "aria-hidden": "true",
            path { d: "m6 9 6 6 6-6" }
        }
    }
}

/// lucide `check`（shadcn Item 选中指示图标），尺寸对齐上游 `size-4`。
fn check_icon() -> Element {
    rsx! {
        svg {
            class: "size-4",
            "viewBox": "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            "stroke-width": "2",
            "stroke-linecap": "round",
            "stroke-linejoin": "round",
            "aria-hidden": "true",
            path { d: "M20 6 9 17l-5-5" }
        }
    }
}

/// 扁平受控的下拉选择器。
///
/// - `value`：当前选中项的 value（`options` 元组的第 0 位）；`None` 表示未选择。
/// - `options`：`(value, label)` 列表，第 1 位作为展示文本。
/// - `placeholder`：未选择时 Trigger 上的占位文本（触发 `data-placeholder`，
///   命中 `data-[placeholder]:text-muted-foreground` 灰显）。
/// - `on_value_change`：选中条目后回调该条目的 value；清空场景回调 `None` 暂未
///   提供（Radix 也没有），保留 `Option` 以备后续。
/// - `trigger_class`：追加在 SelectTrigger class 之后的调用方定制 class。
/// - `attributes`：透传到最外层 `data-slot="select"` 容器上（id、data-* 等）。
#[component]
pub fn Select(
    value: Option<String>,
    options: Vec<(String, String)>,
    placeholder: Option<String>,
    on_value_change: EventHandler<Option<String>>,
    trigger_class: Option<String>,
    #[props(extends=GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let mut open = use_signal(|| false);

    // 当前展示文本：value 对应的 label（不在 options 里则原样回显 value），
    // 未选择时回落到 placeholder。
    let current_label: Option<String> = value.as_ref().map(|v| {
        options
            .iter()
            .find(|(val, _)| val == v)
            .map(|(_, label)| label.clone())
            .unwrap_or_else(|| v.clone())
    });
    let display_text: String = current_label
        .clone()
        .or_else(|| placeholder.clone())
        .unwrap_or_default();
    let trigger_class = trigger_class.map_or_else(
        || TRIGGER_BASE_CLASS.to_string(),
        |c| format!("{TRIGGER_BASE_CLASS} {c}"),
    );
    let content_class = format!("{CONTENT_BASE_CLASS} {}", state_parts(true));

    rsx! {
        div { "data-slot": "select", ..attributes,
            div { class: "relative w-fit",
                button {
                    class: "{trigger_class}",
                    "data-slot": "select-trigger",
                    "data-size": "default",
                    "data-state": if open() { "open" } else { "closed" },
                    "data-placeholder": value.is_none().then_some("true"),
                    role: "combobox",
                    "aria-haspopup": "listbox",
                    "aria-expanded": "{open()}",
                    onclick: move |_| open.set(true),
                    onkeydown: move |e: KeyboardEvent| match e.key() {
                        Key::Escape if open() => open.set(false),
                        Key::ArrowDown | Key::ArrowUp | Key::Enter => open.set(true),
                        _ => {}
                    },
                    span { "data-slot": "select-value", "{display_text}" }
                    {chevron_down_icon()}
                }
                if open() {
                    // 全屏透明遮罩：拦截「点击外部」并顺带把点击 Trigger 变成关闭，
                    // z-40 低于 content 的 z-50，条目仍可点。
                    div {
                        class: "fixed inset-0 z-40 cursor-default",
                        "aria-hidden": "true",
                        onclick: move |_| open.set(false),
                    }
                    div {
                        class: "{content_class}",
                        "data-slot": "select-content",
                        "data-state": "open",
                        role: "listbox",
                        div { class: VIEWPORT_CLASS,
                            for (item_value, label) in options.iter() {
                                {
                                    let is_selected = value.as_ref() == Some(item_value);
                                    let item_value = item_value.clone();
                                    rsx! {
                                        div {
                                            key: "{item_value}",
                                            class: ITEM_CLASS,
                                            "data-slot": "select-item",
                                            "data-state": if is_selected { "checked" } else { "unchecked" },
                                            role: "option",
                                            "aria-selected": "{is_selected}",
                                            onclick: move |_| {
                                                on_value_change.call(Some(item_value.clone()));
                                                open.set(false);
                                            },
                                            span {
                                                class: ITEM_INDICATOR_CLASS,
                                                "data-slot": "select-item-indicator",
                                                if is_selected {
                                                    {check_icon()}
                                                }
                                            }
                                            span { "{label}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
