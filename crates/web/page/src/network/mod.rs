use std::collections::{HashMap, HashSet, VecDeque};

use dioxus::prelude::*;

use crate::store::EntityStore;

/// 3-layer interactive routing editor, Mini-Metro flavored:
///   groups (top) ←→ model mappings (middle) ←→ channel models (bottom).
///
/// Controls:
/// - drag empty canvas: pan; mouse wheel: zoom (RTS top-down feel)
/// - drag node body: reposition (RTS-style free placement, session-scoped)
/// - drag from the small port dot(s) on a node to an adjacent-layer node: wire
/// - click a wire: delete it
/// - click a node (no drag): focus — dim everything except direct neighbors
/// - click a channel card: expand/collapse into its model nodes
/// - ctrl+click a node: toggle it in/out of the multi-selection
/// - shift+drag empty canvas: marquee-select (replaces selection)
/// - drag any selected node: the whole selection follows
/// - drag a port while ≥2 same-layer nodes are selected: wires every selected
///   source with a legal edge to the drop target
/// - 「适配」button: zoom/pan to fit all visible nodes
///
/// Sample data; persistence + real /api/channel + /api/group come later.

struct GroupDef {
    name: &'static str,
    color: &'static str,
}

struct AliasDef {
    name: &'static str,
    color: &'static str,
}

struct DispatchDef {
    channel: usize,
    name: &'static str,
}

const GROUPS: &[GroupDef] = &[
    GroupDef {
        name: "default",
        color: "#e5484d",
    },
    GroupDef {
        name: "claude",
        color: "#3e9bff",
    },
    GroupDef {
        name: "gpt-5",
        color: "#30a46c",
    },
    GroupDef {
        name: "vip",
        color: "#e5c558",
    },
];

const ALIASES: &[AliasDef] = &[
    AliasDef {
        name: "gpt-4o",
        color: "#f472b6",
    },
    AliasDef {
        name: "gpt-5",
        color: "#a78bfa",
    },
    AliasDef {
        name: "claude-sonnet-4",
        color: "#22d3ee",
    },
    AliasDef {
        name: "gemini-2.5-pro",
        color: "#fb923c",
    },
];

const CHANNELS: &[&str] = &[
    "OpenAI 官方",
    "Azure East",
    "OneAPI 上游",
    "Claude 官网",
    "AWS Bedrock",
    "Gemini",
];

const DISPATCH: &[DispatchDef] = &[
    DispatchDef {
        channel: 0,
        name: "gpt-4o",
    },
    DispatchDef {
        channel: 0,
        name: "gpt-5",
    },
    DispatchDef {
        channel: 1,
        name: "gpt-4o",
    },
    DispatchDef {
        channel: 2,
        name: "gpt-4o",
    },
    DispatchDef {
        channel: 2,
        name: "gpt-5",
    },
    DispatchDef {
        channel: 2,
        name: "claude-sonnet-4",
    },
    DispatchDef {
        channel: 3,
        name: "claude-sonnet-4",
    },
    DispatchDef {
        channel: 4,
        name: "claude-sonnet-4",
    },
    DispatchDef {
        channel: 5,
        name: "gemini-2.5-pro",
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum NodeKey {
    Group(usize),
    Mapping(usize),
    /// 调度模型：渠道下真实可用的上游模型。
    Dispatch(usize),
}

impl NodeKey {
    fn layer(self) -> u8 {
        match self {
            NodeKey::Group(_) => 0,
            NodeKey::Mapping(_) => 1,
            NodeKey::Dispatch(_) => 2,
        }
    }
}

fn color(key: NodeKey) -> &'static str {
    match key {
        NodeKey::Group(i) => GROUPS[i].color,
        NodeKey::Mapping(i) => ALIASES[i].color,
        NodeKey::Dispatch(_) => "#3f3f46",
    }
}

fn node_title(key: NodeKey) -> String {
    match key {
        NodeKey::Group(i) => GROUPS[i].name.into(),
        NodeKey::Mapping(i) => ALIASES[i].name.into(),
        NodeKey::Dispatch(i) => DISPATCH[i].name.into(),
    }
}

fn subtitle(key: NodeKey) -> String {
    match key {
        NodeKey::Dispatch(i) => CHANNELS[DISPATCH[i].channel].into(),
        _ => String::new(),
    }
}

// Seed wires: mapping↔group and model↔mapping, mirroring the mock channels.
const SEED_EDGES: &[(NodeKey, NodeKey)] = &[
    (NodeKey::Group(0), NodeKey::Mapping(0)), // default — gpt-4o
    (NodeKey::Group(0), NodeKey::Mapping(3)), // default — gemini-2.5-pro
    (NodeKey::Group(1), NodeKey::Mapping(2)), // claude — claude-sonnet-4
    (NodeKey::Group(2), NodeKey::Mapping(1)), // gpt-5 — gpt-5
    (NodeKey::Group(3), NodeKey::Mapping(0)), // vip — gpt-4o
    (NodeKey::Group(3), NodeKey::Mapping(1)), // vip — gpt-5
    (NodeKey::Group(3), NodeKey::Mapping(2)), // vip — claude-sonnet-4
    (NodeKey::Group(3), NodeKey::Mapping(3)), // vip — gemini-2.5-pro
    (NodeKey::Mapping(0), NodeKey::Dispatch(0)),
    (NodeKey::Mapping(0), NodeKey::Dispatch(2)),
    (NodeKey::Mapping(0), NodeKey::Dispatch(3)),
    (NodeKey::Mapping(1), NodeKey::Dispatch(1)),
    (NodeKey::Mapping(1), NodeKey::Dispatch(4)),
    (NodeKey::Mapping(2), NodeKey::Dispatch(5)),
    (NodeKey::Mapping(2), NodeKey::Dispatch(6)),
    (NodeKey::Mapping(2), NodeKey::Dispatch(7)),
    (NodeKey::Mapping(3), NodeKey::Dispatch(8)),
];

const MARGIN: f64 = 110.0;
const COL_GAP: f64 = 130.0;
const ROW_Y: [f64; 3] = [100.0, 400.0, 700.0];
/// Each layer roams freely inside its own horizontal band (±120 around row).
const BAND_HALF: f64 = 120.0;
fn band_y(layer: u8) -> (f64, f64) {
    (
        ROW_Y[layer as usize] - BAND_HALF,
        ROW_Y[layer as usize] + BAND_HALF,
    )
}
const VIEW_W: f64 = MARGIN * 2.0 + (DISPATCH.len() as f64 - 1.0) * COL_GAP;
const VIEW_H: f64 = 840.0;
const NODE_W: f64 = 104.0;
const NODE_H: f64 = 36.0;

/// Deterministic startup layout, computed from the graph — no physics involved:
/// groups spread evenly; each mapping sits at the average x of the groups it
/// links to; each channel/model sits at the average x of its linked mappings.
/// Same-type spacing enforced with a left-to-right sweep. Physics then only
/// handles collisions and user drags, so the graph no longer reels inward
/// on open (the old grid was wider than the rope slack radius, so every link
/// started taut and pulled everything toward the center).
fn initial_positions() -> HashMap<NodeKey, (f64, f64)> {
    let layers = visible_layers();
    let mut out: HashMap<NodeKey, (f64, f64)> = HashMap::new();
    // row 0: even spread around the canvas center
    let n = layers[0].len();
    for (i, &k) in layers[0].iter().enumerate() {
        out.insert(
            k,
            (
                VIEW_W / 2.0 + (i as f64 - (n as f64 - 1.0) / 2.0) * COL_GAP,
                ROW_Y[0],
            ),
        );
    }
    // rows 1..2: barycenter of upper-layer neighbors (raw edges, folded onto
    // channel cards when the channel is collapsed), fallback to even spread
    for l in 1..3 {
        let mut placed: Vec<(NodeKey, f64)> = Vec::new();
        for &k in &layers[l] {
            let xs: Vec<f64> = SEED_EDGES
                .iter()
                .filter_map(|&(u, lo)| {
                    let folded = lo;
                    if folded == k {
                        out.get(&u).map(|p| p.0)
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
            placed.push((k, x));
        }
        placed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let mut prev = f64::NEG_INFINITY;
        for (k, x) in placed {
            let x = x.max(prev + COL_GAP);
            prev = x;
            out.insert(k, (x, ROW_Y[l]));
        }
    }
    out
}

/// Startup relaxation: pull every node toward the average x of its wired
/// neighbors, then sweep each row left→right enforcing min gap. ~200 passes or
/// until stable. Runs once before first paint; live physics takes over after.
fn settle_layout(
    layers: &[Vec<NodeKey>; 3],
    edges: &[(NodeKey, NodeKey)],
    positions: &mut HashMap<NodeKey, (f64, f64)>,
) {
    const ROW_MIN_GAP: f64 = NODE_W + 16.0;
    for _ in 0..200 {
        let mut max_d = 0.0f64;
        for row in layers.iter() {
            for &k in row {
                let (mut sum, mut n) = (0.0f64, 0.0f64);
                for &(up, low) in edges {
                    let other = if up == k {
                        Some(low)
                    } else if low == k {
                        Some(up)
                    } else {
                        None
                    };
                    if let Some(o) = other {
                        sum += positions.get(&o).map(|p| p.0).unwrap_or(0.0);
                        n += 1.0;
                    }
                }
                if n > 0.0 {
                    let p = positions.get_mut(&k).unwrap();
                    let d = (sum / n - p.0) * 0.2;
                    p.0 += d;
                    max_d = max_d.max(d.abs());
                }
            }
        }
        for row in layers.iter() {
            let mut order = row.clone();
            order.sort_by(|a, b| positions[a].0.partial_cmp(&positions[b].0).unwrap());
            // Sweep is right-only; pin the row's mean so it can't drift off-canvas.
            let mean0 = order.iter().map(|k| positions[k].0).sum::<f64>() / order.len() as f64;
            let mut prev = f64::NEG_INFINITY;
            for k in order.iter().copied() {
                let p = positions.get_mut(&k).unwrap();
                let need = prev + ROW_MIN_GAP - p.0;
                if need > 0.0 {
                    p.0 += need;
                    max_d = max_d.max(need);
                }
                prev = p.0;
            }
            let shift =
                order.iter().map(|k| positions[k].0).sum::<f64>() / order.len() as f64 - mean0;
            for &k in &order {
                positions.get_mut(&k).unwrap().0 -= shift;
                max_d = max_d.max(shift.abs());
            }
        }
        // Anchor the whole graph to the canvas centre each pass, otherwise the
        // sweep's right-bias makes everything random-walk rightward.
        let all: Vec<NodeKey> = layers.iter().flatten().copied().collect();
        let mean = all.iter().map(|k| positions[k].0).sum::<f64>() / all.len() as f64;
        let global_shift = mean - VIEW_W / 2.0;
        for &k in &all {
            positions.get_mut(&k).unwrap().0 -= global_shift;
        }
        if max_d < 0.2 {
            break;
        }
    }
}

/// 不像 settle_layout 会把整图拉回 VIEW_W/2，它只把每层自己的 barycenter 对齐，
/// 原封不动保留左缘锚定，避免焦点态子图又被甩回中央。
fn settle_layout_forward(
    layers: &[Vec<NodeKey>; 3],
    edges: &[(NodeKey, NodeKey)],
    positions: &mut HashMap<NodeKey, (f64, f64)>,
) {
    for _ in 0..60 {
        let mut max_d = 0.0f64;
        for row in layers.iter() {
            for &k in row {
                let (mut sum, mut n) = (0.0f64, 0.0f64);
                for &(up, low) in edges {
                    let other = if up == k {
                        Some(low)
                    } else if low == k {
                        Some(up)
                    } else {
                        None
                    };
                    if let Some(o) = other {
                        if let Some(p) = positions.get(&o) {
                            sum += p.0;
                            n += 1.0;
                        }
                    }
                }
                if n > 0.0 {
                    if let Some(p) = positions.get_mut(&k) {
                        let d = (sum / n - p.0) * 0.25;
                        p.0 += d;
                        max_d = max_d.max(d.abs());
                    }
                }
            }
        }
        if max_d < 0.15 {
            break;
        }
    }
}

/// Node→node edge must span exactly one layer; channels are aggregates only.
fn normalize(a: NodeKey, b: NodeKey) -> Option<(NodeKey, NodeKey)> {
    let la = a.layer() as i16;
    let lb = b.layer() as i16;
    if a == b || (la - lb).abs() != 1 {
        return None;
    }
    Some(if la < lb { (a, b) } else { (b, a) })
}

fn bezier(a: (f64, f64), b: (f64, f64)) -> String {
    let mid = (a.1 + b.1) / 2.0;
    path_str(a, (a.0, mid), (b.0, mid), b)
}

fn path_str(a: (f64, f64), c1: (f64, f64), c2: (f64, f64), b: (f64, f64)) -> String {
    format!(
        "M {:.0} {:.0} C {:.0} {:.0}, {:.0} {:.0}, {:.0} {:.0}",
        a.0, a.1, c1.0, c1.1, c2.0, c2.1, b.0, b.1
    )
}

fn cubic_at(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64), t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    (
        u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0,
        u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1,
    )
}

/// Fraction of lateral control-point offset that dodges blocker rects:
/// first collision-free candidate wins, else the least-colliding one.
fn dodge_frac(a: (f64, f64), b: (f64, f64), blockers: &[(f64, f64)]) -> f64 {
    const FRACS: [f64; 5] = [0.0, 0.12, -0.12, 0.26, -0.26];
    const SAMPLES: usize = 20;
    const PAD_X: f64 = NODE_W / 2.0 + 8.0;
    const PAD_Y: f64 = NODE_H / 2.0 + 8.0;

    let dist = (b.0 - a.0).hypot(b.1 - a.1);
    let mid = (a.1 + b.1) / 2.0;
    let mut best: Option<(usize, f64)> = None;
    for &f in &FRACS {
        let off = f * dist;
        let (c1, c2) = ((a.0 + off, mid), (b.0 + off, mid));
        let mut hits = 0usize;
        for i in 0..SAMPLES {
            let t = i as f64 / (SAMPLES - 1) as f64;
            let p = cubic_at(a, c1, c2, b, t);
            if blockers
                .iter()
                .any(|&(bx, by)| (p.0 - bx).abs() < PAD_X && (p.1 - by).abs() < PAD_Y)
            {
                hits += 1;
            }
        }
        if hits == 0 {
            return f;
        }
        if best.is_none() || hits < best.unwrap().0 {
            best = Some((hits, f));
        }
    }
    best.unwrap_or((0, 0.0)).1
}

/// Bezier path with a lateral control-point dodge of `frac` × edge length.
fn offset_bezier(a: (f64, f64), b: (f64, f64), frac: f64) -> String {
    if frac == 0.0 {
        return bezier(a, b);
    }
    let dist = (b.0 - a.0).hypot(b.1 - a.1);
    let mid = (a.1 + b.1) / 2.0;
    path_str(a, (a.0 + frac * dist, mid), (b.0 + frac * dist, mid), b)
}

/// Pan/zoom so all points sit inside the viewBox with padding. Returns (pan, zoom).
/// 抽屉宽度（客户端 px），与 NodeInspector 的 `sm:w-[320px]` 一致。
/// 焦点子图要排到抽屉左侧，必须知道被遮掉多少。
const DRAWER_W: f64 = 320.0;

/// 可视画布区（viewBox 坐标）。抽屉覆盖的右侧不算可视区，
/// 于是焦点子图才会完整落在左边而不是被压在抽屉底下。
fn visible_box(rect: Option<(f64, f64, f64, f64)>, drawer: bool) -> (f64, f64, f64, f64) {
    let Some((_, _, rw, rh)) = rect else {
        return (0.0, 0.0, VIEW_W, VIEW_H);
    };
    let s = (rw / VIEW_W).min(rh / VIEW_H);
    if s <= 0.0 {
        return (0.0, 0.0, VIEW_W, VIEW_H);
    }
    let ox = (rw - VIEW_W * s) / 2.0;
    // 窄屏抽屉是全宽/底部抽屉，不预留横向空间，否则可视区会退化成 0。
    let avail = if drawer && rw >= 640.0 {
        (rw - DRAWER_W).max(240.0)
    } else {
        rw
    };
    let x0 = ((0.0 - ox) / s).max(0.0);
    let x1 = ((avail - ox) / s).min(VIEW_W);
    (x0, 0.0, (x1 - x0).max(120.0), VIEW_H)
}

/// 把点集缩放平移进指定框（viewBox 坐标）。
fn fit_view_into(pts: &[(f64, f64)], (bx, by, bw, bh): (f64, f64, f64, f64)) -> ((f64, f64), f64) {
    let pad = NODE_W / 2.0 + 24.0;
    let (minx, maxx) = pts
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &(x, _)| {
            (a.min(x), b.max(x))
        });
    let (miny, maxy) = pts
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &(_, y)| {
            (a.min(y), b.max(y))
        });
    let (w, h) = ((maxx - minx).max(1.0), (maxy - miny).max(1.0));
    let z = (((bw - 2.0 * pad) / w).min((bh - 2.0 * pad) / h)).clamp(0.35, 1.6);
    let cy = (miny + maxy) / 2.0;
    // x 左对齐：子图贴可视区左缘（即层标签旁边），不要居中把左侧空出一大块。
    ((bx + pad - minx * z, by + bh / 2.0 - cy * z), z)
}

/// 连通锥：从起点向外扩散，每步必须**远离**起点所在层。
/// 这样点别名只拉出「它的分组 + 它的调度模型」，
/// 而不会经由分组把整张图都拽进来。
fn focus_cone(start: NodeKey, edges: &[(NodeKey, NodeKey)]) -> HashSet<NodeKey> {
    let start_layer = start.layer() as i16;
    let dist = |k: NodeKey| (k.layer() as i16 - start_layer).abs();
    let mut seen: HashSet<NodeKey> = HashSet::from([start]);
    let mut queue: VecDeque<NodeKey> = VecDeque::from([start]);
    while let Some(n) = queue.pop_front() {
        for &(up, low) in edges {
            let next = if up == n && dist(low) > dist(n) {
                Some(low)
            } else if low == n && dist(up) > dist(n) {
                Some(up)
            } else {
                None
            };
            if let Some(m) = next {
                if seen.insert(m) {
                    queue.push_back(m);
                }
            }
        }
    }
    seen
}

/// 焦点子图的一次性排布：每层按当前 x 顺序均匀铺开（保持相对次序，
/// 视觉跳动最小），再松弛让上层压在下层重心上。
/// 只在进入焦点时算一次；之后位置交给物理与拖拽。
fn layout_subgraph(
    sub: &HashSet<NodeKey>,
    edges: &[(NodeKey, NodeKey)],
    cur: &HashMap<NodeKey, (f64, f64)>,
) -> HashMap<NodeKey, (f64, f64)> {
    let mut rows: [Vec<NodeKey>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for &k in sub {
        rows[k.layer() as usize].push(k);
    }
    let mut out: HashMap<NodeKey, (f64, f64)> = HashMap::new();
    for (l, row) in rows.iter_mut().enumerate() {
        row.sort_by(|a, b| {
            let xa = cur.get(a).map(|p| p.0).unwrap_or(0.0);
            let xb = cur.get(b).map(|p| p.0).unwrap_or(0.0);
            xa.partial_cmp(&xb).unwrap_or(std::cmp::Ordering::Equal)
        });
        // 每层都**左对齐**到可视区左缘，贴着层标题；垂直的 z 收缩只剩在 y，
        // 避免「图中央却空一块、又离标签老远」。
        let left = 24.0;
        for (i, &k) in row.iter().enumerate() {
            out.insert(k, (left + i as f64 * COL_GAP, ROW_Y[l]));
        }
    }
    let layers = [rows[0].clone(), rows[1].clone(), rows[2].clone()];
    let sub_edges: Vec<(NodeKey, NodeKey)> = edges
        .iter()
        .copied()
        .filter(|(u, l)| sub.contains(u) && sub.contains(l))
        .collect();
    settle_layout_forward(&layers, &sub_edges, &mut out);
    out
}

/// 位置/视图补间。进出焦点时平滑移动，节点不瞬移。
#[derive(Clone)]
struct Tween {
    from: HashMap<NodeKey, (f64, f64)>,
    to: HashMap<NodeKey, (f64, f64)>,
    from_pan: (f64, f64),
    to_pan: (f64, f64),
    from_zoom: f64,
    to_zoom: f64,
    /// 每个节点的错峰延迟，让队列次第滑入（而非整段卡死）。
    delay: HashMap<NodeKey, f64>,
    /// 0 → 1
    t: f64,
}

/// 两次进入更顺滑：前期快启动，末段缓刹。
fn ease_out_quint(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(5)
}

/// 给锥内节点算错峰延迟：从起点向外传播，越远的越晚进。
fn stagger_delay(start: NodeKey, edges: &[(NodeKey, NodeKey)]) -> HashMap<NodeKey, f64> {
    let mut delay: HashMap<NodeKey, f64> = HashMap::from([(start, 0.0)]);
    let mut queue: VecDeque<NodeKey> = VecDeque::from([start]);
    while let Some(n) = queue.pop_front() {
        let d0 = delay[&n];
        for &(u, l) in edges {
            let m = if u == n {
                l
            } else if l == n {
                u
            } else {
                continue;
            };
            if !delay.contains_key(&m) {
                delay.insert(m, d0 + 0.10);
                queue.push_back(m);
            }
        }
    }
    // 退出时反着来：先动的先回
    delay
}

/// Tween 统一构造，省得三个位置重复写字段。
fn make_tween(
    from: HashMap<NodeKey, (f64, f64)>,
    to: HashMap<NodeKey, (f64, f64)>,
    from_pan: (f64, f64),
    to_pan: (f64, f64),
    from_zoom: f64,
    to_zoom: f64,
    delay: HashMap<NodeKey, f64>,
) -> Tween {
    Tween {
        from,
        to,
        from_pan,
        to_pan,
        from_zoom,
        to_zoom,
        delay,
        t: 0.0,
    }
}

fn fit_view(pts: &[(f64, f64)]) -> ((f64, f64), f64) {
    let pad = NODE_W / 2.0 + 24.0;
    let (minx, maxx) = pts
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &(x, _)| {
            (a.min(x), b.max(x))
        });
    let (miny, maxy) = pts
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &(_, y)| {
            (a.min(y), b.max(y))
        });
    let z = (((VIEW_W - 2.0 * pad) / (maxx - minx)).min((VIEW_H - 2.0 * pad) / (maxy - miny)))
        .clamp(0.35, 3.0);
    let cx = (minx + maxx) / 2.0;
    let cy = (miny + maxy) / 2.0;
    ((VIEW_W / 2.0 - cx * z, VIEW_H / 2.0 - cy * z), z)
}

#[derive(Clone, Copy)]
enum Drag {
    /// Node body drag: reposition. sx/sy = start client coords.
    Move {
        key: NodeKey,
        sx: f64,
        sy: f64,
        ox: f64,
        oy: f64,
        moved: bool,
    },
    /// Port drag: pull a wire.
    Wire { src: NodeKey },
    /// Background drag: pan. px/py = pan at drag start.
    Pan {
        sx: f64,
        sy: f64,
        px: f64,
        py: f64,
        moved: bool,
    },
    /// Background drag: marquee-select; both rect corners live in `marquee`.
    Select,
}

#[component]
pub fn NetworkPanel() -> Element {
    let mut edges = use_signal(|| {
        SEED_EDGES
            .iter()
            .copied()
            .collect::<HashSet<(NodeKey, NodeKey)>>()
    });
    let mut drag = use_signal(|| None::<Drag>);
    let mut hover_wire = use_signal(|| None::<(NodeKey, NodeKey)>);
    let mut hover = use_signal(|| None::<NodeKey>);
    let mut cursor_world = use_signal(|| (0.0f64, 0.0f64));
    let mut rect = use_signal(|| None::<(f64, f64, f64, f64)>);
    let mut pan = use_signal(|| (0.0f64, 0.0f64));
    let mut zoom = use_signal(|| 1.0f64);
    let mut selected = use_signal(HashSet::<NodeKey>::new);
    // 侧边抽屉当前检视的节点；None 表示抽屉关闭。
    let mut inspect = use_signal(|| None::<NodeKey>);
    // 焦点空间：进入后只保留与起点连通的锥体，其余节点移出画布。
    // Some(集合) 表示处于焦点态；集合在焦点期间**不变**（点左侧其他
    // 节点只切抽屉，不重算子图），符合「不会变树」的要求。
    let mut focus_space = use_signal(|| None::<HashSet<NodeKey>>);
    // 进出焦点的补间动画；Some 时 ticker 每帧推进。
    let mut tween = use_signal(|| None::<Tween>);
    // 焦点态下每个节点的目标位；退出时用来还原。
    let mut saved_positions =
        use_signal(|| None::<(HashMap<NodeKey, (f64, f64)>, (f64, f64), f64)>);
    // 面包屑：焦点空间内看过哪些节点（含起点），点击可回走。
    let mut crumbs = use_signal(Vec::<NodeKey>::new);
    // Group-move anchors: (node, world offset from cursor) for the whole selection.
    let mut moving = use_signal(Vec::<(NodeKey, f64, f64)>::new);
    // Marquee rect in viewBox coords while a Select drag is active.
    let mut marquee = use_signal(|| None::<((f64, f64), (f64, f64))>);
    // Per-edge dodge curve factor, eased by the ticker (0 = straight).
    let mut dodge = use_signal(HashMap::<(NodeKey, NodeKey), f64>::new);
    let mut positions = use_signal(|| HashMap::<NodeKey, (f64, f64)>::new());
    // Live spring layout: a 16ms ticker integrates forces frame by frame.
    // Structure changes (wires / collapse) bump `wake`; the loop steps while
    // it's woken, while a node is being dragged, or while energy remains,
    // then sleeps. Dragged nodes are held; live neighbors dodge in real time.
    let mut wake = use_signal(|| 0u32);
    use_effect(move || {
        let _ = (edges(), drag());
        let next = wake.peek().wrapping_add(1);
        wake.set(next);
    });
    use_hook(move || {
        // Deterministic startup layout before the ticker takes over.
        {
            let mut p = initial_positions();
            let layers = visible_layers();
            let pairs = display_edge_pairs(&edges.peek());
            let pairs_xy: Vec<(NodeKey, NodeKey)> = pairs.iter().map(|&(u, l, _)| (u, l)).collect();
            settle_layout(&layers, &pairs_xy, &mut p);
            *positions.write() = p;
        }
        spawn(async move {
            let mut velocities = HashMap::<NodeKey, (f64, f64)>::new();
            let mut seen_wake = *wake.peek();
            let mut energy = f64::MAX;
            // Dodge easing keeps the ticker alive until curves converge.
            let mut dodge_active = false;
            loop {
                gloo_timers::future::TimeoutFuture::new(16).await;
                let held = match *drag.peek() {
                    Some(Drag::Move { key, .. }) => Some(key),
                    _ => None,
                };
                let dirty = seen_wake != *wake.peek();
                seen_wake = *wake.peek();

                // 补间优先：动画期间接管位置与视图，跳过物理，
                // 否则弹簧会把节点从目标位拽回去。
                if tween.peek().is_some() {
                    let done = {
                        let mut tw = tween.write();
                        let Some(t) = tw.as_mut() else { continue };
                        // 总时长约 350ms，错峰只延迟各节点的开始，不拉长全程
                        t.t = (t.t + 0.045).min(1.0);
                        {
                            let mut pw = positions.write();
                            for (k, to) in t.to.iter() {
                                let from = t.from.get(k).copied().unwrap_or(*to);
                                // 错峰：这个节点先静下来再动，序列感
                                let d = t.delay.get(k).copied().unwrap_or(0.0);
                                let local = ((t.t - d) / (1.0 - d)).clamp(0.0, 1.0);
                                let e = ease_out_quint(local);
                                pw.insert(
                                    *k,
                                    (from.0 + (to.0 - from.0) * e, from.1 + (to.1 - from.1) * e),
                                );
                            }
                        }
                        // 视图不做错峰，主视角平滑即可
                        let e = ease_out_quint(t.t);
                        pan.set((
                            t.from_pan.0 + (t.to_pan.0 - t.from_pan.0) * e,
                            t.from_pan.1 + (t.to_pan.1 - t.from_pan.1) * e,
                        ));
                        zoom.set(t.from_zoom + (t.to_zoom - t.from_zoom) * e);
                        t.t >= 1.0
                    };
                    if done {
                        tween.set(None);
                        velocities.clear();
                        energy = f64::MAX;
                    }
                    continue;
                }

                if !(dirty || held.is_some() || energy > 0.08 || dodge_active) {
                    continue;
                }
                let mut layers = visible_layers();
                let pairs = display_edge_pairs(&edges.peek());
                // 焦点态：物理只作用在锥内节点上，锥外节点不受力不动位，
                // 避免 vip 这类多连分组被锥外别名拽走。
                if let Some(cone) = focus_space.peek().as_ref() {
                    for row in layers.iter_mut() {
                        row.retain(|k| cone.contains(k));
                    }
                }
                let pairs_xy: Vec<(NodeKey, NodeKey)> = {
                    let cone = focus_space.peek();
                    pairs
                        .iter()
                        .filter(|&(u, l, _)| {
                            cone.as_ref()
                                .map_or(true, |c| c.contains(u) && c.contains(l))
                        })
                        .map(|&(u, l, _)| (u, l))
                        .collect()
                };
                energy = physics_step(
                    &layers,
                    &pairs_xy,
                    held,
                    &mut positions.write(),
                    &mut velocities,
                );
                // Wire dodge eases toward its target each frame: 0 while any
                // drag is live (wires follow rigidly, no morphing) and the
                // best dodge fraction after release, so avoidance grows in
                // smoothly instead of snapping mid-drag.
                let dragging = matches!(*drag.peek(), Some(Drag::Move { .. } | Drag::Wire { .. }));
                {
                    let ps = positions.peek();
                    let nodes: Vec<NodeKey> = layers.iter().flatten().copied().collect();
                    let mut dw = dodge.write();
                    dodge_active = false;
                    for &(u, l, raw) in &pairs {
                        let (pu, pl) = (ps[&u], ps[&l]);
                        let (a, b) = ((pu.0, pu.1 + NODE_H / 2.0), (pl.0, pl.1 - NODE_H / 2.0));
                        let target = if dragging {
                            0.0
                        } else {
                            // Only nodes whose x falls inside the edge's span
                            // (plus node padding) can be hit — cheap prefilter.
                            let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
                            let (pad_x, pad_y) = (NODE_W / 2.0 + 8.0, NODE_H / 2.0 + 8.0);
                            let blockers: Vec<(f64, f64)> = nodes
                                .iter()
                                .filter(|&&k| k != u && k != l)
                                .map(|&k| ps[&k])
                                .filter(|&(bx, by)| {
                                    bx >= x0 - pad_x
                                        && bx <= x1 + pad_x
                                        && by >= (a.1.min(b.1)) - pad_y
                                        && by <= (a.1.max(b.1)) + pad_y
                                })
                                .collect();
                            if blockers.is_empty() {
                                0.0
                            } else {
                                dodge_frac(a, b, &blockers)
                            }
                        };
                        let cur = dw.entry(raw).or_insert(0.0);
                        let next = *cur + (target - *cur) * 0.18;
                        let next = if (next - target).abs() < 0.003 {
                            target
                        } else {
                            next
                        };
                        if (*cur - next).abs() > 0.0005 {
                            dodge_active = true;
                            *cur = next;
                        }
                    }
                }
            }
        });
    });

    // ---- Visible nodes per layer (collapse-aware) ----
    let cone_now = focus_space();
    // 焦点态：无关节点直接不渲染（不是变暗），物理仍照全图跑，
    // 退出时才需要它们的位置。
    let mut layers = visible_layers();
    if let Some(cone) = &cone_now {
        for row in layers.iter_mut() {
            row.retain(|k| cone.contains(k));
        }
    }
    let selection_now = selected();
    let layers_fit = layers.clone(); // owned copy for the 适配 button's handler

    let positions_now = positions();
    let pos = |key: NodeKey| -> (f64, f64) {
        positions_now
            .get(&key)
            .copied()
            .unwrap_or((VIEW_W / 2.0, ROW_Y[key.layer() as usize]))
    };

    let display_edges: Vec<(NodeKey, NodeKey, (NodeKey, NodeKey))> = {
        let all = display_edge_pairs(&edges());
        match &cone_now {
            Some(cone) => all
                .into_iter()
                .filter(|(u, l, _)| cone.contains(u) && cone.contains(l))
                .collect(),
            None => all,
        }
    };
    let dodge_now = dodge();

    // ---- Focus: union of layer-distance BFS from every selected node ----
    // A step is admitted iff it strictly increases |layer − start.layer|.
    // Multi-selection dims anything not connected to ANY selected node;
    // a single selection behaves like the old click-to-focus.
    let focus: Option<HashSet<NodeKey>> = if cone_now.is_some() {
        // 焦点空间里所有可见节点都相关，不需要再调光
        None
    } else if selection_now.is_empty() {
        None
    } else {
        let mut out = HashSet::new();
        for start in &selection_now {
            let start_layer = start.layer() as i16;
            let dist = |k: NodeKey| (k.layer() as i16 - start_layer).abs();
            let mut seen: HashSet<NodeKey> = HashSet::from([*start]);
            let mut queue: VecDeque<NodeKey> = VecDeque::from([*start]);
            while let Some(n) = queue.pop_front() {
                for &(up, low, _) in &display_edges {
                    let next = if up == n && dist(low) > dist(n) {
                        Some(low)
                    } else if low == n && dist(up) > dist(n) {
                        Some(up)
                    } else {
                        None
                    };
                    if let Some(m) = next {
                        if seen.insert(m) {
                            queue.push_back(m);
                        }
                    }
                }
            }
            out.extend(seen);
        }
        Some(out)
    };

    // ---- Coordinate helpers ----
    // preserveAspectRatio="xMidYMid meet": uniform scale + centered letterbox.
    // All client→viewBox mapping MUST go through this or it drifts off-cursor.
    let client_to_view = move |cx: f64, cy: f64| -> (f64, f64) {
        let Some((rx, ry, rw, rh)) = *rect.peek() else {
            return (cx, cy);
        };
        let s = (rw / VIEW_W).min(rh / VIEW_H);
        (
            (cx - rx - (rw - VIEW_W * s) / 2.0) / s,
            (cy - ry - (rh - VIEW_H * s) / 2.0) / s,
        )
    };

    let to_world = move |client_x: f64, client_y: f64| -> (f64, f64) {
        let view = client_to_view(client_x, client_y);
        let (px, py) = *pan.peek();
        let z = *zoom.peek();
        ((view.0 - px) / z, (view.1 - py) / z)
    };

    let anchor = |key: NodeKey, as_upper: bool| -> (f64, f64) {
        let (x, y) = pos(key);
        (
            x,
            if as_upper {
                y + NODE_H / 2.0
            } else {
                y - NODE_H / 2.0
            },
        )
    };

    // Dangling wire endpoint: snap to legal hovered node, else cursor.
    let drag_now = drag();
    let hover_now = hover();
    let temp_end: (f64, f64) = (|| {
        if let (Some(Drag::Wire { src }), Some(t)) = (drag_now, hover_now) {
            if let Some(pair) = normalize(src, t) {
                if !edges_read(&edges).contains(&pair) {
                    return anchor(t, t.layer() == 0 || pair.0 == t);
                }
            }
        }
        cursor_world()
    })();

    // Commit the active marquee into `selected`. Releasing over a node
    // stop_propagates to this node's mouseup instead of the canvas', so both
    // handlers route through here — otherwise the marquee hangs mid-drag.
    let mut commit_select = move || {
        match *marquee.peek() {
            // Tiny rect → plain click on empty canvas: clear.
            Some((a, b)) if (a.0 - b.0).abs() + (a.1 - b.1).abs() > 4.0 => {
                // Marquee corners are viewBox coords → world directly.
                let (px, py) = *pan.peek();
                let z = *zoom.peek();
                let wa = ((a.0 - px) / z, (a.1 - py) / z);
                let wb = ((b.0 - px) / z, (b.1 - py) / z);
                let (wx0, wx1) = (wa.0.min(wb.0), wa.0.max(wb.0));
                let (wy0, wy1) = (wa.1.min(wb.1), wa.1.max(wb.1));
                let layers = visible_layers();
                let hit: HashSet<NodeKey> = layers
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|&k| {
                        let Some((x, y)) = positions.peek().get(&k).copied() else {
                            return false;
                        };
                        x >= wx0 && x <= wx1 && y >= wy0 && y <= wy1
                    })
                    .collect();
                selected.set(hit);
            }
            _ => selected.set(HashSet::new()),
        }
        marquee.set(None);
    };

    let port_of = |key: NodeKey| -> &'static str {
        match key {
            NodeKey::Group(_) => "bottom",
            NodeKey::Mapping(_) => "both",
            NodeKey::Dispatch(_) => "top",
        }
    };

    let hint = match drag_now {
        _ if cone_now.is_some() => {
            "焦点空间：只显示相关节点 · 点左侧节点切换编辑 · 点空白或再点同节点退出"
        }
        Some(Drag::Wire { .. }) => "拖到相邻层节点松开连线",
        Some(Drag::Move { .. }) => "松开落位",
        _ => "滚轮缩放 · 拖空白平移 · Shift拖空白框选 · Ctrl点选多个 · 拖节点摆位 · 拖圆点连线",
    };

    rsx! {
        div { class: "flex h-full min-h-[480px] flex-col",
            div { class: "flex flex-wrap items-center gap-3 px-1 pb-3",
                for g in GROUPS {
                    span { class: "inline-flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900 px-2.5 py-1 text-xs text-zinc-400",
                        span { class: "h-2.5 w-2.5 rounded-full", style: "background: {g.color}" }
                        "{g.name}"
                    }
                }
                button {
                    class: "ml-auto rounded-md border border-zinc-800 bg-zinc-900 px-2.5 py-1 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
                    onclick: move |_| {
                        // Zoom/pan so every visible node fits inside the viewBox.
                        let pts: Vec<(f64, f64)> = layers_fit
                            .iter()
                            .flatten()
                            .filter_map(|k| positions.peek().get(k).copied())
                            .collect();
                        if pts.is_empty() {
                            return;
                        }
                        let ((px, py), z) = fit_view(&pts);
                        pan.set((px, py));
                        zoom.set(z);
                    },
                    "适配"
                }
                // w-full pins the hint to its own line: the legend row and the
                // canvas below must never shift when the hint text changes.
                span { class: "w-full text-right text-xs text-zinc-600 leading-4", "{hint}" }
            }
            // 画布与抽屉同层：抽屉 absolute 覆盖右侧，画布尺寸恒定，
            // 开合不引起任何重排。
            div { class: "relative min-h-0 flex-1",
                div { class: "h-full overflow-hidden rounded-xl border border-zinc-800 bg-zinc-950",
                svg {
                    view_box: "0 0 {VIEW_W:.0} {VIEW_H:.0}",
                    width: "100%",
                    height: "100%",
                    preserve_aspect_ratio: "xMidYMid meet",
                    style: match drag_now {
                        Some(Drag::Wire { .. }) => "cursor: crosshair",
                        Some(Drag::Pan { .. }) => "cursor: grabbing",
                        _ => "cursor: default",
                    },
                    onmounted: move |e| async move {
                        if let Ok(r) = e.get_client_rect().await {
                            rect.set(Some((r.origin.x, r.origin.y, r.size.width, r.size.height)));
                        }
                    },
                    // Background press → pan, or marquee with Shift held
                    // (nodes/ports/wires stop propagation)
                    onmousedown: move |e| {
                        let c = e.client_coordinates();
                        if e.modifiers().shift() {
                            // Marquee anchor = press point in viewBox coords.
                            if rect.peek().is_some() {
                                let v = client_to_view(c.x, c.y);
                                marquee.set(Some((v, v)));
                            }
                            drag.set(Some(Drag::Select));
                        } else {
                            let (px, py) = pan();
                            drag.set(Some(Drag::Pan { sx: c.x, sy: c.y, px, py, moved: false }));
                        }
                    },
                    onmousemove: move |e| {
                        let c = e.client_coordinates();
                        let world = to_world(c.x, c.y);
                        let current = *drag.peek();
                        match current {
                            Some(Drag::Pan { sx, sy, px, py, .. }) => {
                                let moved_flag = (c.x - sx).abs() + (c.y - sy).abs() > 4.0;
                                if let Some((_, _, rw, rh)) = *rect.peek() {
                                    let s = (rw / VIEW_W).min(rh / VIEW_H);
                                    pan.set((px + (c.x - sx) / s, py + (c.y - sy) / s));
                                }
                                drag.set(Some(Drag::Pan { sx, sy, px, py, moved: moved_flag }));
                            }
                            Some(Drag::Move { key, sx, sy, ox, oy, .. }) => {
                                let moved_flag = (c.x - sx).abs() + (c.y - sy).abs() > 4.0;
                                {
                                    let mut pos_w = positions.write();
                                    let (kb0, kb1) = band_y(key.layer());
                                    // Leader always follows the cursor, even when flying solo.
                                    pos_w.insert(key, (world.0 + ox, (world.1 + oy).clamp(kb0, kb1)));
                                    // Group members keep their offsets when part of a selection.
                                    for &(k, kox, koy) in moving.peek().iter() {
                                        if k == key {
                                            continue;
                                        }
                                        let (b0, b1) = band_y(k.layer());
                                        pos_w.insert(k, (world.0 + kox, (world.1 + koy).clamp(b0, b1)));
                                    }
                                }
                                drag.set(Some(Drag::Move { key, sx, sy, ox, oy, moved: moved_flag }));
                            }
                            Some(Drag::Wire { .. }) => cursor_world.set(world),
                            Some(Drag::Select) => {
                                {
                                    let v = client_to_view(c.x, c.y);
                                    let cur = *marquee.read();
                                    if let Some((a, _)) = cur {
                                        marquee.set(Some((a, v)));
                                    }
                                }
                            }
                            None => {}
                        }
                    },
                    onmouseup: move |_| {
                        let current = *drag.peek();
                        match current {
                            Some(Drag::Pan { moved: false, .. }) => {
                                selected.set(HashSet::new());
                                inspect.set(None);
                                if let Some((saved, saved_pan, saved_zoom)) = saved_positions.take() {
                                    let from = positions.peek().clone();
                                    let (to_pan, to_zoom) = (saved_pan, saved_zoom);
                                    let mut delay = HashMap::new();
                                    for (i, k) in saved.keys().enumerate() {
                                        delay.insert(*k, i as f64 * 0.06);
                                    }
                                    tween.set(Some(make_tween(
                                        from,
                                        saved,
                                        *pan.peek(),
                                        to_pan,
                                        *zoom.peek(),
                                        to_zoom,
                                        delay,
                                    )));
                                }
                                focus_space.set(None);
                                crumbs.set(Vec::new());
                                crumbs.set(Vec::new());
                            }
                            Some(Drag::Select) => commit_select(),
                            _ => {}
                        }
                        drag.set(None);
                        hover.set(None);
                    },
                    onmouseleave: move |_| {
                        drag.set(None);
                        hover.set(None);
                        marquee.set(None);
                        moving.set(Vec::new());
                    },
                    onwheel: move |e| {
                        e.prevent_default();
                        let dy = e.delta().strip_units().y;
                        let factor = if dy < 0.0 { 1.12 } else { 0.9 };
                        let z0 = zoom();
                        let z = (z0 * factor).clamp(0.35, 3.0);
                        // Keep the world point under the cursor stationary.
                        let c = e.client_coordinates();
                        if rect.peek().is_some() {
                            let view = client_to_view(c.x, c.y);
                            let (px, py) = pan();
                            let world = ((view.0 - px) / z0, (view.1 - py) / z0);
                            pan.set((view.0 - world.0 * z, view.1 - world.1 * z));
                        }
                        zoom.set(z);
                    },
                    if let Some(((ax, ay), (bx, by))) = marquee() {
                        rect {
                            x: "{ax.min(bx):.0}",
                            y: "{ay.min(by):.0}",
                            width: "{(ax - bx).abs().max(1.0):.0}",
                            height: "{(ay - by).abs().max(1.0):.0}",
                            fill: "#fafafa",
                            fill_opacity: "0.05",
                            stroke: "#a1a1aa",
                            stroke_width: "1",
                            stroke_dasharray: "5 4",
                            pointer_events: "none",
                        }
                    }

                    // 层标题留在屏幕空间（在 pan 组之外），缩放平移时不动。
                    // 1240807 那次改动误删了这三个标签，此处恢复并改用新术语。
                    for (label, y) in [("分组", ROW_Y[0]), ("模型别名", ROW_Y[1]), ("调度模型", ROW_Y[2])] {
                        text {
                            x: "8",
                            y: "{y - 26.0:.0}",
                            fill: "#52525b",
                            font_size: "11",
                            pointer_events: "none",
                            "{label}"
                        }
                    }

                    g { transform: "translate({pan().0:.1} {pan().1:.1}) scale({zoom():.3})",

                        // World grid dots (RTS map feel)
                        defs {
                            pattern {
                                id: "grid-dots",
                                width: "40",
                                height: "40",
                                pattern_units: "userSpaceOnUse",
                                circle { cx: "20", cy: "20", r: "1.3", fill: "#27272a" }
                            }
                        }
                        rect {
                            x: "-3000",
                            y: "-3000",
                            width: "{VIEW_W + 6000.0:.0}",
                            height: "{VIEW_H + 6000.0:.0}",
                            fill: "url(#grid-dots)",
                            pointer_events: "none",
                        }
                        // ---- Committed wires ----
                        for (du, dl, raw) in display_edges.clone() {
                            {
                                let wire_color = color(du);
                                // Dodge factor is eased by the ticker; render only reads it.
                                let frac = dodge_now.get(&raw).copied().unwrap_or(0.0);
                                let d = offset_bezier(anchor(du, true), anchor(dl, false), frac);
                                let opacity = match &focus {
                                    Some(set) if set.contains(&du) && set.contains(&dl) => "0.9",
                                    Some(_) => "0.10",
                                    None => "0.75",
                                };
                                let upper = du.layer() == 0;
                                let hov_w = hover_wire() == Some(raw);
                                let (sw, sw_hov) = if upper { (4.0, 6.5) } else { (2.5, 4.5) };
                                let sw_now = if hov_w { sw_hov } else { sw };
                                let dur_now = if hov_w { "0.4s" } else { "0.9s" };
                                let flow_dur = if hov_w { "0.7s" } else { "1.6s" };
                                rsx! {
                                    path {
                                        d: "{d}",
                                        fill: "none",
                                        stroke: "{wire_color}",
                                        stroke_width: "{sw_now}",
                                        stroke_linecap: "round",
                                        opacity: if hov_w { "1" } else { opacity },
                                        stroke_dasharray: "10 7",
                                        animate {
                                            attribute_name: "stroke-dashoffset",
                                            from: "17",
                                            to: "0",
                                            dur: "{dur_now}",
                                            repeat_count: "indefinite",
                                        }
                                    }
                                    // Moving bright dashes on top of the solid wire (flow feel)
                                    // Moving tractor dots: dark halo under a white core so they read on
                                    // both bright (normal) and dimmed (focus) wires.
                                    path {
                                        d: "{d}",
                                        fill: "none",
                                        stroke: "#09090b",
                                        stroke_width: "3.4",
                                        stroke_linecap: "round",
                                        stroke_dasharray: "2.5 38",
                                        opacity: "0.8",
                                        pointer_events: "none",
                                        animate {
                                            attribute_name: "stroke-dashoffset",
                                            from: "40.5",
                                            to: "0",
                                            dur: "{flow_dur}",
                                            repeat_count: "indefinite",
                                        }
                                    }
                                    path {
                                        d: "{d}",
                                        fill: "none",
                                        stroke: "#fafafa",
                                        stroke_width: "1.5",
                                        stroke_linecap: "round",
                                        stroke_dasharray: "2.5 38",
                                        opacity: "0.95",
                                        pointer_events: "none",
                                        animate {
                                            attribute_name: "stroke-dashoffset",
                                            from: "40.5",
                                            to: "0",
                                            dur: "{flow_dur}",
                                            repeat_count: "indefinite",
                                        }
                                    }
                                    path {
                                        class: "cursor-pointer",
                                        d: "{d}",
                                        fill: "none",
                                        stroke: "rgba(255,255,255,0)",
                                        stroke_width: "16",
                                        onmousedown: move |e| e.stop_propagation(),
                                        onmouseenter: move |_| hover_wire.set(Some(raw)),
                                        onmouseleave: move |_| {
                                            if *hover_wire.peek() == Some(raw) { hover_wire.set(None) }
                                        },
                                        onclick: move |e| { e.stop_propagation(); edges.write().remove(&raw); },
                                        title { "点击删除连线" }
                                    }
                                }
                            }
                        }

                        // ---- Dangling wire ----
                        if let Some(Drag::Wire { src }) = drag_now {
                            path {
                                d: "{bezier(anchor(src, true), temp_end)}",
                                fill: "none",
                                stroke: "{color(src)}",
                                stroke_width: "3",
                                stroke_linecap: "round",
                                stroke_dasharray: "6 6",
                                opacity: "0.9",
                                pointer_events: "none",
                            }
                        }

                        // ---- Nodes ----
                        for layer in 0..3usize {
                            for key in layers[layer].clone() {
                                {
                                    let (x, y) = pos(key);
                                    let node_color = color(key);
                                    let title_text = node_title(key);
                                    let sub_text = subtitle(key);
                                    let ports = port_of(key);
                                    let node_opacity = match &focus {
                                        Some(set) if set.contains(&key) => "1",
                                        Some(_) => "0.18",
                                        None => "1",
                                    };
                                    let legal = matches!(drag_now, Some(Drag::Wire { src }) if {
                                        src != key
                                            && normalize(src, key).is_some()
                                            && !edges_read(&edges).contains(&normalize(src, key).unwrap())
                                    });
                                    let hov = hover_now == Some(key);
                                    let sel = selection_now.contains(&key);
                                    let title_y = if sub_text.is_empty() { y + 4.5 } else { y - 0.5 };
                                    // Per-type look: group = soft tinted pill, mapping = square chip,
                                    // channel/model = solid card. Distinct at a glance in any state.
                                    let (rx, fill, fill_op, sw, sw_hov, title_c): (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str) =
                                        match key.layer() {
                                            0 => ("18", node_color, "0.15", "2.5", "3.5", node_color),
                                            1 => ("6", node_color, "0.08", "1.75", "2.75", node_color),
                                            _ => ("12", "#1c1c21", "1", "1.5", "2.5", "#e4e4e7"),
                                        };
                                    rsx! {
                                        g {
                                            opacity: "{node_opacity}",
                                            onmouseenter: move |_| hover.set(Some(key)),
                                            onmouseleave: move |_| {
                                                if *hover.peek() == Some(key) { hover.set(None) }
                                            },
                                            onmousedown: move |e| {
                                                e.stop_propagation();
                                                let c = e.client_coordinates();
                                                let w = to_world(c.x, c.y);
                                                let (nx, ny) = (x, y);
                                                // Dragging one member moves the whole selection:
                                                // record each member's offset from the cursor once.
                                                {
                                                    let sel = selected.read();
                                                    if sel.len() > 1 && sel.contains(&key) {
                                                        let ps = positions.peek();
                                                        moving.set(
                                                            sel.iter().map(|&k| {
                                                                let p = ps[&k];
                                                                (k, p.0 - w.0, p.1 - w.1)
                                                            }).collect(),
                                                        );
                                                    } else {
                                                        moving.set(Vec::new());
                                                    }
                                                }
                                                drag.set(Some(Drag::Move { key, sx: c.x, sy: c.y, ox: nx - w.0, oy: ny - w.1, moved: false }));
                                            },
                                            onmouseup: move |e| {
                                                e.stop_propagation();
                                                let current = *drag.peek();
                                                match current {
                                                    // Releasing a marquee over a node must still commit
                                                    // (stop_propagation hides it from the canvas handler).
                                                    Some(Drag::Select) => commit_select(),
                                                    Some(Drag::Wire { src }) if src != key => {
                                                        // Multi-wire: every same-layer selected source
                                                        // with a legal edge to this target connects too.
                                                        let sources: Vec<NodeKey> = {
                                                            let sel = selected.read();
                                                            if sel.len() > 1 && sel.contains(&src) {
                                                                sel.iter()
                                                                    .copied()
                                                                    .filter(|&s| {
                                                                        s.layer() == src.layer()
                                                                            && normalize(s, key)
                                                                                .is_some_and(|pair| !edges_read(&edges).contains(&pair))
                                                                    })
                                                                    .collect()
                                                            } else {
                                                                vec![src]
                                                            }
                                                        };
                                                        let mut ew = edges.write();
                                                        for s in sources {
                                                            if let Some(pair) = normalize(s, key) {
                                                                ew.insert(pair);
                                                            }
                                                        }
                                                    }
                                                    Some(Drag::Move { key: k, moved: false, .. }) if k == key => {
                                                        if e.modifiers().ctrl() {
                                                            let mut sel = selected.write();
                                                            if !sel.remove(&key) {
                                                                sel.insert(key);
                                                            }
                                                        } else {
                                                            {
                                                                let mut sel = selected.write();
                                                                sel.clear();
                                                                sel.insert(key);
                                                            }
                                                            let same = *inspect.peek() == Some(key);
                                                            if same {
                                                                // 再点同一节点：退出焦点空间，还原全图
                                                                inspect.set(None);
                                                                if let Some((saved, saved_pan, saved_zoom)) = saved_positions.take() {
                                                                    let from = positions.peek().clone();
                                                                    let (to_pan, to_zoom) = (saved_pan, saved_zoom);
                                                                    let mut delay = HashMap::new();
                                                                    for (i, k) in saved.keys().enumerate() {
                                                                        delay.insert(*k, i as f64 * 0.06);
                                                                    }
                                                                    tween.set(Some(make_tween(
                                                                        from,
                                                                        saved,
                                                                        *pan.peek(),
                                                                        to_pan,
                                                                        *zoom.peek(),
                                                                        to_zoom,
                                                                        delay,
                                                                    )));
                                                                }
                                                                focus_space.set(None);
                                                            } else if focus_space.peek().is_some() {
                                                                // 焦点态内点其他节点：只换抽屉，树不变；
                                                                // 顺手记到面包屑里便于回走。
                                                                inspect.set(Some(key));
                                                                if crumbs.peek().last().copied() != Some(key) {
                                                                    crumbs.write().push(key);
                                                                }
                                                            } else {
                                                                // 进入焦点空间：算连通锥 → 一次性排布 → 补间
                                                                inspect.set(Some(key));
                                                                let all = edges_read(&edges);
                                                                let ev: Vec<(NodeKey, NodeKey)> =
                                                                    all.iter().copied().collect();
                                                                let cone = focus_cone(key, &ev);
                                                                let cur = positions.peek().clone();
                                                                let target = layout_subgraph(&cone, &ev, &cur);
                                                                let vb = visible_box(*rect.peek(), true);
                                                                let pts: Vec<(f64, f64)> =
                                                                    target.values().copied().collect();
                                                                let (to_pan, to_zoom) = fit_view_into(&pts, vb);
                                                                saved_positions.set(Some((cur.clone(), *pan.peek(), *zoom.peek())));
                                                                let delay = stagger_delay(key, &ev);
                                                                tween.set(Some(make_tween(
                                                                    cur,
                                                                    target,
                                                                    *pan.peek(),
                                                                    to_pan,
                                                                    *zoom.peek(),
                                                                    to_zoom,
                                                                    delay,
                                                                )));
                                                                focus_space.set(Some(cone));
                                                                crumbs.set(vec![key]);
                                                            }
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                                drag.set(None);
                                                hover.set(None);
                                            },

                                            rect {
                                                class: "cursor-grab",
                                                x: "{x - NODE_W / 2.0:.0}",
                                                y: "{y - NODE_H / 2.0:.0}",
                                                width: "{NODE_W:.0}",
                                                height: "{NODE_H:.0}",
                                                rx: "{rx}",
                                                fill: "{fill}",
                                                fill_opacity: "{fill_op}",
                                                // Selected nodes get a white ring; non-connected ones
                                                // dim out via focus, so the picks stay recognizable.
                                                stroke: if sel || (legal && hov) { "#fafafa" } else { node_color },
                                                stroke_width: if hov || legal || sel { sw_hov } else { sw },
                                            }
                                            text {
                                                x: "{x:.0}",
                                                y: "{title_y:.0}",
                                                text_anchor: "middle",
                                                fill: "{title_c}",
                                                font_size: "12",
                                                font_weight: "600",
                                                pointer_events: "none",
                                                "{title_text}"
                                            }
                                            if !sub_text.is_empty() {
                                                text {
                                                    x: "{x:.0}",
                                                    y: "{y + 12.0:.0}",
                                                    text_anchor: "middle",
                                                    fill: "#71717a",
                                                    font_size: "10",
                                                    pointer_events: "none",
                                                    "{sub_text}"
                                                }
                                            }
                                            // Port dots (wire start)
                                            if ports != "top" {
                                                circle {
                                                    class: "cursor-crosshair",
                                                    cx: "{x:.0}",
                                                    cy: "{y + NODE_H / 2.0:.0}",
                                                    r: "4.5",
                                                    fill: "{node_color}",
                                                    stroke: "#09090b",
                                                    stroke_width: "1.5",
                                                    onmousedown: move |e| {
                                                        e.stop_propagation();
                                                        drag.set(Some(Drag::Wire { src: key }));
                                                        let c = e.client_coordinates();
                                                        cursor_world.set(to_world(c.x, c.y));
                                                    },
                                                }
                                            }
                                            if ports != "bottom" {
                                                circle {
                                                    class: "cursor-crosshair",
                                                    cx: "{x:.0}",
                                                    cy: "{y - NODE_H / 2.0:.0}",
                                                    r: "4.5",
                                                    fill: "{node_color}",
                                                    stroke: "#09090b",
                                                    stroke_width: "1.5",
                                                    onmousedown: move |e| {
                                                        e.stop_propagation();
                                                        drag.set(Some(Drag::Wire { src: key }));
                                                        let c = e.client_coordinates();
                                                        cursor_world.set(to_world(c.x, c.y));
                                                    },
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                }
                if let Some(node) = inspect() {
                    NodeInspector {
                        node: node,
                        crumbs: crumbs(),
                        on_crumb: move |i: usize| {
                            let trail = crumbs();
                            if let Some(back) = trail.get(i).copied() {
                                // 回到第 i 个：截断其后的记录
                                crumbs.set(trail[..=i].to_vec());
                                inspect.set(Some(back));
                            }
                        },
                        on_close: move |_| {
                            inspect.set(None);
                            if let Some((saved, saved_pan, saved_zoom)) = saved_positions.take() {
                                let from = positions.peek().clone();
                                let (to_pan, to_zoom) = (saved_pan, saved_zoom);
                                let mut delay = HashMap::new();
                                for (i, k) in saved.keys().enumerate() {
                                    delay.insert(*k, i as f64 * 0.06);
                                }
                                tween.set(Some(make_tween(
                                    from,
                                    saved,
                                    *pan.peek(),
                                    to_pan,
                                    *zoom.peek(),
                                    to_zoom,
                                    delay,
                                )));
                            }
                            focus_space.set(None);
                        },
                    }
                }
            }
        }
    }
}

fn edges_read(edges: &Signal<HashSet<(NodeKey, NodeKey)>>) -> HashSet<(NodeKey, NodeKey)> {
    edges.read().clone()
}
// ---- Layout ----

/// 三层可见节点。调度模型全部平铺——不再有渠道聚合卡，
/// 因为渠道是凭证容器（在设置页编辑），不是图上的节点。
fn visible_layers() -> [Vec<NodeKey>; 3] {
    [
        (0..GROUPS.len()).map(NodeKey::Group).collect(),
        (0..ALIASES.len()).map(NodeKey::Mapping).collect(),
        (0..DISPATCH.len()).map(NodeKey::Dispatch).collect(),
    ]
}

/// 待绘制的边。节点不再折叠，故显示边即存储边；第三元保留
/// 原始边（删除时用），维持调用方签名不变。
fn display_edge_pairs(
    edges: &HashSet<(NodeKey, NodeKey)>,
) -> Vec<(NodeKey, NodeKey, (NodeKey, NodeKey))> {
    edges.iter().map(|&(u, l)| (u, l, (u, l))).collect()
}

/// One physics frame for the layered graph. x only — y is pinned to the row.
/// One physics frame. Nodes roam the full 2D plane — dropped where you drop
/// them, nothing snaps back to any row. Two boids-style forces only:
/// - rope spring on each link: zero while slack, tugs beyond REST length
/// - all-pairs separation: overlapping nodes push apart like billiard balls
/// Returns max speed for the sleep decision; `held` follows the cursor.
fn physics_step(
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
        let (pu, pl) = (positions[&up], positions[&low]);
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

/// 侧边抽屉：左键点节点后就地编辑该实体。
///
/// `absolute` 覆盖画布右侧，画布尺寸恒定，开合不引起重排。
/// 三层的编辑内容不同：
/// - 分组：名字（可改）
/// - 模型别名：名字（可改）
/// - 调度模型：模型名**只读**（来自上游，改了就路由不到），
///   附带展示所属渠道的 URL/Key，渠道本身在设置页改
#[component]
fn NodeInspector(
    node: NodeKey,
    crumbs: Vec<NodeKey>,
    on_crumb: EventHandler<usize>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let title = node_title(node);
    let kind_label = match node {
        NodeKey::Group(_) => "分组",
        NodeKey::Mapping(_) => "模型别名",
        NodeKey::Dispatch(_) => "调度模型",
    };
    let accent = color(node);

    rsx! {
        aside { class: "absolute inset-y-0 right-0 z-20 flex w-full flex-col border-l border-zinc-800 bg-zinc-900/97 backdrop-blur sm:w-[320px]",
            // 面包屑：焦点空间内浏览过哪些节点；点击回走，不改树。
            if crumbs.len() > 1 {
                div { class: "flex shrink-0 flex-wrap items-center gap-1 border-b border-zinc-800 px-3 py-1.5",
                    for (i, crumb) in crumbs.iter().enumerate() {
                        {
                            let is_last = i == crumbs.len() - 1;
                            let label = node_title(*crumb);
                            let tone = if is_last {
                                "text-zinc-200"
                            } else {
                                "text-zinc-500 hover:text-zinc-300"
                            };
                            rsx! {
                                if i > 0 {
                                    span { class: "text-[11px] text-zinc-700", "/" }
                                }
                                button {
                                    class: "text-[11px] transition-colors {tone}",
                                    disabled: is_last,
                                    onclick: move |_| on_crumb.call(i),
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
            // 头部
            div { class: "flex shrink-0 items-center gap-2 border-b border-zinc-800 px-3 py-2",
                span { class: "h-2.5 w-2.5 shrink-0 rounded-full", style: "background: {accent}" }
                div { class: "min-w-0 flex-1",
                    p { class: "truncate text-sm font-medium text-zinc-100", "{title}" }
                    p { class: "text-[11px] text-zinc-500", "{kind_label}" }
                }
                button {
                    class: "rounded-md px-1.5 text-zinc-500 hover:text-zinc-200",
                    title: "退出焦点空间",
                    onclick: move |e| on_close.call(e),
                    "✕"
                }
            }
            // 主体
            div { class: "min-h-0 flex-1 space-y-3 overflow-y-auto p-3",
                match node {
                    NodeKey::Group(i) => rsx! { GroupInspect { index: i } },
                    NodeKey::Mapping(i) => rsx! { AliasInspect { index: i } },
                    NodeKey::Dispatch(i) => rsx! { DispatchInspect { index: i } },
                }
            }
            // 底部操作条
            div { class: "flex shrink-0 items-center gap-2 border-t border-zinc-800 px-3 py-2",
                button { class: "rounded-md border border-zinc-800 px-2.5 py-1 text-xs text-zinc-400 hover:border-red-700 hover:text-red-400", "删除" }
                span { class: "flex-1" }
                button { class: "rounded-md border border-zinc-100 bg-zinc-100 px-2.5 py-1 text-xs font-medium text-zinc-900 hover:bg-zinc-300", "保存" }
            }
        }
    }
}

#[component]
fn GroupInspect(index: usize) -> Element {
    // 与「设置」tab 共享 store：这里改名，那边立即可见。
    let mut store = use_context::<EntityStore>();
    let row = store.groups.read().get(index).cloned();
    let aliases: Vec<&'static str> = SEED_EDGES
        .iter()
        .filter(|(u, _)| *u == NodeKey::Group(index))
        .filter_map(|(_, l)| match l {
            NodeKey::Mapping(i) => Some(ALIASES[*i].name),
            _ => None,
        })
        .collect();
    let Some(r) = row else {
        return rsx! { p { class: "text-xs text-zinc-600", "该分组不存在" } };
    };

    rsx! {
        BoundField {
            label: "分组名",
            value: r.name,
            placeholder: "vip",
            on_change: move |v: String| store.groups.write()[index].name = v,
        }
        BoundField {
            label: "展示名",
            value: r.display,
            placeholder: "默认分组",
            on_change: move |v: String| store.groups.write()[index].display = v,
        }
        InspectList { title: "包含的模型别名", items: aliases, empty: "拖端口连线以加入别名" }
    }
}

#[component]
fn AliasInspect(index: usize) -> Element {
    let mut store = use_context::<EntityStore>();
    let row = store.aliases.read().get(index).cloned();
    let groups: Vec<&'static str> = SEED_EDGES
        .iter()
        .filter(|(_, l)| *l == NodeKey::Mapping(index))
        .filter_map(|(u, _)| match u {
            NodeKey::Group(i) => Some(GROUPS[*i].name),
            _ => None,
        })
        .collect();
    let dispatch: Vec<&'static str> = SEED_EDGES
        .iter()
        .filter(|(u, _)| *u == NodeKey::Mapping(index))
        .filter_map(|(_, l)| match l {
            NodeKey::Dispatch(i) => Some(DISPATCH[*i].name),
            _ => None,
        })
        .collect();
    let Some(r) = row else {
        return rsx! { p { class: "text-xs text-zinc-600", "该别名不存在" } };
    };

    rsx! {
        BoundField {
            label: "别名",
            value: r.alias,
            placeholder: "gpt-4o",
            on_change: move |v: String| store.aliases.write()[index].alias = v,
        }
        BoundField {
            label: "展示名",
            value: r.display,
            placeholder: "GPT-4o",
            on_change: move |v: String| store.aliases.write()[index].display = v,
        }
        InspectList { title: "所属分组", items: groups, empty: "未加入任何分组" }
        InspectList { title: "路由到的调度模型", items: dispatch, empty: "未连接调度模型" }
    }
}

#[component]
fn DispatchInspect(index: usize) -> Element {
    let mut store = use_context::<EntityStore>();
    let d = &DISPATCH[index];
    let ci = d.channel;
    let row = store.channels.read().get(ci).cloned();
    let aliases: Vec<&'static str> = SEED_EDGES
        .iter()
        .filter(|(_, l)| *l == NodeKey::Dispatch(index))
        .filter_map(|(u, _)| match u {
            NodeKey::Mapping(i) => Some(ALIASES[*i].name),
            _ => None,
        })
        .collect();

    rsx! {
        div { class: "space-y-1",
            span { class: "text-[11px] text-zinc-500", "模型名（只读，来自上游）" }
            div { class: "rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 font-mono text-sm text-zinc-300", "{d.name}" }
        }
        if let Some(c) = row {
            div { class: "space-y-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3",
                span { class: "text-[11px] uppercase tracking-wider text-zinc-600", "所属渠道" }
                BoundField {
                    label: "渠道名称",
                    value: c.name,
                    placeholder: "OpenAI 官方",
                    on_change: move |v: String| store.channels.write()[ci].name = v,
                }
                BoundField {
                    label: "Base URL",
                    value: c.url,
                    placeholder: "https://…",
                    on_change: move |v: String| store.channels.write()[ci].url = v,
                }
                BoundField {
                    label: "API Key",
                    value: c.keys,
                    placeholder: "sk-…",
                    on_change: move |v: String| store.channels.write()[ci].keys = v,
                }
            }
        }
        InspectList { title: "被哪些别名路由", items: aliases, empty: "未被任何别名引用" }
    }
}

/// 受控输入：直接写回 store 对应字段，随打随存。
#[component]
fn BoundField(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |e| on_change.call(e.value()),
            }
        }
    }
}

#[component]
fn CredRow(label: &'static str, value: &'static str) -> Element {
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "w-8 shrink-0 text-[11px] text-zinc-600", "{label}" }
            span { class: "truncate font-mono text-[11px] text-zinc-400", "{value}" }
        }
    }
}

#[component]
fn InspectField(label: &'static str, value: Signal<String>, placeholder: &'static str) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value.read()}",
                placeholder: "{placeholder}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

#[component]
fn InspectList(title: &'static str, items: Vec<&'static str>, empty: &'static str) -> Element {
    rsx! {
        div { class: "space-y-1.5",
            span { class: "text-[11px] text-zinc-500", "{title}" }
            if items.is_empty() {
                p { class: "text-[11px] text-zinc-600", "{empty}" }
            } else {
                div { class: "flex flex-wrap gap-1.5",
                    for it in items.iter() {
                        span { class: "inline-flex items-center gap-1 rounded-full border border-zinc-700 bg-zinc-900 px-2 py-0.5 text-[11px] text-zinc-300",
                            "{it}"
                            button { class: "text-zinc-600 hover:text-red-400", "✕" }
                        }
                    }
                }
            }
        }
    }
}
