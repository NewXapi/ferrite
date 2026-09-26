//! class 契约测试：Button/Badge 的 variant/size → (data 键名, class 串) 映射逐字钉死。
//!
//! 视觉契约基准变更：Button 的 variant/size 串已由 shadcn new-york-v4 上游切换为
//! dsh（deepseek-harness）规格，对照文档在 `todo/web-ui-reference/dsh-visual-spec.md`
//! §4.1（PR 侧附映射摘要）；改这批串的唯一合法理由是「同步该规格文档的修订」，
//! 此时本测试期望值应随规格 diff 一起更新。Badge 不在本次范围，仍钉死 shadcn
//! new-york-v4 上游（ui/badge.tsx:9-19）。
//!
//! 为什么逐字断言：这些映射是组件的**视觉契约**——任何一串 class 的漂移都是一次
//! 用户可见的样式回归。

use ui_components::badge::{BadgeVariant, variant_parts as badge_variant_parts};
use ui_components::button::{ButtonSize, ButtonVariant, size_parts, variant_parts};

#[test]
fn button_variant_classes_match_dsh_basis() {
    // (枚举, data-variant 键名, dsh 基准 variant class，对照 dsh-visual-spec.md §4.1)
    let cases: [(ButtonVariant, &str, &str); 6] = [
        (
            ButtonVariant::Primary,
            "default",
            "bg-primary text-primary-foreground hover:bg-primary/90",
        ),
        (
            ButtonVariant::Secondary,
            "secondary",
            "bg-secondary text-secondary-foreground hover:bg-secondary-hover",
        ),
        (
            ButtonVariant::Destructive,
            "destructive",
            "bg-destructive text-foreground hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
        ),
        (
            ButtonVariant::Outline,
            "outline",
            "border bg-transparent hover:bg-accent",
        ),
        (
            ButtonVariant::Ghost,
            "ghost",
            "hover:bg-accent active:bg-primary/15",
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
fn button_size_classes_match_dsh_basis() {
    // (枚举, data-size 键名, dsh 基准 size class，对照 dsh-visual-spec.md §4.1)
    let cases: [(ButtonSize, &str, &str); 8] = [
        (
            ButtonSize::Xs,
            "xs",
            "h-6 gap-1 px-2 text-xs has-[>svg]:px-1.5 [&_svg:not([class*='size-'])]:size-3",
        ),
        (
            ButtonSize::Sm,
            "sm",
            "h-7 gap-1 px-2.5 text-xs has-[>svg]:px-2",
        ),
        (
            ButtonSize::Default,
            "default",
            "h-9 px-3.5 py-2 has-[>svg]:px-3",
        ),
        (ButtonSize::Lg, "lg", "h-10 px-6 has-[>svg]:px-4"),
        (ButtonSize::Icon, "icon", "size-9"),
        (
            ButtonSize::IconXs,
            "icon-xs",
            "size-6 [&_svg:not([class*='size-'])]:size-3",
        ),
        (ButtonSize::IconSm, "icon-sm", "size-7"),
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
            "bg-destructive text-foreground focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40 [a&]:hover:bg-destructive/90",
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
