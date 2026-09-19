use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use client::ApiClient;
use contract::api::admin::{ChannelDto, GroupDto};

use crate::api::{ModelView, list_channels_api, list_groups_api, list_models_api};
use crate::state::EntityStore;

use super::shared::{MSG_LOAD_CHANNELS_FAILED, MSG_LOAD_GROUPS_FAILED, MSG_LOAD_MODELS_FAILED};

/// 拓扑写路径成功后的画布刷新信号（#183 起画布由 `load_network_data`
/// 真实数据驱动，store 行不再是事实源）。写函数（`crate::drawer_write`
/// 的分组/渠道 CRUD）成功后调 `bump_topo_refresh()`；`NetworkPanel`
/// 挂载时启动一个 400ms 轮询，发现版本号变化即重拉三端点并重建
/// edges / positions（与挂载时同一条链路）。
static TOPO_REFRESH: AtomicU64 = AtomicU64::new(0);

/// 写操作成功后调用：递增版本号，让网络页画布重拉真实拓扑数据。
pub fn bump_topo_refresh() {
    TOPO_REFRESH.fetch_add(1, Ordering::SeqCst);
}

/// 读当前版本号（`NetworkPanel` 的轮询 hook 消费）。
pub fn topo_refresh_version() -> u64 {
    TOPO_REFRESH.load(Ordering::SeqCst)
}

/// 拓扑数据源三态:loading → 成功后带 Some(view),error → 保留错误信息,
/// empty 由调用方对 `GraphView` 的层数判断(三组全空)。
pub type NetworkResult = Result<GraphView, String>;

/// 从 store 派生的图快照：拓扑图、抽屉、设置页共用同一事实源，
/// 任一侧改名/增删，其他侧立即反映。
#[doc(hidden)]
#[derive(Clone)]
pub struct GraphView {
    /// 分组名（按 store 顺序；即 NodeKey::Group(i) 中的 i）
    pub groups: Vec<String>,
    /// 模型别名
    pub aliases: Vec<String>,
    /// 渠道名
    pub channels: Vec<String>,
    /// 调度模型：(渠道序号, 模型名)，展开自每个渠道的 dispatch
    pub dispatch: Vec<(usize, String)>,
    /// 每个渠道服务的分组名(与后端 ChannelDto.groups 对齐;
    /// 空 Vec = 该渠道不服务任何分组,不产生 分组→别名 边)
    pub channel_groups: Vec<Vec<String>>,
}

impl GraphView {
    pub fn from_store(store: &EntityStore) -> Self {
        let channels = store.channels.read();
        let groups: Vec<String> = store.groups.read().iter().map(|g| g.name.clone()).collect();
        let aliases: Vec<String> = store
            .aliases
            .read()
            .iter()
            .map(|a| a.alias.clone())
            .collect();
        let channel_names: Vec<String> = channels.iter().map(|c| c.name.clone()).collect();
        let dispatch: Vec<(usize, String)> = channels
            .iter()
            .enumerate()
            .flat_map(|(ci, c)| c.dispatch.iter().map(move |m| (ci, m.clone())))
            .collect();
        // 本地 store 的 ChannelRow.group 是逗号分隔字符串,按后端
        // ChannelDto.groups 的 Vec 形状展开
        let channel_groups: Vec<Vec<String>> = channels
            .iter()
            .map(|c| c.group.split(',').map(|s| s.trim().to_string()).collect())
            .collect();
        Self {
            groups,
            aliases,
            channels: channel_names,
            dispatch,
            channel_groups,
        }
    }

    /// 真实数据版:从分组/模型/渠道三组 DTO 直接组快照(不经过本地 store)。
    /// 渠道的 `models` JSONB 形状与快照展开规则一致:
    /// - 字符串数组 `["gpt-4o"]` → dispatch 模型名即该字符串;
    /// - 对象数组 `[{"alias":"gpt-4o","upstream":"gpt-4o-2024-05"}]` → 用 `alias`(对外名)。
    ///
    /// 解析失败/形状不符的条目静默跳过,不阻塞整图。
    pub fn from_dtos(groups: &[GroupDto], models: &[ModelView], channels: &[ChannelDto]) -> Self {
        Self {
            groups: groups.iter().map(|g| g.name.clone()).collect(),
            aliases: models.iter().map(|m| m.name.clone()).collect(),
            channels: channels.iter().map(|c| c.name.clone()).collect(),
            dispatch: channels
                .iter()
                .enumerate()
                .flat_map(|(ci, c)| channel_models(&c.models).into_iter().map(move |m| (ci, m)))
                .collect(),
            channel_groups: channels.iter().map(|c| c.groups.clone()).collect(),
        }
    }
}

/// 展开渠道的 `models` JSONB → 对外模型名列表(对象取 alias,字符串直用)。
/// 纯函数,DTO 形状断言见 tests/network_wire.rs。
pub fn channel_models(models: &serde_json::Value) -> Vec<String> {
    models
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| {
                    m.as_str().map(|s| s.to_string()).or_else(|| {
                        m.get("alias")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 三组端点并发拉取真实拓扑数据。全 401/网络错时统一返回 Err(错误摘要),
/// 由调用方渲染错误态。纯异步,无 UI 依赖。
pub async fn load_network_data(client: &ApiClient) -> NetworkResult {
    // 三组端点真正并发：join! 等价于 max(各请求耗时) 而非顺序求和
    // （ocr finding：注释声称并发却顺序 await，属注释撒谎）。
    let (groups, channels, models) = futures_util::join!(
        list_groups_api(client),
        list_channels_api(client),
        list_models_api(client),
    );
    let groups = groups.map_err(|e| format!("{MSG_LOAD_GROUPS_FAILED}{e}"))?;
    let channels = channels.map_err(|e| format!("{MSG_LOAD_CHANNELS_FAILED}{e}"))?;
    let models = models.map_err(|e| format!("{MSG_LOAD_MODELS_FAILED}{e}"))?;
    Ok(GraphView::from_dtos(&groups, &models, &channels))
}

/// 边推导:与后端快照展开规则(admin snapshot `expand_models_json`)一致 ——
/// 每个渠道按 `channel_groups` 笛卡尔积挂到它服务的每个分组,每个对外模型
/// (dispatch)挂到同名别名节点(没有同名别名时该模型悬空,不产生边);
/// 渠道 groups 为空则不服务任何分组,只留 别名→调度模型 边。
/// 纯函数,形状断言见 tests/network_wire.rs。
pub fn edges_of(view: &GraphView) -> Vec<(NodeKey, NodeKey)> {
    let alias_of = |name: &str| -> Option<usize> { view.aliases.iter().position(|a| a == name) };
    let mut out: Vec<(NodeKey, NodeKey)> = Vec::new();
    for ci in 0..view.channels.len() {
        let models: Vec<String> = view
            .dispatch
            .iter()
            .filter_map(|(c, m)| if *c == ci { Some(m.clone()) } else { None })
            .collect();
        for m in &models {
            let Some(ai) = alias_of(m) else {
                continue;
            };
            // 渠道服务的每个分组都拿到它的对外模型(笛卡尔积)
            for g in view
                .channel_groups
                .get(ci)
                .map(|v| v.as_slice())
                .unwrap_or_default()
            {
                if let Some(gi) = view.groups.iter().position(|x| x == g) {
                    out.push((NodeKey::Group(gi), NodeKey::Mapping(ai)));
                }
            }
            // 别名 → 调度模型(按渠道序号挂到该渠道的 dispatch 节点)
            out.push((NodeKey::Mapping(ai), NodeKey::Dispatch(ci)));
        }
    }
    out
}

/// 调色板：store 里条目可增删，颜色按序号取模循环，不存进 store。
pub const GROUP_PALETTE: &[&str] = &[
    "#e5484d", "#3e9bff", "#30a46c", "#e5c558", "#d946ef", "#f97316",
];
pub const ALIAS_PALETTE: &[&str] = &[
    "#f472b6", "#a78bfa", "#22d3ee", "#fb923c", "#34d399", "#f87171",
];

pub fn view_color(_view: &GraphView, key: NodeKey) -> &'static str {
    match key {
        NodeKey::Group(i) => GROUP_PALETTE[i % GROUP_PALETTE.len()],
        NodeKey::Mapping(i) => ALIAS_PALETTE[i % ALIAS_PALETTE.len()],
        NodeKey::Dispatch(_) => "#3f3f46",
    }
}

pub fn view_title(view: &GraphView, key: NodeKey) -> String {
    match key {
        NodeKey::Group(i) => view.groups.get(i).cloned().unwrap_or_default(),
        NodeKey::Mapping(i) => view.aliases.get(i).cloned().unwrap_or_default(),
        NodeKey::Dispatch(i) => view
            .dispatch
            .get(i)
            .map(|(_, n)| n.clone())
            .unwrap_or_default(),
    }
}

pub fn view_subtitle(view: &GraphView, key: NodeKey) -> String {
    match key {
        NodeKey::Dispatch(i) => view
            .dispatch
            .get(i)
            .and_then(|(ci, _)| view.channels.get(*ci))
            .cloned()
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// 按 GraphView 尺寸算出可见层。
#[doc(hidden)]
pub fn visible_layers_of(view: &GraphView) -> [Vec<NodeKey>; 3] {
    [
        (0..view.groups.len()).map(NodeKey::Group).collect(),
        (0..view.aliases.len()).map(NodeKey::Mapping).collect(),
        (0..view.dispatch.len()).map(NodeKey::Dispatch).collect(),
    ]
}

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
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NodeKey {
    Group(usize),
    Mapping(usize),
    /// 调度模型：渠道下真实可用的上游模型。
    Dispatch(usize),
}

impl NodeKey {
    pub fn layer(self) -> u8 {
        match self {
            NodeKey::Group(_) => 0,
            NodeKey::Mapping(_) => 1,
            NodeKey::Dispatch(_) => 2,
        }
    }
}

#[doc(hidden)]
pub const MARGIN: f64 = 110.0;
pub const COL_GAP: f64 = 130.0;
// 三层收紧成地图式分层；横向空间留给节点本身，宽层再折行。
pub const ROW_Y: [f64; 3] = [100.0, 330.0, 560.0];
/// Each layer roams freely inside its own horizontal band (±90 around row).
pub const BAND_HALF: f64 = 90.0;
pub fn band_y(layer: u8) -> (f64, f64) {
    (
        ROW_Y[layer as usize] - BAND_HALF,
        ROW_Y[layer as usize] + BAND_HALF,
    )
}
// 9 个调度模型按两排显示，避免用过宽 viewBox 把节点缩小。
#[doc(hidden)]
pub const VIEW_W: f64 = MARGIN * 2.0 + 7.0 * COL_GAP;
#[doc(hidden)]
pub const VIEW_H: f64 = 700.0;
#[doc(hidden)]
pub const NODE_W: f64 = 104.0;
#[doc(hidden)]
pub const NODE_H: f64 = 36.0;

/// Deterministic startup layout, computed from the graph — no physics involved:
/// groups spread evenly; each mapping sits at the average x of the groups it
/// links to; each channel/model sits at the average x of its linked mappings.
/// Same-type spacing enforced with a left-to-right sweep. Physics then only
/// handles collisions and user drags, so the graph no longer reels inward
/// on open (the old grid was wider than the rope slack radius, so every link
/// started taut and pulled everything toward the center).
#[doc(hidden)]
pub fn initial_positions(view: &GraphView) -> HashMap<NodeKey, (f64, f64)> {
    let layers = visible_layers_of(view);
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
    let edges = edges_of(view);
    for l in 1..3 {
        let mut placed: Vec<(NodeKey, f64)> = Vec::new();
        for &k in &layers[l] {
            let xs: Vec<f64> = edges
                .iter()
                .filter_map(|&(u, lo)| {
                    if lo == k {
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
        // 每排限制为 5 个；多出来的节点叠到下一排，保持节点可读。
        const MAX_PER_ROW: usize = 5;
        let per_row = MAX_PER_ROW.min(placed.len()).max(1);
        let row_count = placed.len().div_ceil(per_row);
        for (chunk_i, chunk) in placed.chunks(per_row).enumerate() {
            let row_y = ROW_Y[l]
                + (chunk_i as f64 - (row_count.saturating_sub(1) as f64 / 2.0)) * (NODE_H + 14.0);
            let mut row: Vec<(NodeKey, f64)> = Vec::with_capacity(chunk.len());
            let mut prev = f64::NEG_INFINITY;
            for &(k, x) in chunk {
                let x = x.max(prev + COL_GAP);
                prev = x;
                row.push((k, x));
            }
            // 保留 barycenter 的相对顺序，同时把每排重新居中，保证两侧
            // 留出抽屉和画布边缘的安全区。
            let mean = row.iter().map(|(_, x)| *x).sum::<f64>() / row.len() as f64;
            let shift = VIEW_W / 2.0 - mean;
            for (k, x) in row {
                out.insert(k, ((x + shift).clamp(MARGIN, VIEW_W - MARGIN), row_y));
            }
        }
    }
    out
}

/// Node→node edge must span exactly one layer; channels are aggregates only.
pub fn normalize(a: NodeKey, b: NodeKey) -> Option<(NodeKey, NodeKey)> {
    let la = a.layer() as i16;
    let lb = b.layer() as i16;
    if a == b || (la - lb).abs() != 1 {
        return None;
    }
    Some(if la < lb { (a, b) } else { (b, a) })
}

#[doc(hidden)]
pub fn bezier(a: (f64, f64), b: (f64, f64)) -> String {
    let mid = (a.1 + b.1) / 2.0;
    path_str(a, (a.0, mid), (b.0, mid), b)
}

fn path_str(a: (f64, f64), c1: (f64, f64), c2: (f64, f64), b: (f64, f64)) -> String {
    format!(
        "M {:.0} {:.0} C {:.0} {:.0}, {:.0} {:.0}, {:.0} {:.0}",
        a.0, a.1, c1.0, c1.1, c2.0, c2.1, b.0, b.1
    )
}

#[doc(hidden)]
pub fn cubic_at(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    t: f64,
) -> (f64, f64) {
    let u = 1.0 - t;
    (
        u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0,
        u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1,
    )
}

/// Fraction of lateral control-point offset that dodges blocker rects:
/// first collision-free candidate wins, else the least-colliding one.
#[doc(hidden)]
pub fn dodge_frac(a: (f64, f64), b: (f64, f64), blockers: &[(f64, f64)]) -> f64 {
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
pub fn offset_bezier(a: (f64, f64), b: (f64, f64), frac: f64) -> String {
    if frac == 0.0 {
        return bezier(a, b);
    }
    let dist = (b.0 - a.0).hypot(b.1 - a.1);
    let mid = (a.1 + b.1) / 2.0;
    path_str(a, (a.0 + frac * dist, mid), (b.0 + frac * dist, mid), b)
}

/// Pan/zoom so all points sit inside the viewBox with padding. Returns (pan, zoom).
/// 抽屉宽度（客户端 px），与 NodeInspector 的 `sm:w-[320px]` 一致。
/// 焦点子图要避开它。
pub const DRAWER_W: f64 = 320.0;

/// 焦点适配：把点集缩放平移到画布上「抽屉左侧的可视区」正中心。
/// 坐标先折成 CSS 像素再折算回 viewBox，就不会被 preserveAspectRatio
/// 的 letterbox 横移骗到。
pub fn fit_view_into(
    pts: &[(f64, f64)],
    rect: Option<(f64, f64, f64, f64)>,
    drawer: bool,
) -> ((f64, f64), f64) {
    let Some((_, _, rw, rh)) = rect else {
        return ((0.0, 0.0), 1.0);
    };
    let s = (rw / VIEW_W).min(rh / VIEW_H);
    if s <= 0.0 {
        return ((0.0, 0.0), 1.0);
    }
    let ox = (rw - VIEW_W * s) / 2.0; // letterbox 左侧偏移（viewBox 像素）
    let oy = (rh - VIEW_H * s) / 2.0;

    // 抽屉占去的 CSS 宽；窄屏（抽屉是底部抽屉）不留边
    let drawer_css = if drawer && rw >= 640.0 { DRAWER_W } else { 0.0 };
    let avail = (rw - drawer_css).max(240.0);

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
    let pad_css = 48.0;
    let z =
        (((avail - pad_css * 2.0) / (w * s)).min((rh - pad_css * 2.0) / (h * s))).clamp(0.35, 1.6);
    let (cx, cy) = ((minx + maxx) / 2.0, (miny + maxy) / 2.0);
    // css = ox + (pan + world * z) * s  →  pan = (target - ox)/s - world * z
    let pan_x = (avail / 2.0 - ox) / s - cx * z;
    let pan_y = (rh / 2.0 - oy) / s - cy * z;
    ((pan_x, pan_y), z)
}

/// 连通锥：从起点向外扩散，每步必须**远离**起点所在层。
/// 这样点别名只拉出「它的分组 + 它的调度模型」，
/// 而不会经由分组把整张图都拽进来。
pub fn focus_cone(start: NodeKey, edges: &[(NodeKey, NodeKey)]) -> HashSet<NodeKey> {
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
            if let Some(m) = next
                && seen.insert(m)
            {
                queue.push_back(m);
            }
        }
    }
    seen
}
/// 焦点子图的一次性排布：每层按当前 x 顺序均匀铺开（保持相对次序，
/// 视觉跳动最小），再松弛让上层压在下层重心上。
/// 只在进入焦点时算一次；之后位置交给物理与拖拽。
/// 排布按可视框宽度走：每层按其当前 x 保持相对次序，在框内均匀摊开；
/// 再做几轮轻重心的 barycenter 渐拢让连线尽量垂直（力矩减半，避免塌列）。
/// 只在进入焦点时算一次；之后位置交给拖拽，物理不接管（见 ticker）。
pub fn layout_subgraph(
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
        // 围绕世界中心铺开，不要往可视框里算：位置是世界坐标，
        // fit_view_into 负责再把世界框到可视框（两个变换叠加会偏）。
        // 一排超过 5 个就折行，和初始布局同一套规则。
        const MAX_PER_ROW: usize = 5;
        let per_row = MAX_PER_ROW.min(row.len()).max(1);
        let row_count = row.len().div_ceil(per_row);
        let sub_edges: Vec<(NodeKey, NodeKey)> = edges
            .iter()
            .copied()
            .filter(|(u, lo)| sub.contains(u) && sub.contains(lo))
            .collect();
        for (ci, chunk) in row.chunks(per_row).enumerate() {
            let y = ROW_Y[l]
                + (ci as f64 - (row_count.saturating_sub(1) as f64 / 2.0)) * (NODE_H + 14.0);
            let n = chunk.len();
            let center = VIEW_W / 2.0;
            let span = (n as f64 - 1.0).max(0.0) * COL_GAP;
            for (i, &k) in chunk.iter().enumerate() {
                out.insert(k, (center - span / 2.0 + i as f64 * COL_GAP, y));
            }
            // 往邻居重心收，再按行重心强制最小间距，保证不叠一坨
            for &k in chunk {
                let (mut sum, mut cnt) = (0.0f64, 0.0f64);
                for &(u, lo) in &sub_edges {
                    let other = if u == k {
                        Some(lo)
                    } else if lo == k {
                        Some(u)
                    } else {
                        None
                    };
                    if let Some(o) = other
                        && let Some(op) = out.get(&o)
                    {
                        sum += op.0;
                        cnt += 1.0;
                    }
                }
                if cnt > 0.0 {
                    let cur_x = out[&k].0;
                    out.insert(k, (cur_x + (sum / cnt - cur_x) * 0.45, y));
                }
            }
            let mut sorted: Vec<NodeKey> = chunk.to_vec();
            sorted.sort_by(|a, b| out[a].0.partial_cmp(&out[b].0).unwrap());
            if sorted.len() > 1 {
                let span = (sorted.len() as f64 - 1.0) * COL_GAP;
                let cx: f64 = sorted.iter().map(|k| out[k].0).sum::<f64>() / sorted.len() as f64;
                for (i, &k) in sorted.iter().enumerate() {
                    out.insert(k, (cx - span / 2.0 + i as f64 * COL_GAP, y));
                }
            }
        }
    }
    out
}

/// 位置/视图补间。进出焦点时平滑移动，节点不瞬移。
#[derive(Clone)]
pub struct Tween {
    pub from: HashMap<NodeKey, (f64, f64)>,
    pub to: HashMap<NodeKey, (f64, f64)>,
    pub from_pan: (f64, f64),
    pub to_pan: (f64, f64),
    pub from_zoom: f64,
    pub to_zoom: f64,
    /// 每个节点的错峰延迟（秒），让队列次第滑入（而非整段卡死）。
    pub delay: HashMap<NodeKey, f64>,
    /// 开始时刻（performance.now()，毫秒）。用真实时间驱动进度，
    /// 帧率抖动就不会造成节奏忽快忽慢。
    pub started_ms: f64,
    /// 总时长（毫秒，不含错峰）。
    pub duration_ms: f64,
    /// 0 → 1
    pub t: f64,
}

/// 两次进入更顺滑：前期快启动，末段缓刹。
#[doc(hidden)]
pub fn ease_out_quint(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(5)
}

/// 给锥内节点算错峰延迟：从起点向外传播，越远的越晚进。
pub fn stagger_delay(start: NodeKey, edges: &[(NodeKey, NodeKey)]) -> HashMap<NodeKey, f64> {
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
            if let std::collections::hash_map::Entry::Vacant(e) = delay.entry(m) {
                e.insert(d0 + 0.09);
                queue.push_back(m);
            }
        }
    }
    // 退出时反着来：先动的先回
    delay
}

/// Tween 统一构造，省得三个位置重复写字段。
pub fn make_tween(
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
        started_ms: now_ms(),
        duration_ms: 780.0,
        t: 0.0,
    }
}

pub fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

#[doc(hidden)]
pub fn fit_view(pts: &[(f64, f64)]) -> ((f64, f64), f64) {
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

/// 右侧抽屉的页别:节点检视是默认,设置/导入换成对应面板。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DrawerTab {
    Node,
    Settings,
    Import,
}

#[derive(Clone, Copy)]
pub enum Drag {
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

// —— 共享文案已迁至 `tab-page-network/shared.rs` ——
// LBL_GROUP / LBL_ALIAS / LBL_DISPATCH / LBL_NODES / BTN_IMPORT / BTN_SETTINGS /
// BTN_FIT / FIELD_DISPLAY / EXAMPLE_CHANNEL 现定义在 `super::shared`,
// 本文件不再重定义,调用方改为 `use super::shared::...`。
