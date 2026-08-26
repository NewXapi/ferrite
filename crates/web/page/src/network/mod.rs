use std::collections::{HashMap, HashSet, VecDeque};

use dioxus::prelude::*;

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

struct MappingDef {
    name: &'static str,
    color: &'static str,
}

struct ModelDef {
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

const MAPPINGS: &[MappingDef] = &[
    MappingDef {
        name: "gpt-4o",
        color: "#f472b6",
    },
    MappingDef {
        name: "gpt-5",
        color: "#a78bfa",
    },
    MappingDef {
        name: "claude-sonnet-4",
        color: "#22d3ee",
    },
    MappingDef {
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

const MODELS: &[ModelDef] = &[
    ModelDef {
        channel: 0,
        name: "gpt-4o",
    },
    ModelDef {
        channel: 0,
        name: "gpt-5",
    },
    ModelDef {
        channel: 1,
        name: "gpt-4o",
    },
    ModelDef {
        channel: 2,
        name: "gpt-4o",
    },
    ModelDef {
        channel: 2,
        name: "gpt-5",
    },
    ModelDef {
        channel: 2,
        name: "claude-sonnet-4",
    },
    ModelDef {
        channel: 3,
        name: "claude-sonnet-4",
    },
    ModelDef {
        channel: 4,
        name: "claude-sonnet-4",
    },
    ModelDef {
        channel: 5,
        name: "gemini-2.5-pro",
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum NodeKey {
    Group(usize),
    Mapping(usize),
    Model(usize),
    /// Collapsed channel card; never appears in stored edges.
    Channel(usize),
}

impl NodeKey {
    fn layer(self) -> u8 {
        match self {
            NodeKey::Group(_) => 0,
            NodeKey::Mapping(_) => 1,
            NodeKey::Model(_) | NodeKey::Channel(_) => 2,
        }
    }
}

fn color(key: NodeKey) -> &'static str {
    match key {
        NodeKey::Group(i) => GROUPS[i].color,
        NodeKey::Mapping(i) => MAPPINGS[i].color,
        NodeKey::Model(_) | NodeKey::Channel(_) => "#3f3f46",
    }
}

fn node_title(key: NodeKey) -> String {
    match key {
        NodeKey::Group(i) => GROUPS[i].name.into(),
        NodeKey::Mapping(i) => MAPPINGS[i].name.into(),
        NodeKey::Model(i) => MODELS[i].name.into(),
        NodeKey::Channel(c) => CHANNELS[c].into(),
    }
}

fn subtitle(key: NodeKey) -> String {
    match key {
        NodeKey::Model(i) => CHANNELS[MODELS[i].channel].into(),
        NodeKey::Channel(c) => {
            let n = MODELS.iter().filter(|m| m.channel == c).count();
            format!("{n} 个模型")
        }
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
    (NodeKey::Mapping(0), NodeKey::Model(0)),
    (NodeKey::Mapping(0), NodeKey::Model(2)),
    (NodeKey::Mapping(0), NodeKey::Model(3)),
    (NodeKey::Mapping(1), NodeKey::Model(1)),
    (NodeKey::Mapping(1), NodeKey::Model(4)),
    (NodeKey::Mapping(2), NodeKey::Model(5)),
    (NodeKey::Mapping(2), NodeKey::Model(6)),
    (NodeKey::Mapping(2), NodeKey::Model(7)),
    (NodeKey::Mapping(3), NodeKey::Model(8)),
];

const MARGIN: f64 = 110.0;
const COL_GAP: f64 = 130.0;
const ROW_Y: [f64; 3] = [90.0, 340.0, 590.0];
/// Each layer roams freely inside its own horizontal band (±80 around row).
const BAND_HALF: f64 = 80.0;
fn band_y(layer: u8) -> (f64, f64) {
    (
        ROW_Y[layer as usize] - BAND_HALF,
        ROW_Y[layer as usize] + BAND_HALF,
    )
}
const VIEW_W: f64 = MARGIN * 2.0 + (MODELS.len() as f64 - 1.0) * COL_GAP;
const VIEW_H: f64 = 700.0;
const NODE_W: f64 = 104.0;
const NODE_H: f64 = 36.0;

/// Deterministic startup layout, computed from the graph — no physics involved:
/// groups spread evenly; each mapping sits at the average x of the groups it
/// links to; each channel/model sits at the average x of its linked mappings.
/// Same-type spacing enforced with a left-to-right sweep. Physics then only
/// handles collisions and user drags, so the graph no longer reels inward
/// on open (the old grid was wider than the rope slack radius, so every link
/// started taut and pulled everything toward the center).
fn initial_positions(expanded: &HashSet<usize>) -> HashMap<NodeKey, (f64, f64)> {
    let layers = visible_layers(expanded);
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
                    let folded = match lo {
                        NodeKey::Model(i) if !expanded.contains(&MODELS[i].channel) => {
                            NodeKey::Channel(MODELS[i].channel)
                        }
                        other => other,
                    };
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

/// Node→node edge must span exactly one layer; channels are aggregates only.
fn normalize(a: NodeKey, b: NodeKey) -> Option<(NodeKey, NodeKey)> {
    let la = a.layer() as i16;
    let lb = b.layer() as i16;
    if a == b
        || (la - lb).abs() != 1
        || matches!(a, NodeKey::Channel(_))
        || matches!(b, NodeKey::Channel(_))
    {
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

/// Cubic bezier that dodges blocker node rects while keeping the curve look:
/// tries growing lateral control-point offsets and keeps the first
/// collision-free one, else the least-colliding.
fn routed_bezier(a: (f64, f64), b: (f64, f64), blockers: &[(f64, f64)]) -> String {
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
            return path_str(a, c1, c2, b);
        }
        if best.is_none() || hits < best.unwrap().0 {
            best = Some((hits, f));
        }
    }
    let off = best.unwrap_or((0, 0.0)).1 * dist;
    path_str(a, (a.0 + off, mid), (b.0 + off, mid), b)
}

/// Pan/zoom so all points sit inside the viewBox with padding. Returns (pan, zoom).
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
    // Group-move anchors: (node, world offset from cursor) for the whole selection.
    let mut moving = use_signal(Vec::<(NodeKey, f64, f64)>::new);
    // Marquee rect in viewBox coords while a Select drag is active.
    let mut marquee = use_signal(|| None::<((f64, f64), (f64, f64))>);
    let mut expanded = use_signal(|| HashSet::from([2usize])); // OneAPI 上游 pre-expanded
    let mut positions = use_signal(|| HashMap::<NodeKey, (f64, f64)>::new());
    // Live spring layout: a 16ms ticker integrates forces frame by frame.
    // Structure changes (wires / collapse) bump `wake`; the loop steps while
    // it's woken, while a node is being dragged, or while energy remains,
    // then sleeps. Dragged nodes are held; live neighbors dodge in real time.
    let mut wake = use_signal(|| 0u32);
    use_effect(move || {
        let _ = (edges(), expanded());
        let next = wake.peek().wrapping_add(1);
        wake.set(next);
    });
    use_hook(move || {
        // Deterministic startup layout before the ticker takes over.
        {
            let ex = expanded.peek().clone();
            let mut p = initial_positions(&ex);
            let layers = visible_layers(&ex);
            let pairs = display_edge_pairs(&edges.peek(), &ex);
            let pairs_xy: Vec<(NodeKey, NodeKey)> = pairs.iter().map(|&(u, l, _)| (u, l)).collect();
            settle_layout(&layers, &pairs_xy, &mut p);
            *positions.write() = p;
        }
        spawn(async move {
            let mut velocities = HashMap::<NodeKey, (f64, f64)>::new();
            let mut seen_wake = *wake.peek();
            let mut energy = f64::MAX;
            loop {
                gloo_timers::future::TimeoutFuture::new(16).await;
                let held = match *drag.peek() {
                    Some(Drag::Move { key, .. }) => Some(key),
                    _ => None,
                };
                let dirty = seen_wake != *wake.peek();
                seen_wake = *wake.peek();
                if !(dirty || held.is_some() || energy > 0.08) {
                    continue;
                }
                let ex = expanded.peek().clone();
                let layers = visible_layers(&ex);
                let pairs = display_edge_pairs(&edges.peek(), &ex);
                let pairs_xy: Vec<(NodeKey, NodeKey)> =
                    pairs.iter().map(|&(u, l, _)| (u, l)).collect();
                energy = physics_step(
                    &layers,
                    &pairs_xy,
                    held,
                    &mut positions.write(),
                    &mut velocities,
                );
            }
        });
    });

    // ---- Visible nodes per layer (collapse-aware) ----
    let expanded_now = expanded();

    let layers = visible_layers(&expanded_now);
    let selection_now = selected();
    let layers_fit = layers.clone(); // owned copy for the 适配 button's handler

    let positions_now = positions();
    let pos = |key: NodeKey| -> (f64, f64) {
        positions_now
            .get(&key)
            .copied()
            .unwrap_or((VIEW_W / 2.0, ROW_Y[key.layer() as usize]))
    };

    let display_edges = display_edge_pairs(&edges(), &expanded_now);

    // ---- Focus set: layer-distance BFS from the selected node ----
    // A step is admitted iff it strictly increases |layer − start.layer|:
    //   group (0)   reaches 0 → 1 → 2
    //   mapping (1) reaches 1 → 0  and  1 → 2  (one hop each, no turnback)
    //   model (2)   reaches 2 → 1 → 0
    // "seen" marks visited nodes (type + id), so cycles terminate on their own.
    let focus: Option<HashSet<NodeKey>> = (if selection_now.len() == 1 {
        selection_now.iter().next().copied()
    } else {
        None
    })
    .map(|start| {
        let start_layer = start.layer() as i16;
        let dist = |k: NodeKey| (k.layer() as i16 - start_layer).abs();
        let mut seen: HashSet<NodeKey> = HashSet::from([start]);
        let mut queue: VecDeque<NodeKey> = VecDeque::from([start]);
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
        seen
    });

    // ---- Coordinate helpers ----
    let to_world = move |client_x: f64, client_y: f64| -> (f64, f64) {
        let Some((rx, ry, rw, rh)) = *rect.peek() else {
            return (client_x, client_y);
        };
        let view = ((client_x - rx) * VIEW_W / rw, (client_y - ry) * VIEW_H / rh);
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

    let port_of = |key: NodeKey| -> &'static str {
        match key {
            NodeKey::Group(_) => "bottom",
            NodeKey::Mapping(_) => "both",
            NodeKey::Model(_) | NodeKey::Channel(_) => "top",
        }
    };

    let hint = match drag_now {
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
                span { class: "text-xs text-zinc-600", "{hint}" }
            }
            div { class: "min-h-0 flex-1 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-950",
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
                            if let Some((rx, ry, rw, rh)) = *rect.peek() {
                                let v = ((c.x - rx) * VIEW_W / rw, (c.y - ry) * VIEW_H / rh);
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
                                    pan.set((px + (c.x - sx) * VIEW_W / rw, py + (c.y - sy) * VIEW_H / rh));
                                }
                                drag.set(Some(Drag::Pan { sx, sy, px, py, moved: moved_flag }));
                            }
                            Some(Drag::Move { key, sx, sy, ox, oy, .. }) => {
                                let moved_flag = (c.x - sx).abs() + (c.y - sy).abs() > 4.0;
                                {
                                    let mut pos_w = positions.write();
                                    for &(k, kox, koy) in moving.peek().iter() {
                                        let (b0, b1) = band_y(k.layer());
                                        pos_w.insert(k, (world.0 + kox, (world.1 + koy).clamp(b0, b1)));
                                    }
                                }
                                drag.set(Some(Drag::Move { key, sx, sy, ox, oy, moved: moved_flag }));
                            }
                            Some(Drag::Wire { .. }) => cursor_world.set(world),
                            Some(Drag::Select) => {
                                if let Some((rx, ry, rw, rh)) = *rect.peek() {
                                    let v = ((c.x - rx) * VIEW_W / rw, (c.y - ry) * VIEW_H / rh);
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
                            Some(Drag::Pan { moved: false, .. }) => selected.set(HashSet::new()),
                            Some(Drag::Select) => {
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
                            }
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
                        if let Some((rx, ry, rw, rh)) = *rect.peek() {
                            let view = ((c.x - rx) * VIEW_W / rw, (c.y - ry) * VIEW_H / rh);
                            let (px, py) = pan();
                            let world = ((view.0 - px) / z0, (view.1 - py) / z0);
                            pan.set((view.0 - world.0 * z, view.1 - world.1 * z));
                        }
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
                        // Layer activity bands (per-type roaming ranges)
                        for l in 0..3u8 {
                            rect {
                                x: "-3000",
                                y: "{band_y(l).0:.0}",
                                width: "{VIEW_W + 6000.0:.0}",
                                height: "{BAND_HALF * 2.0:.0}",
                                fill: "#ffffff",
                                opacity: "0.018",
                                pointer_events: "none",
                            }
                        }

                        // ---- Committed wires ----
                        for (du, dl, raw) in display_edges.clone() {
                            {
                                let wire_color = color(du);
                                let blockers: Vec<(f64, f64)> = layers
                                    .iter()
                                    .flatten()
                                    .filter(|&&k| k != du && k != dl)
                                    .map(|&k| pos(k))
                                    .collect();
                                let d = routed_bezier(anchor(du, true), anchor(dl, false), &blockers);
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
                                    let is_channel = matches!(key, NodeKey::Channel(_));
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
                                                        let mut sel = selected.write();
                                                        if e.modifiers().ctrl() {
                                                            if !sel.remove(&key) {
                                                                sel.insert(key);
                                                            }
                                                        } else {
                                                            sel.clear();
                                                            sel.insert(key);
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                                drag.set(None);
                                                hover.set(None);
                                            },

                                            rect {
                                                class: if is_channel { "cursor-pointer" } else { "cursor-grab" },
                                                x: "{x - NODE_W / 2.0:.0}",
                                                y: "{y - NODE_H / 2.0:.0}",
                                                width: "{NODE_W:.0}",
                                                height: "{NODE_H:.0}",
                                                rx: "{rx}",
                                                fill: "{fill}",
                                                fill_opacity: "{fill_op}",
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
                                            // Expand toggle on channel cards
                                            if is_channel {
                                                g {
                                                    class: "cursor-pointer",
                                                    onmousedown: move |e| e.stop_propagation(),
                                                    onmouseup: move |e| {
                                                        e.stop_propagation();
                                                        if let NodeKey::Channel(c) = key {
                                                            let mut set = expanded.write();
                                                            if !set.remove(&c) { set.insert(c); }
                                                        }
                                                        drag.set(None);
                                                    },
                                                    rect {
                                                        x: "{x + NODE_W / 2.0 - 16.0:.0}",
                                                        y: "{y - 6.0:.0}",
                                                        width: "12",
                                                        height: "12",
                                                        rx: "3",
                                                        fill: "#27272a",
                                                    }
                                                    text {
                                                        x: "{x + NODE_W / 2.0 - 10.0:.0}",
                                                        y: "{y + 3.0:.0}",
                                                        text_anchor: "middle",
                                                        fill: "#a1a1aa",
                                                        font_size: "11",
                                                        pointer_events: "none",
                                                        "＋"
                                                    }
                                                }
                                            }
                                            // Port dots (wire start)
                                            if ports != "top" && !is_channel {
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
                                            if ports != "bottom" && !is_channel {
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
        }
    }
}

fn edges_read(edges: &Signal<HashSet<(NodeKey, NodeKey)>>) -> HashSet<(NodeKey, NodeKey)> {
    edges.read().clone()
}
// ---- Layout ----

/// Visible nodes grouped by layer; a channel collapses to one card otherwise
/// contributes all its model nodes.
fn visible_layers(expanded: &HashSet<usize>) -> [Vec<NodeKey>; 3] {
    let mut layers: [Vec<NodeKey>; 3] = [
        (0..GROUPS.len()).map(NodeKey::Group).collect(),
        (0..MAPPINGS.len()).map(NodeKey::Mapping).collect(),
        Vec::new(),
    ];
    for c in 0..CHANNELS.len() {
        if expanded.contains(&c) {
            layers[2].extend(
                (0..MODELS.len())
                    .filter(|&i| MODELS[i].channel == c)
                    .map(NodeKey::Model),
            );
        } else {
            layers[2].push(NodeKey::Channel(c));
        }
    }
    layers
}

/// Edges as drawn: a Model endpoint folds into its Channel card while the
/// channel is collapsed; duplicates from folded models are dropped. The third
/// element is the stored (raw) edge used for deletion.
fn display_edge_pairs(
    edges: &HashSet<(NodeKey, NodeKey)>,
    expanded: &HashSet<usize>,
) -> Vec<(NodeKey, NodeKey, (NodeKey, NodeKey))> {
    let mut out = Vec::new();
    let mut seen: HashSet<(NodeKey, NodeKey)> = HashSet::new();
    for &(u, l) in edges {
        let dl = match l {
            NodeKey::Model(i) if !expanded.contains(&MODELS[i].channel) => {
                NodeKey::Channel(MODELS[i].channel)
            }
            other => other,
        };
        if u != dl && seen.insert((u, dl)) {
            out.push((u, dl, (u, l)));
        }
    }
    out
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
    // Separation between every node pair, box-aware: nodes are wide pills,
    // so horizontal clearance (node width + 12) and vertical clearance
    // (node height + 8) are separate; the pair resolves along the axis with
    // the smaller penetration, like two rounded boxes touching.
    // O(n²) pairwise scan; fine at this node count.
    let nodes: Vec<NodeKey> = layers.iter().flatten().copied().collect();
    for i in 0..nodes.len() {
        for j in i + 1..nodes.len() {
            let (a, b) = (nodes[i], nodes[j]);
            let (pa, pb) = (positions[&a], positions[&b]);
            let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
            let ox = (NODE_W + 12.0) - dx.abs();
            let oy = (NODE_H + 8.0) - dy.abs();
            if ox <= 0.0 || oy <= 0.0 {
                continue;
            }
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
    for &k in &nodes {
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
