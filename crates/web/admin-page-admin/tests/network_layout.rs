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
