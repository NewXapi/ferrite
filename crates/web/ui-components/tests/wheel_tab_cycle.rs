//! `cycle_index` 循环下标契约测试。
//!
//! 为什么值得测：这是滚轮切换 tab 的唯一算术核心，六个接入点
//! （SectionRail / TopNavBar / TimeframeTabs / StatTabsCard /
//! SegmentedCapsule / lib.rs 的 SectionPill、TopNavMeter）全经它算下一站。
//! 它若算错，表现为「滚到末尾不回开头」或「跳错 tab」——纯交互回归，
//! 没有类型系统能拦，只能靠断言钉住。
//!
//! 覆盖：空集防御、正向/反向越界绕回、`current` 超出 `len` 时不 panic
//! （`rem_euclid` 对任意被除数都落在 `0..len`）、以及 `current == len-1`
//! 这个用户明确要求的「最后一个 → 第一个」边界。

use ui_components::cycle_index;

#[test]
fn empty_set_returns_zero_instead_of_panicking() {
    // 空 tab 集：调用方不应传入，但真传了也不能 panic（除零）。
    assert_eq!(cycle_index(0, 0, 1), 0, "空集正向");
    assert_eq!(cycle_index(0, 0, -1), 0, "空集反向");
}

#[test]
fn forward_wraps_last_to_first() {
    // 用户原话：「到最后一个的时候能切换到第一个」。
    assert_eq!(cycle_index(3, 2, 1), 0, "末项 +1 绕回首项");
    assert_eq!(cycle_index(4, 3, 1), 0, "四档末项绕回");
}

#[test]
fn backward_wraps_first_to_last() {
    assert_eq!(cycle_index(3, 0, -1), 2, "首项 -1 绕回末项");
}

#[test]
fn interior_steps_without_wrapping() {
    assert_eq!(cycle_index(4, 1, 1), 2, "中间正向");
    assert_eq!(cycle_index(4, 2, -1), 1, "中间反向");
}

#[test]
fn single_item_set_stays_put() {
    // 只有一项时无论方向都停原地（绕回自身）。
    assert_eq!(cycle_index(1, 0, 1), 0, "单项正向");
    assert_eq!(cycle_index(1, 0, -1), 0, "单项反向");
}

#[test]
fn out_of_range_current_is_clamped_not_panicking() {
    // 调用方传入越界 current（如 SegmentedCapsule 的 active 由外部给出）时，
    // rem_euclid 必须仍返回合法下标，而不是算术溢出或返回越界值。
    for len in 1usize..=5 {
        for current in [len, len + 1, len + 7] {
            let next = cycle_index(len, current, 1);
            assert!(
                next < len,
                "len={len} current={current} 越界: 得到 {next}"
            );
        }
    }
}
