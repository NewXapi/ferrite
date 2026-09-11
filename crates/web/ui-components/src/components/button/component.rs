//! Button — shadcn new-york-v4 风格按钮基元。
//!
//! 样式不再走 dxc 的 css_module / `data-style`，而是把 shadcn `buttonVariants`
//! （registry/new-york-v4/ui/button.tsx）的 Tailwind class 逐字拼进 `class`；
//! 调用方传入的 `class` 由 [`with_class`] 追加到组件基串之后，其余属性原样透传。

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// shadcn new-york-v4 `buttonVariants` 基础 class（ui/button.tsx:7，逐字）。
const BUTTON_BASE_CLASS: &str = "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-sm font-medium whitespace-nowrap transition-all outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

impl ButtonVariant {
    /// 变体的稳定 slug（沿用旧 data-style 键名，供调用方做属性钩子）。
    pub fn class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "primary",
            ButtonVariant::Secondary => "secondary",
            ButtonVariant::Destructive => "destructive",
            ButtonVariant::Outline => "outline",
            ButtonVariant::Ghost => "ghost",
            ButtonVariant::Link => "link",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ButtonSize {
    Xs,
    Sm,
    #[default]
    Default,
    Lg,
    Icon,
    IconXs,
    IconSm,
    IconLg,
}

impl ButtonSize {
    /// 尺寸的稳定 slug（沿用旧 data-size 键名，供调用方做属性钩子）。
    pub fn class(&self) -> &'static str {
        match self {
            ButtonSize::Xs => "xs",
            ButtonSize::Sm => "sm",
            ButtonSize::Default => "default",
            ButtonSize::Lg => "lg",
            ButtonSize::Icon => "icon",
            ButtonSize::IconXs => "icon-xs",
            ButtonSize::IconSm => "icon-sm",
            ButtonSize::IconLg => "icon-lg",
        }
    }
}

/// 变体对应的 shadcn `data-variant` 键名与 class 串（ui/button.tsx:11-20，逐字）。
///
/// 公开为组件视觉契约的一部分（等价于 shadcn 导出的 `buttonVariants`），
/// 供 `tests/` 契约测试与需要按变体取样的调用方使用。
pub fn variant_parts(variant: ButtonVariant) -> (&'static str, &'static str) {
    match variant {
        ButtonVariant::Primary => (
            "default",
            "bg-primary text-primary-foreground hover:bg-primary/90",
        ),
        ButtonVariant::Secondary => (
            "secondary",
            "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ),
        ButtonVariant::Destructive => (
            "destructive",
            "bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
        ),
        ButtonVariant::Outline => (
            "outline",
            "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50",
        ),
        ButtonVariant::Ghost => (
            "ghost",
            "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
        ),
        ButtonVariant::Link => ("link", "text-primary underline-offset-4 hover:underline"),
    }
}

/// 尺寸对应的 shadcn `data-size` 键名与 class 串（ui/button.tsx:23-30，逐字）。
///
/// 公开理由同 [`variant_parts`]。
pub fn size_parts(size: ButtonSize) -> (&'static str, &'static str) {
    match size {
        ButtonSize::Xs => (
            "xs",
            "h-6 gap-1 rounded-md px-2 text-xs has-[>svg]:px-1.5 [&_svg:not([class*='size-'])]:size-3",
        ),
        ButtonSize::Sm => ("sm", "h-8 gap-1.5 rounded-md px-3 has-[>svg]:px-2.5"),
        ButtonSize::Default => ("default", "h-9 px-4 py-2 has-[>svg]:px-3"),
        ButtonSize::Lg => ("lg", "h-10 rounded-md px-6 has-[>svg]:px-4"),
        ButtonSize::Icon => ("icon", "size-9"),
        // shadcn 原生带 icon-xs（ui/button.tsx:28）：size-6 方形 + svg 缩到 size-3，
        // 与我们多出的 IconXs 枚举一一对应，无需自造等价组合。
        ButtonSize::IconXs => (
            "icon-xs",
            "size-6 rounded-md [&_svg:not([class*='size-'])]:size-3",
        ),
        ButtonSize::IconSm => ("icon-sm", "size-8"),
        ButtonSize::IconLg => ("icon-lg", "size-10"),
    }
}

/// 把调用方传入的 `class` 追加到组件 shadcn 基串之后，其余属性原样保留。
///
/// 取代 dxc 的 `dioxus_primitives::merge_attributes`：只摘出 `class` 属性做字符串
/// 拼接（先组件基串、后调用方串；Tailwind 语义下拼接顺序不影响命中），非 `class`
/// 属性原样透传，保证调用方传的 class 与其他属性都不丢。
fn with_class(attributes: Vec<Attribute>, extra: &str) -> Vec<Attribute> {
    let mut caller_class = String::new();
    let mut rest = Vec::with_capacity(attributes.len() + 1);
    for attribute in attributes {
        match (&attribute.value, attribute.name) {
            (AttributeValue::Text(value), "class") if !value.trim().is_empty() => {
                if !caller_class.is_empty() {
                    caller_class.push(' ');
                }
                caller_class.push_str(value.trim());
            }
            _ => rest.push(attribute),
        }
    }
    let class = if caller_class.is_empty() {
        extra.to_string()
    } else {
        format!("{extra} {caller_class}")
    };
    rest.push(Attribute::new("class", class, None, false));
    rest
}

/// shadcn new-york-v4 风格按钮。
#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(extends=GlobalAttributes)]
    #[props(extends=button)]
    attributes: Vec<Attribute>,
    onclick: Option<EventHandler<MouseEvent>>,
    onmousedown: Option<EventHandler<MouseEvent>>,
    onmouseup: Option<EventHandler<MouseEvent>>,
    onkeydown: Option<EventHandler<KeyboardEvent>>,
    children: Element,
) -> Element {
    let (variant_key, variant_class) = variant_parts(variant);
    let (size_key, size_class) = size_parts(size);
    let class = with_class(
        attributes,
        &format!("{BUTTON_BASE_CLASS} {variant_class} {size_class}"),
    );

    rsx! {
        button {
            "data-slot": "button",
            "data-variant": variant_key,
            "data-size": size_key,
            onclick: move |event| {
                if let Some(f) = &onclick {
                    f.call(event);
                }
            },
            onmousedown: move |event| {
                if let Some(f) = &onmousedown {
                    f.call(event);
                }
            },
            onmouseup: move |event| {
                if let Some(f) = &onmouseup {
                    f.call(event);
                }
            },
            onkeydown: move |event| {
                if let Some(f) = &onkeydown {
                    f.call(event);
                }
            },
            ..class,
            {children}
        }
    }
}
