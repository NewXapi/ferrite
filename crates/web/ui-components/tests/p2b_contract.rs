//! PR2b 契约测试：Select 的 data-state 两态动画 class、Toast 的 variant→class
//! 映射与秒级清扫逻辑。
//!
//! 为什么逐字断言：这些映射是组件的**视觉契约**——class 串必须与 shadcn
//! new-york-v4 源码一致（ui/select.tsx:64 的 data-state 动画段；sonner normal
//! 配色 border=--border / 文字=popover-foreground，destructive 用 destructive
//! token）。改动的唯一合法理由是「同步 shadcn 上游新版本」，此时期望值应随
//! 上游 diff 一起更新，并在 PR 里贴出上游对照。

use ui_components::components::select::state_parts;
use ui_components::components::toast::{
    TOAST_TTL_SECS, ToastItem, ToastVariant, sweep_toasts, variant_parts as toast_variant_parts,
};

#[test]
fn select_state_parts_match_shadcn() {
    // shadcn ui/select.tsx SelectContent：data-[state=open] 入场三段 /
    // data-[state=closed] 退场三段，逐字。
    assert_eq!(
        state_parts(true),
        "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95"
    );
    assert_eq!(
        state_parts(false),
        "data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95"
    );
}

#[test]
fn toast_variant_classes_match_sonner() {
    // (变体, data-variant 键名, 叠加 class)：normal 配色供 default/success，
    // destructive 边框与文字走 destructive token。
    let cases: [(ToastVariant, &str, &str); 3] = [
        (
            ToastVariant::Default,
            "default",
            "border-border text-popover-foreground",
        ),
        (
            ToastVariant::Success,
            "success",
            "border-border text-popover-foreground",
        ),
        (
            ToastVariant::Destructive,
            "destructive",
            "border-destructive text-destructive",
        ),
    ];
    for (variant, key, class) in cases {
        assert_eq!(toast_variant_parts(variant), (key, class), "{key} 漂移");
    }
}

#[test]
fn sweep_toasts_expires_after_ttl() {
    // 清扫语义：新条目带满 TTL，清扫 TOAST_TTL_SECS-1 轮后仍存活，
    // 第 TOAST_TTL_SECS 轮被移除（1 tick ≈ 1 秒）。
    let make_item = |id: u64| ToastItem {
        id,
        title: format!("toast-{id}"),
        description: Some("desc".into()),
        variant: ToastVariant::Default,
        ttl_ticks: TOAST_TTL_SECS,
    };
    let mut queue = vec![make_item(1), make_item(2)];
    for _ in 0..(TOAST_TTL_SECS - 1) {
        queue = sweep_toasts(queue);
    }
    assert_eq!(queue.len(), 2, "未满 TTL 的条目不应被清扫");
    queue = sweep_toasts(queue);
    assert!(queue.is_empty(), "满 TTL 后应全部移除");
}

#[test]
fn sweep_toasts_preserves_order_and_ids() {
    // 清扫只做倒计时与移除，不应改变队列顺序或篡改条目内容（点击关闭按 id 定位）。
    let queue = vec![
        ToastItem {
            id: 7,
            title: "a".into(),
            description: None,
            variant: ToastVariant::Success,
            ttl_ticks: 2,
        },
        ToastItem {
            id: 3,
            title: "b".into(),
            description: None,
            variant: ToastVariant::Destructive,
            ttl_ticks: 1,
        },
    ];
    let swept = sweep_toasts(queue);
    assert_eq!(swept.len(), 1, "ttl=1 的条目应在本轮被移除");
    assert_eq!(swept[0].id, 7);
    assert_eq!(swept[0].ttl_ticks, 1, "幸存条目应恰好减一");
    assert_eq!(swept[0].variant, ToastVariant::Success);
}
