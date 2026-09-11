//! [`tavern_state::dock`] 纯函数层单测。
//!
//! 覆盖：跨区移动（源区激活切换 + order 追加末尾）、同区 reorder 越界 clamp、
//! dock_tab 去重、split 越界 clamp、serialize/deserialize 往返与损坏输入。

use tavern_state::dock::{
    DockItem, DockLayout, Side, SplitRatio, Zone, deserialize, dock_tab, move_item,
    reorder_in_zone, serialize, set_split, set_zone_collapsed,
};

fn item(id: &str, zone: Zone, order: u32, enabled: bool) -> DockItem {
    DockItem {
        id: id.to_string(),
        zone,
        order,
        enabled,
        title: id.to_string(),
    }
}

fn layout_with_active(items: Vec<DockItem>, active: &[(Zone, Option<&str>)]) -> DockLayout {
    // 从 total map（全四区键恒存在）出发，再覆盖测试指定的激活项；
    // 与 Default 的不变量保持一致。
    let mut active_by_zone = DockLayout::default().active_by_zone;
    for (z, id) in active {
        active_by_zone.insert(*z, id.map(String::from));
    }
    DockLayout {
        items,
        active_by_zone,
        ..Default::default()
    }
}

/// 测跨区移动：order 追加到目标区末尾（目标区最大 order + 1），且 enabled 被置 true。
#[test]
fn move_item_cross_zone_appends_order_tail() {
    let mut layout = layout_with_active(
        vec![
            item("a", Zone::LeftTop, 0, true),
            item("b", Zone::RightTop, 5, true),
            item("c", Zone::RightTop, 6, true),
        ],
        &[],
    );
    move_item(&mut layout, "a", Zone::RightTop);

    let a = layout.items.iter().find(|i| i.id == "a").unwrap();
    // a 追加到 RightTop 末尾：b=5, c=6 → a 应为 7。
    assert_eq!(a.zone, Zone::RightTop);
    assert_eq!(a.order, 7);
    // 移动即启用（Flint 语义：拖进某区即停靠）。
    assert!(a.enabled);
}

/// 测跨区移动 + 源区激活切换：被移走的项是源区激活项时，激活切到剩余启用项中
/// order 最大者；源区无剩余启用项时置 None。
#[test]
fn move_item_switches_active_to_last_remaining() {
    let mut layout = layout_with_active(
        vec![
            item("x", Zone::LeftBottom, 0, true),
            item("y", Zone::LeftBottom, 2, true),
        ],
        &[(Zone::LeftBottom, Some("y"))],
    );
    move_item(&mut layout, "y", Zone::RightBottom);

    // 源区 LeftBottom 激活原为 y，y 被移走后剩 x（order 0 且唯一）→ 激活切 x。
    assert_eq!(
        layout.active_by_zone[&Zone::LeftBottom].as_deref(),
        Some("x")
    );
    // 目标区激活不受影响（原本无激活 → 仍无激活）。
    assert_eq!(layout.active_by_zone.get(&Zone::RightBottom), Some(&None));

    // 再把 x 也移走：源区无剩余启用项 → 激活置 None（而非删除键或 panic）。
    move_item(&mut layout, "x", Zone::RightTop);
    assert_eq!(layout.active_by_zone[&Zone::LeftBottom], None);
}

/// 测同区移动是 no-op：reorder_in_zone 才负责区内重排，move_item 同区不应改 order/zone。
#[test]
fn move_item_same_zone_is_noop() {
    let mut layout = layout_with_active(vec![item("a", Zone::LeftTop, 3, true)], &[]);
    move_item(&mut layout, "a", Zone::LeftTop);
    let a = &layout.items[0];
    assert_eq!(a.order, 3, "同区移动不得重排 order");
    assert_eq!(a.zone, Zone::LeftTop);

    // 找不到 item 也是 no-op，不 panic。
    move_item(&mut layout, "ghost", Zone::RightTop);
    assert_eq!(layout.items.len(), 1);
}

/// 测区内 reorder：按启用项 order 升序序列换位并重写 order 为 0..=n-1；
/// to_index 越界 clamp 到 [0, n-1]，from_index 越界 no-op。
#[test]
fn reorder_in_zone_stable_swap_and_clamp() {
    let mut layout = layout_with_active(
        vec![
            item("a", Zone::RightTop, 0, true),
            item("b", Zone::RightTop, 1, true),
            item("c", Zone::RightTop, 2, true),
            item("d", Zone::RightBottom, 0, true),
        ],
        &[],
    );
    reorder_in_zone(&mut layout, Zone::RightTop, 2, 0);

    // 序列 [a,b,c] 中 index2(c) 移到 index0 → [c,a,b]，order 重写 0,1,2。
    // 显式传 layout 而非闭包捕获，否则后续 &mut layout 调用与闭包的生命周期冲突（E0502）。
    let ord = |l: &DockLayout, id: &str| l.items.iter().find(|i| i.id == id).unwrap().order;
    assert_eq!(ord(&layout, "c"), 0);
    assert_eq!(ord(&layout, "a"), 1);
    assert_eq!(ord(&layout, "b"), 2);
    // 其他区不受影响。
    assert_eq!(ord(&layout, "d"), 0);

    // to_index 越界（99）clamp 到 n-1=2：把 a（index1）移到末尾 → [c,b,a]。
    reorder_in_zone(&mut layout, Zone::RightTop, 1, 99);
    assert_eq!(ord(&layout, "c"), 0);
    assert_eq!(ord(&layout, "b"), 1);
    assert_eq!(ord(&layout, "a"), 2);

    // from_index 越界（5 >= n=3）no-op，order 不变。
    let before = layout.items.clone();
    reorder_in_zone(&mut layout, Zone::RightTop, 5, 0);
    assert_eq!(layout.items, before);
}

/// 测 dock_tab 去重：同 id 已存在时更新 zone 并保留原 order；
/// order 与目标区冲突时顺延到目标区末尾。
#[test]
fn dock_tab_dedupes_and_resolves_order_conflict() {
    let mut layout = layout_with_active(
        vec![
            item("model", Zone::LeftTop, 1, true),
            item("notes", Zone::RightTop, 1, true),
        ],
        &[],
    );
    // model 拖进 RightTop：已有 order=1 与 notes 冲突 → 顺延到末尾（max+1=2）。
    dock_tab(&mut layout, item("model", Zone::RightTop, 1, true));
    let model = layout.items.iter().find(|i| i.id == "model").unwrap();
    assert_eq!(model.zone, Zone::RightTop);
    assert_eq!(model.order, 2);
    // 区内不产生重复 id。
    assert_eq!(layout.items.iter().filter(|i| i.id == "model").count(), 1);

    // 新 id 直接插入（zone/order 按传入值）。
    dock_tab(&mut layout, item("sessions", Zone::LeftBottom, 0, true));
    let sessions = layout.items.iter().find(|i| i.id == "sessions").unwrap();
    assert_eq!(sessions.zone, Zone::LeftBottom);
    assert_eq!(sessions.order, 0);
}

/// 测 set_split 越界 clamp：比例被限制在 0.05..=0.95，且作用于对应侧列。
#[test]
fn set_split_clamps_out_of_range() {
    let mut layout = DockLayout::default();
    set_split(&mut layout, Side::Left, SplitRatio::new_unchecked(0.3));
    set_split(&mut layout, Side::Right, SplitRatio::new_unchecked(0.9));

    set_split(&mut layout, Side::Left, SplitRatio::new_unchecked(0.01));
    set_split(&mut layout, Side::Right, SplitRatio::new_unchecked(0.99));

    assert_eq!(layout.split_left.value(), 0.05, "低于下限 clamp 到 0.05");
    assert_eq!(layout.split_right.value(), 0.95, "高于上限 clamp 到 0.95");

    // NaN 不合法，归为默认 0.5，避免持久化出无效值。
    set_split(&mut layout, Side::Left, SplitRatio::new_unchecked(f32::NAN));
    assert_eq!(layout.split_left.value(), 0.5);
}

/// 测 set_zone_collapsed：true 时该区全禁用且激活置 None；false 时全恢复启用。
#[test]
fn set_zone_collapsed_toggles_enabled() {
    let mut layout = layout_with_active(
        vec![
            item("a", Zone::LeftTop, 0, true),
            item("b", Zone::LeftTop, 1, false),
            item("c", Zone::RightTop, 0, true),
        ],
        &[(Zone::LeftTop, Some("a"))],
    );
    set_zone_collapsed(&mut layout, Zone::LeftTop, true);
    let ids: Vec<(&str, bool)> = layout
        .items
        .iter()
        .filter(|i| i.zone == Zone::LeftTop)
        .map(|i| (i.id.as_str(), i.enabled))
        .collect();
    assert_eq!(ids, vec![("a", false), ("b", false)], "收起 = 该区全部禁用");
    assert_eq!(
        layout.active_by_zone[&Zone::LeftTop],
        None,
        "收起时激活置 None"
    );
    // 其他区不受影响。
    assert!(layout.items.iter().find(|i| i.id == "c").unwrap().enabled);

    set_zone_collapsed(&mut layout, Zone::LeftTop, false);
    // 展开 = 全部恢复启用（不记忆原先 b 的禁用状态，这是最简可单测语义）。
    assert!(layout.items.iter().find(|i| i.id == "b").unwrap().enabled);
}

/// 测 serialize/deserialize 往返一致性：同结构（含 split、active）往返不变。
#[test]
fn serialize_roundtrip_preserves_layout() {
    let mut layout = layout_with_active(
        vec![
            item("sessions", Zone::LeftTop, 0, true),
            item("model", Zone::RightBottom, 1, false),
        ],
        &[(Zone::LeftTop, Some("sessions")), (Zone::RightBottom, None)],
    );
    layout.split_left = SplitRatio::new_unchecked(0.3);
    layout.split_right = SplitRatio::new_unchecked(0.7);

    let v = serialize(&layout);
    let back = deserialize(&v).expect("合法 JSON 反序列化不应报错");
    assert_eq!(back, layout, "往返应保持完全一致");
}

/// 测 deserialize 损坏输入返回 Err 而非 panic：缺 items、items 非数组、未知 zone。
#[test]
fn deserialize_rejects_corrupt_input() {
    // 缺 items 字段。
    assert!(deserialize(&serde_json::json!({})).is_err());
    // items 不是数组。
    assert!(deserialize(&serde_json::json!({"items": 42})).is_err());
    // item zone 未知。
    assert!(
        deserialize(&serde_json::json!({
            "items": [{"id": "a", "zone": "nowhere", "order": 0, "enabled": true}]
        }))
        .is_err()
    );
    // 完全非对象输入。
    assert!(deserialize(&serde_json::json!(123)).is_err());

    // 容忍未知字段 + 缺省字段：只给 items 也能解析（split 默认 0.5）。
    let v = serde_json::json!({
        "unknown_extra": "ignored",
        "items": [{"id": "a", "zone": "left-top", "title": "A"}]
    });
    let back = deserialize(&v).expect("缺省字段应容忍");
    assert_eq!(back.items.len(), 1);
    assert_eq!(back.items[0].order, 0, "缺 order 默认 0");
    assert!(back.items[0].enabled, "缺 enabled 默认 true");
    assert_eq!(back.split_left, SplitRatio::DEFAULT);
}
