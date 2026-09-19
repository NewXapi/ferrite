//! StatCard 视觉契约测试：`StatSize` 变体 → 三段 class 串映射逐字钉死。
//!
//! 为什么逐字断言：这些串是组件的**视觉契约**——收敛前它们分散在
//! `admin-page-admin/groups.rs`、`admin-page-users/panel.rs`、
//! `admin-page-account/keys.rs`（Sm，三处逐字节相同）与
//! `admin-page-account/usage_logs.rs`（Lg）四个私有定义里；任何一串漂移都是
//! 一次用户可见的样式回归。期望值即各处原定义的 class 原文，改它的唯一合法
//! 理由是「同步设计规格修订」，此时本测试期望值应随规格 diff 一起更新。
//!
//! 缺省档（`StatSize::default()`）= Sm：调用点省略 `size` 时渲染结果必须与
//! 原先 groups/users/keys 三处逐字一致——这是迁移「不改调用点也行」的前提。

use ui_components::components::stat_card::{StatSize, size_parts};

#[test]
fn stat_size_parts_pin_original_classes() {
    // Sm：原 groups.rs:550（= panel.rs:413 = keys.rs:273）逐字。
    assert_eq!(
        size_parts(StatSize::Sm),
        (
            "rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 transition-colors hover:border-zinc-600",
            "text-xl font-semibold tracking-tight text-white",
            "mt-0.5 text-xs text-zinc-500",
        ),
        "Sm 档 class 漂移",
    );
    // Lg：原 usage_logs.rs:283 逐字（注意 padding 与 hover/transition 顺序与 Sm 不同，保留原序）。
    assert_eq!(
        size_parts(StatSize::Lg),
        (
            "rounded-xl border border-zinc-800 bg-zinc-900/60 px-5 py-4 hover:border-zinc-600 transition-colors",
            "text-2xl font-semibold text-zinc-100 tabular-nums",
            "mt-1 text-xs text-zinc-500",
        ),
        "Lg 档 class 漂移",
    );
}

#[test]
fn stat_size_default_is_sm() {
    // 调用点省略 size 时的缺省档 = Sm：groups/users/keys 三处历史标记。
    assert_eq!(StatSize::default(), StatSize::Sm);
    // slug 与变体一一对应，供 data 钩子断言。
    assert_eq!(StatSize::Sm.key(), "sm");
    assert_eq!(StatSize::Lg.key(), "lg");
}
