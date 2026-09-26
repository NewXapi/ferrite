//! 网络画布的布局物理与纯渲染辅助：边的读取/显示投影、每帧力学积分
//! （`physics_step`），以及从 store 派生的节点标题与主色调。
//! 供 `NetworkPanel` 每帧与检视器（`inspector`）调用。

use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::network_data::*;
use crate::state::EntityStore;

pub fn edges_read(edges: &Signal<HashSet<(NodeKey, NodeKey)>>) -> HashSet<(NodeKey, NodeKey)> {
    edges.read().clone()
}
// ---- Layout ----

/// 三层可见节点。调度模型全部平铺——不再有渠道聚合卡，
/// 因为渠道是凭证容器（在设置页编辑），不是图上的节点。
/// 待绘制的边。节点不再折叠，故显示边即存储边；第三元保留
/// 原始边（删除时用），维持调用方签名不变。
pub fn display_edge_pairs(
    edges: &HashSet<(NodeKey, NodeKey)>,
) -> Vec<(NodeKey, NodeKey, (NodeKey, NodeKey))> {
    edges.iter().map(|&(u, l)| (u, l, (u, l))).collect()
}

/// One physics frame for the layered graph. x only — y is pinned to the row.
/// One physics frame. Nodes roam the full 2D plane — dropped where you drop
/// them, nothing snaps back to any row. Two boids-style forces only:
/// - rope spring on each link: zero while slack, tugs beyond REST length
/// - all-pairs separation: overlapping nodes push apart like billiard balls
///
/// Returns max speed for the sleep decision; `held` follows the cursor.
pub fn physics_step(
    layers: &[Vec<NodeKey>; 3],
    edges: &[(NodeKey, NodeKey)],
    held: Option<NodeKey>,
    positions: &mut HashMap<NodeKey, (f64, f64)>,
    velocities: &mut HashMap<NodeKey, (f64, f64)>,
) -> f64 {
    // Late-appearing nodes (e.g. after expanding a channel) spawn beside their
    // wired neighbors so separation can push them into the band organically.
    for (l, row) in layers.iter().enumerate() {
        for &k in row.iter() {
            if positions.contains_key(&k) {
                continue;
            }
            let xs: Vec<f64> = edges
                .iter()
                .filter_map(|&(u, lo)| {
                    if lo == k {
                        positions.get(&u).map(|p| p.0)
                    } else if u == k {
                        positions.get(&lo).map(|p| p.0)
                    } else {
                        None
                    }
                })
                .collect();
            let x = if xs.is_empty() {
                VIEW_W / 2.0
            } else {
                xs.iter().sum::<f64>() / xs.len() as f64
            };
            positions.insert(k, (x, ROW_Y[l]));
            velocities.insert(k, (0.0, 0.0));
        }
    }
    let mut forces: HashMap<NodeKey, (f64, f64)> = HashMap::new();
    // Rope spring: zero force while the link is slack, tugs only past REST.
    const REST: f64 = 520.0;
    for &(up, low) in edges {
        // 边可能引用不在任何层的悬空节点（数据里的悬空引用）——跳过, 避免 HashMap 索引 panic。
        let (Some(&pu), Some(&pl)) = (positions.get(&up), positions.get(&low)) else {
            continue;
        };
        let (dx, dy) = (pl.0 - pu.0, pl.1 - pu.1);
        let dist = dx.hypot(dy).max(1.0);
        let stretch = dist - REST;
        if stretch <= 0.0 {
            continue;
        }
        let pull = stretch * 0.05;
        let (fx, fy) = (dx / dist * pull, dy / dist * pull);
        let fu = forces.entry(up).or_default();
        fu.0 += fx;
        fu.1 += fy;
        let fl = forces.entry(low).or_default();
        fl.0 -= fx;
        fl.1 -= fy;
    }
    // Separation, box-aware: nodes are wide pills; horizontal clearance
    // (width + 12) and vertical clearance (height + 8) resolve along the
    // shallower penetration axis. Sweep over all nodes sorted by x: inner
    // loop breaks as soon as the x gap clears, so cost stays
    // O(n log n + collisions) per frame regardless of band geometry.
    let mut sweep: Vec<NodeKey> = layers.iter().flatten().copied().collect();
    sweep.sort_by(|&a, &b| positions[&a].0.partial_cmp(&positions[&b].0).unwrap());
    for i in 0..sweep.len() {
        let a = sweep[i];
        let pa = positions[&a];
        for &b in &sweep[i + 1..] {
            let pb = positions[&b];
            let dx = pb.0 - pa.0;
            if dx >= NODE_W + 12.0 {
                break; // sorted: everything further right is clear of a
            }
            let dy = pb.1 - pa.1;
            let oy = (NODE_H + 8.0) - dy.abs();
            if oy <= 0.0 {
                continue;
            }
            let ox = (NODE_W + 12.0) - dx;
            // Push out along the shallower overlap axis (pure x or pure y).
            let (fx, fy) = if ox < oy {
                (dx.signum() * ox * 0.55, 0.0)
            } else {
                (0.0, dy.signum() * oy * 0.55)
            };
            let fa = forces.entry(a).or_default();
            fa.0 -= fx;
            fa.1 -= fy;
            let fb = forces.entry(b).or_default();
            fb.0 += fx;
            fb.1 += fy;
        }
    }
    let mut max_v = 0.0f64;
    for &k in &sweep {
        let v = velocities.entry(k).or_default();
        if Some(k) == held {
            *v = (0.0, 0.0);
            continue;
        }
        let (fx, fy) = forces.get(&k).copied().unwrap_or((0.0, 0.0));
        v.0 = ((v.0 + fx) * 0.86).clamp(-40.0, 40.0);
        v.1 = ((v.1 + fy) * 0.86).clamp(-40.0, 40.0);
        if v.0.hypot(v.1) < 0.05 {
            *v = (0.0, 0.0); // static friction: kill micro-drift
        }
        let p = positions.get_mut(&k).unwrap();
        let (b0, b1) = band_y(k.layer());
        p.0 = (p.0 + v.0).max(60.0);
        p.1 = (p.1 + v.1).clamp(b0, b1);
        max_v = max_v.max(v.0.hypot(v.1));
    }
    max_v
}

/// 别名/分组标题都从 store 派生，读不到时回退为占位串（那说明 store 还没装上）。
pub fn node_title_from_store(node: NodeKey) -> String {
    let store = use_context::<EntityStore>();
    let view = GraphView::from_store(&store);
    view_title(&view, node)
}

/// 节点的主色调，同样由 GraphView 派生。
pub fn accent_color(node: NodeKey) -> &'static str {
    store_view_color(node)
}

/// view_color 的免费变量版本：直接读 store。
fn store_view_color(node: NodeKey) -> &'static str {
    match node {
        NodeKey::Group(i) => GROUP_PALETTE[i % GROUP_PALETTE.len()],
        NodeKey::Mapping(i) => ALIAS_PALETTE[i % ALIAS_PALETTE.len()],
        NodeKey::Dispatch(_) => "#3f3f46",
    }
}
