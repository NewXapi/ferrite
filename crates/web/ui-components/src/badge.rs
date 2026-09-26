//! Badge — shadcn new-york-v4 风格徽章基元。
//!
//! 样式不再走 dxc 的 css_module / `data-style`，而是把 shadcn `badgeVariants`
//! （registry/new-york-v4/ui/badge.tsx）的 Tailwind class 逐字拼进 `class`；
//! 调用方传入的 `class` 由 [`with_class`] 追加到组件基串之后，其余属性原样透传。
//! [`VerifiedIcon`] 改为内联 lucide `badge-check` SVG，不再依赖 `dioxus_icons`。

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// shadcn new-york-v4 `badgeVariants` 基础 class（ui/badge.tsx:7，逐字）。
const BADGE_BASE_CLASS: &str = "inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-full border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap transition-[color,box-shadow] focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&>svg]:pointer-events-none [&>svg]:size-3";

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum BadgeVariant {
    #[default]
    Primary,
    Secondary,
    Destructive,
    Outline,
}

impl BadgeVariant {
    /// 变体的稳定 slug（沿用旧 data-style 键名，供调用方做属性钩子）。
    pub fn class(&self) -> &'static str {
        match self {
            BadgeVariant::Primary => "primary",
            BadgeVariant::Secondary => "secondary",
            BadgeVariant::Destructive => "destructive",
            BadgeVariant::Outline => "outline",
        }
    }
}

/// 变体对应的 shadcn `data-variant` 键名与 class 串（ui/badge.tsx:9-19，逐字）。
///
/// 公开为组件视觉契约的一部分，供 `tests/` 契约测试使用。
pub fn variant_parts(variant: BadgeVariant) -> (&'static str, &'static str) {
    match variant {
        BadgeVariant::Primary => (
            "default",
            "bg-primary text-primary-foreground [a&]:hover:bg-primary/90",
        ),
        BadgeVariant::Secondary => (
            "secondary",
            "bg-secondary text-secondary-foreground [a&]:hover:bg-secondary/90",
        ),
        BadgeVariant::Destructive => (
            "destructive",
            "bg-destructive text-white focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40 [a&]:hover:bg-destructive/90",
        ),
        BadgeVariant::Outline => (
            "outline",
            "border-border text-foreground [a&]:hover:bg-accent [a&]:hover:text-accent-foreground",
        ),
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

/// shadcn new-york-v4 风格徽章。
#[component]
pub fn Badge(
    #[props(default)] variant: BadgeVariant,
    /// Additional attributes to extend the badge element
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let (variant_key, variant_class) = variant_parts(variant);
    let class = with_class(attributes, &format!("{BADGE_BASE_CLASS} {variant_class}"));

    rsx! {
        span {
            "data-slot": "badge",
            "data-variant": variant_key,
            ..class,
            {children}
        }
    }
}

/// 内联 lucide `badge-check` 对勾图标。
///
/// 取代 `dioxus_icons::lucide::BadgeCheck`：两个 path（圆齿外框 + 对勾）逐字来自
/// lucide badge-check.svg；stroke 跟随 `currentColor`，`size-3`（12px）等价尺寸，
/// 另设 `width`/`height` 兜底，Tailwind 工具类尚未可用时也保持 12px。
#[component]
pub fn VerifiedIcon() -> Element {
    rsx! {
        svg {
            "xmlns": "http://www.w3.org/2000/svg",
            class: "size-3",
            "width": "12",
            "height": "12",
            "viewBox": "0 0 24 24",
            "fill": "none",
            "stroke": "currentColor",
            "stroke-width": "2",
            "stroke-linecap": "round",
            "stroke-linejoin": "round",
            "aria-hidden": "true",
            path { "d": "M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.74Z" },
            path { "d": "m9 12 2 2 4-4" },
        }
    }
}
