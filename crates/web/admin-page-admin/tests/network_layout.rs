//! network.rs 的布局/几何纯函数(从 src/network.rs 内联测试迁出)。
//! 这些项以 `#[doc(hidden)] pub` 暴露,仅为满足"测试统一放 tests/"的项目
//! 约定;非公共 API,勿在 crate 之外使用。

use admin_page_admin::network::{
    GraphView, MARGIN, NODE_H, NODE_W, NodeKey, VIEW_H, VIEW_W, bezier, cubic_at, dodge_frac,
    ease_out_quint, fit_view, initial_positions, visible_layers_of,
};

// 测试专用快照构造: 不经过 EntityStore (Signal 需要 Dioxus runtime),
// 直接摆数据 —— GraphView 是纯结构体, 可在裸测试环境里构造。
fn test_view() -> GraphView {
    GraphView {
        groups: vec!["default".into(), "claude".into(), "vip".into()],
        aliases: vec!["gpt-4o".into(), "gpt-5".into(), "claude-sonnet-4".into()],
        channels: vec!["OpenAI".into(), "Claude".into(), "OneAPI".into()],
        dispatch: vec![
            (0usize, "gpt-4o".to_string()),
            (0, "gpt-5".to_string()),
            (1, "claude-sonnet-4".to_string()),
        ],
    }
}

/// 落位要覆盖全部可见节点且不越出画布。
#[test]
fn initial_layout_covers_all_visible_nodes_in_view() {
    let view = test_view();
    let pos = initial_positions(&view);
    for layer in visible_layers_of(&view) {
        for k in &layer {
            let (x, y) = pos[k];
            assert!(
                (MARGIN..=VIEW_W - MARGIN).contains(&x),
                "{k:?} x={x} out of view"
            );
            assert!((0.0..=VIEW_H).contains(&y), "{k:?} y={y} out of view");
        }
    }
}

#[test]
fn initial_layout_no_same_layer_overlap() {
    let view = test_view();
    let pos = initial_positions(&view);
    let all: Vec<NodeKey> = visible_layers_of(&view).into_iter().flatten().collect();
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            let (pa, pb) = (pos[a], pos[b]);
            let clash = (pb.0 - pa.0).abs() < NODE_W && (pb.1 - pa.1).abs() < NODE_H;
            assert!(!clash, "{a:?} {pa:?} overlaps {b:?} {pb:?}");
        }
    }
}

#[test]
fn bezier_start_end_match_input() {
    let a = (10.0, 20.0);
    let b = (110.0, 220.0);
    let d = bezier(a, b);
    // "M 10 20 C ... 110 220" — 首尾坐标要出现在 path 里
    assert!(d.starts_with("M 10 20"), "path: {d}");
    assert!(d.ends_with("110 220"), "path: {d}");
}

#[test]
fn cubic_at_endpoints() {
    let p0 = (0.0, 0.0);
    let p3 = (1.0, 0.0);
    let p1 = (0.0, 0.0);
    let p2 = (1.0, 0.0);
    assert_eq!(cubic_at(p0, p1, p2, p3, 0.0), p0);
    assert_eq!(cubic_at(p0, p1, p2, p3, 1.0), p3);
    let mid = cubic_at(p0, p1, p2, p3, 0.5);
    assert!(mid.1.abs() < 1e-9, "零偏置 mid y 应为 0, got {}", mid.1);
}

#[test]
fn ease_out_quint_monotonic_endpoints() {
    assert_eq!(ease_out_quint(0.0), 0.0);
    assert!((ease_out_quint(1.0) - 1.0).abs() < 1e-9);
    let samples: Vec<f64> = (0..=10).map(|i| ease_out_quint(i as f64 / 10.0)).collect();
    for w in samples.windows(2) {
        assert!(w[0] <= w[1], "ease_out_quint 必须单调递增");
    }
}

#[test]
fn dodge_frac_zero_when_no_blockers() {
    let a = (0.0, 0.0);
    let b = (200.0, 0.0);
    assert_eq!(dodge_frac(a, b, &[]), 0.0, "无遮挡时首候选 0.0 即无碰撞");
}

#[test]
fn fit_view_centers_point_set() {
    let pts = vec![(0.0, 0.0), (100.0, 0.0), (50.0, 50.0)];
    let ((cx, cy), _z) = fit_view(&pts);
    let (sum_x, sum_y): (f64, f64) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    // 中心应接近点集质心(近似; 允许 zoom/pan 常数误差)
    let expect_cx = sum_x / pts.len() as f64;
    let expect_cy = sum_y / pts.len() as f64;
    assert!(
        (cx - expect_cx).abs() < VIEW_W,
        "cx={cx} too far from {expect_cx}"
    );
    assert!(
        (cy - expect_cy).abs() < VIEW_H,
        "cy={cy} too far from {expect_cy}"
    );
}

// ---------- 无连线节点 spawn 散开(spread_isolated_x) ----------

use admin_page_admin::network::spread_isolated_x;

/// N 个同层无连线节点 spawn 后 x 互不相等、两两间距 ≥ 段宽的一半,
/// 且全部落在画布安全区内。这是聚线 bug 的几何断言:原先统一落
/// VIEW_W/2,完全重合的位置让分离力为零,节点永久叠死在中心竖线上。
#[test]
fn spread_isolated_x_unique_and_spaced_for_n_nodes() {
    let span = VIEW_W - 2.0 * MARGIN;
    for count in 2..=12usize {
        let xs: Vec<f64> = (0..count)
            .map(|slot| spread_isolated_x(slot, count))
            .collect();
        // 全部在安全区内
        for (i, x) in xs.iter().enumerate() {
            assert!(
                (MARGIN..=VIEW_W - MARGIN).contains(x),
                "count={count} slot={i} x={x} 越出画布安全区"
            );
        }
        // 两两间距:≥ 0.5 段宽(jitter 幅度 ≤ 段宽 1/8,理论下界 0.75 段宽)
        let min_gap = span / count as f64 * 0.5;
        for (i, a) in xs.iter().enumerate() {
            for b in &xs[i + 1..] {
                assert!(
                    (a - b).abs() >= min_gap,
                    "count={count} 间距 {:.2} < 下限 {:.2}",
                    (a - b).abs(),
                    min_gap
                );
            }
        }
    }
}

/// 单个无连线节点不再落在画布中心(有邻居节点的重心锚 ≈ 中心,
/// 精确重合同样会让分离力归零)。
#[test]
fn spread_isolated_x_single_node_avoids_center() {
    let x = spread_isolated_x(0, 1);
    let center = VIEW_W / 2.0;
    assert!(
        (x - center).abs() > 0.1 * (VIEW_W - 2.0 * MARGIN),
        "单节点 x={x} 过于贴近中心 {center},仍会与重心锚叠死"
    );
}

/// 同参数多次调用结果一致(散列 jitter 不引入 RNG,布局可复现)。
#[test]
fn spread_isolated_x_is_deterministic() {
    for count in [1usize, 3, 7, 11] {
        for slot in 0..count {
            assert_eq!(
                spread_isolated_x(slot, count),
                spread_isolated_x(slot, count)
            );
        }
    }
}

/// 槽位序号越大 x 单调不降(散开保持稳定次序,视觉不跳变)。
#[test]
fn spread_isolated_x_keeps_slot_order() {
    for count in [2usize, 5, 9] {
        let xs: Vec<f64> = (0..count)
            .map(|slot| spread_isolated_x(slot, count))
            .collect();
        for w in xs.windows(2) {
            assert!(w[0] < w[1], "count={count}: 槽位次序被 jitter 打乱 {xs:?}");
        }
    }
}
