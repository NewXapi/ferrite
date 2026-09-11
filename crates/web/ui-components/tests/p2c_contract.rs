//! PR2c 契约测试：Sidebar 的开合 state 映射、MenuButton 的 active 映射与 Sheet 的
//! 四方向 side_parts 映射，必须与 shadcn new-york-v4 源码逐字一致
//! （ui/sidebar.tsx:477 sidebarMenuButtonVariants 核心段；ui/sheet.tsx SheetContent
//! side 分支）。
//!
//! 为什么逐字断言：这些映射是组件的**视觉契约**——任何一串 class 的漂移都是一次
//! 用户可见的样式回归。改 class 串的唯一合法理由是「同步 shadcn 上游新版本」，
//! 此时本测试的期望值应随上游 diff 一起更新，并在 PR 里贴出上游对照。

use ui_components::components::sheet::{SheetSide, side_parts};
use ui_components::components::sidebar::{menu_button_parts, state_parts as sidebar_state_parts};

#[test]
fn sidebar_menu_button_active_parts_match_shadcn() {
    // (active → data-active 值, sidebarMenuButtonVariants 核心段)：两态 class 同串，
    // 激活样式全部走 data-[active=true] 属性选择器（shadcn 原样）。
    let class = "flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-left text-sm hover:bg-sidebar-accent hover:text-sidebar-accent-foreground active:bg-sidebar-accent data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0";
    assert_eq!(menu_button_parts(true), ("true", class), "active=true 漂移");
    assert_eq!(
        menu_button_parts(false),
        ("false", class),
        "active=false 漂移"
    );
}

#[test]
fn sidebar_state_parts_match_shadcn() {
    // (open → data-state 值, transform class)：开启就位、关闭滑出左缘（offcanvas），
    // transition 由根 class 的 transition-transform duration-200 ease-linear 承担。
    assert_eq!(sidebar_state_parts(true), ("open", "translate-x-0"));
    assert_eq!(sidebar_state_parts(false), ("closed", "-translate-x-full"));
}

#[test]
fn sheet_side_parts_match_shadcn() {
    // (方向, 定位 class, 关态位移 class, 开态 class)：定位段逐字取自
    // ui/sheet.tsx SheetContent 的 side 分支；关态位移替代 tw-animate-css 的
    // slide-out-to-* / slide-in-from-* 动画段（常驻元素 transform 直写）。
    let cases: [(SheetSide, &str, &str, &str); 4] = [
        (
            SheetSide::Top,
            "inset-x-0 top-0 h-auto border-b",
            "-translate-y-full",
            "translate-y-0",
        ),
        (
            SheetSide::Right,
            "inset-y-0 right-0 h-full w-3/4 border-l sm:max-w-sm",
            "translate-x-full",
            "translate-x-0",
        ),
        (
            SheetSide::Bottom,
            "inset-x-0 bottom-0 h-auto border-t",
            "translate-y-full",
            "translate-y-0",
        ),
        (
            SheetSide::Left,
            "inset-y-0 left-0 h-full w-3/4 border-r sm:max-w-sm",
            "-translate-x-full",
            "translate-x-0",
        ),
    ];
    for (side, position, closed_shift, open_shift) in cases {
        assert_eq!(
            side_parts(side),
            (position, closed_shift, open_shift),
            "{side:?} 漂移"
        );
    }
}
