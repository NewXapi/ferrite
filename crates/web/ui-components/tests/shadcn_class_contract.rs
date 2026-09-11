//! shadcn class 契约测试：Button/Badge 的 variant/size → (data 键名, class 串) 映射
//! 必须与 shadcn new-york-v4 源码逐字一致（ui/button.tsx:11-30、ui/badge.tsx:9-19）。
//!
//! 为什么逐字断言：这些映射是组件的**视觉契约**——任何一串 class 的漂移都是一次
//! 用户可见的样式回归。改 class 串的唯一合法理由是「同步 shadcn 上游新版本」，
//! 此时本测试的期望值应随上游 diff 一起更新，并在 PR 里贴出上游对照。

use ui_components::components::badge::{BadgeVariant, variant_parts as badge_variant_parts};
use ui_components::components::button::{ButtonSize, ButtonVariant, size_parts, variant_parts};

#[test]
fn button_variant_classes_match_shadcn() {
    // (枚举, data-variant 键名, shadcn ui/button.tsx variant class)
    let cases: [(ButtonVariant, &str, &str); 6] = [
        (
            ButtonVariant::Primary,
            "default",
            "bg-primary text-primary-foreground hover:bg-primary/90",
        ),
        (
            ButtonVariant::Secondary,
            "secondary",
            "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ),
        (
            ButtonVariant::Destructive,
            "destructive",
            "bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
        ),
        (
            ButtonVariant::Outline,
            "outline",
            "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50",
        ),
        (
            ButtonVariant::Ghost,
            "ghost",
            "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
        ),
        (
            ButtonVariant::Link,
            "link",
            "text-primary underline-offset-4 hover:underline",
        ),
    ];
    for (variant, key, class) in cases {
        assert_eq!(variant_parts(variant), (key, class), "{key} 漂移");
    }
}

#[test]
fn button_size_classes_match_shadcn() {
    // (枚举, data-size 键名, shadcn ui/button.tsx size class)
    let cases: [(ButtonSize, &str, &str); 8] = [
        (
            ButtonSize::Xs,
            "xs",
            "h-6 gap-1 rounded-md px-2 text-xs has-[>svg]:px-1.5 [&_svg:not([class*='size-'])]:size-3",
        ),
        (
            ButtonSize::Sm,
            "sm",
            "h-8 gap-1.5 rounded-md px-3 has-[>svg]:px-2.5",
        ),
        (
            ButtonSize::Default,
            "default",
            "h-9 px-4 py-2 has-[>svg]:px-3",
        ),
        (ButtonSize::Lg, "lg", "h-10 rounded-md px-6 has-[>svg]:px-4"),
        (ButtonSize::Icon, "icon", "size-9"),
        (
            ButtonSize::IconXs,
            "icon-xs",
            "size-6 rounded-md [&_svg:not([class*='size-'])]:size-3",
        ),
        (ButtonSize::IconSm, "icon-sm", "size-8"),
        (ButtonSize::IconLg, "icon-lg", "size-10"),
    ];
    for (size, key, class) in cases {
        assert_eq!(size_parts(size), (key, class), "{key} 漂移");
    }
}

#[test]
fn badge_variant_classes_match_shadcn() {
    // (枚举, data-variant 键名, shadcn ui/badge.tsx variant class)
    let cases: [(BadgeVariant, &str, &str); 4] = [
        (
            BadgeVariant::Primary,
            "default",
            "bg-primary text-primary-foreground [a&]:hover:bg-primary/90",
        ),
        (
            BadgeVariant::Secondary,
            "secondary",
            "bg-secondary text-secondary-foreground [a&]:hover:bg-secondary/90",
        ),
        (
            BadgeVariant::Destructive,
            "destructive",
            "bg-destructive text-white focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40 [a&]:hover:bg-destructive/90",
        ),
        (
            BadgeVariant::Outline,
            "outline",
            "border-border text-foreground [a&]:hover:bg-accent [a&]:hover:text-accent-foreground",
        ),
    ];
    for (variant, key, class) in cases {
        assert_eq!(badge_variant_parts(variant), (key, class), "{key} 漂移");
    }
}
