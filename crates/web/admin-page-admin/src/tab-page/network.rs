use dioxus::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::components::network_drawer::{DrawerHeader, DrawerTabs, ImportPanel};
use crate::components::network_inspector::NodeInspector;
use crate::components::network_shared::{
    BTN_FIT, BTN_IMPORT, BTN_SETTINGS, LBL_ALIAS, LBL_CHANNELS, LBL_DISPATCH, LBL_GROUP,
    LBL_IMPORT_SUBTITLE, MSG_EMPTY, MSG_LOADING, MSG_LOADING_ARIA, MSG_WIRE_DELETE_HINT,
    SEC_HINT_FOCUS, SEC_HINT_IDLE, SEC_HINT_MOVING, SEC_HINT_WIRING, SEC_SETTINGS_STALE,
};
use crate::network_data::*;
use crate::network_physics::{display_edge_pairs, edges_read, physics_step};
use crate::state::EntityStore;
use crate::tab_page::entities::EntitiesPanel;
use client::ApiClient;
use ui::ScrollSpyNav;

#[component]
pub fn NetworkPanel() -> Element {
    let store = use_context::<EntityStore>();
    // 真实数据三态:挂载时拉 /api/group + /api/channel + /api/models。
    // 期间画布渲染 skeleton;失败保留错误摘要渲染柔和红边错误条;
    // 成功且三组全空 → 占位「暂无调度数据」。
    let mut net_state = use_signal(|| None::<NetworkResult>);
    // 节点初始边:先用 store 快照算(启动布局用),拉取成功后切真实数据。
    let view_seed = GraphView::from_store(&store);
    let seed_edges = edges_of(&view_seed);
    let mut edges = use_signal(|| seed_edges.iter().copied().collect::<HashSet<_>>());
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
    // 抽屉当前页别：HUD 按钮可直接切到设置/导入。
    let mut drawer_tab = use_signal(|| DrawerTab::Node);
    // 边的撤销栈：一次手势一条记录（多选连多根 → 一次 Ctrl+Z 全撤）。
    // true=本次操作是新增，false=删除；Vec 存当次涉及的全部边
    let mut history = use_signal(Vec::<(bool, Vec<(NodeKey, NodeKey)>)>::new);
    // Group-move anchors: (node, world offset from cursor) for the whole selection.
    let mut moving = use_signal(Vec::<(NodeKey, f64, f64)>::new);
    // Marquee rect in viewBox coords while a Select drag is active.
    let mut marquee = use_signal(|| None::<((f64, f64), (f64, f64))>);
    // Per-edge dodge curve factor, eased by the ticker (0 = straight).
    let mut dodge = use_signal(HashMap::<(NodeKey, NodeKey), f64>::new);
    let mut positions = use_signal(HashMap::<NodeKey, (f64, f64)>::new);
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
        spawn(async move {
            let mut ev = document::eval(
                r#"
                if (!window.__topoUndoBound) {
                    window.__topoUndoBound = true;
                    window.addEventListener('keydown', (e) => {
                        if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== 'z') return;
                        const t = e.target;
                        if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return;
                        e.preventDefault();
                        dioxus.send(true);
                    });
                }
            "#,
            );
            // undo 走 bool（true=执行一次撤销）
            // 注意：不能在这里顺便收数字，单一 eval 流只保一种类型最稳
            while let Ok(true) = ev.recv::<bool>().await {
                if let Some((added, pairs)) = history.write().pop() {
                    let mut ew = edges.write();
                    for pair in pairs {
                        if added {
                            ew.remove(&pair);
                        } else {
                            ew.insert(pair);
                        }
                    }
                }
            }
        });
        // Deterministic startup layout before the ticker takes over.
        {
            let view0 = GraphView::from_store(&store);
            let p = initial_positions(&view0);
            // initial_positions already centers each wrapped row; relaxing all
            // nodes as one logical layer would undo the stacked layout.
            *positions.write() = p;
        }
        // 真实数据拉取:成功后把初始边换成数据驱动的边,并唤醒物理 tick。
        spawn(async move {
            let client = ApiClient::shared().clone();
            let res = load_network_data(&client).await;
            match res {
                Ok(view) => {
                    let new_edges = edges_of(&view).into_iter().collect::<HashSet<_>>();
                    let p = initial_positions(&view);
                    edges.set(new_edges);
                    *positions.write() = p;
                    *net_state.write() = Some(Ok(view));
                    let next = *wake.peek() + 1;
                    *wake.write() = next;
                }
                Err(msg) => {
                    *net_state.write() = Some(Err(msg));
                }
            }
        });
        // 写路径成功后的画布刷新：drawer 写函数（drawer_write）成功后调
        // bump_topo_refresh() 递增 TOPO_REFRESH；这里 400ms 轮询版本号，
        // 变化即重拉三端点并重建 edges / positions（与挂载时同一条链路）。
        // #183 起画布不再由 store 行驱动，必须重拉才反映写结果。
        {
            let last = topo_refresh_version();
            spawn(async move {
                let mut seen = last;
                loop {
                    gloo_timers::future::TimeoutFuture::new(400).await;
                    let v = topo_refresh_version();
                    if v == seen {
                        continue;
                    }
                    seen = v;
                    let client = ApiClient::shared().clone();
                    if let Ok(view) = load_network_data(&client).await {
                        let new_edges = edges_of(&view).into_iter().collect::<HashSet<_>>();
                        let p = initial_positions(&view);
                        edges.set(new_edges);
                        *positions.write() = p;
                        *net_state.write() = Some(Ok(view));
                        let next = *wake.peek() + 1;
                        *wake.write() = next;
                    }
                }
            });
        }
        spawn(async move {
            let mut velocities = HashMap::<NodeKey, (f64, f64)>::new();
            let mut seen_wake = *wake.peek();
            let mut energy = 0.0;
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
                        // 真实时间驱动：progress = (now - start) / duration，
                        // 帧间隔抖动不会反映到节奏上。
                        let raw = ((now_ms() - t.started_ms) / t.duration_ms).clamp(0.0, 1.0);
                        t.t = raw;
                        {
                            let mut pw = positions.write();
                            for (k, to) in t.to.iter() {
                                let from = t.from.get(k).copied().unwrap_or(*to);
                                // 错峰：这个节点先静下来再动，序列感
                                let d = t.delay.get(k).copied().unwrap_or(0.0);
                                let local = ((raw - d) / (1.0 - d)).clamp(0.0, 1.0);
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
                if focus_space.peek().is_some() {
                    // 焦点态下物理完全停摆，拖拽也别回滚——
                    // positions 只在拖拽处理时手动改，ticker 不跑力学。
                    energy = 0.0;
                    dodge_active = false;
                    continue;
                }
                // 事实源优先级:真实数据拉取成功 → 用拉取快照;否则回退本地 store。
                let view_now = match &*net_state.peek() {
                    Some(Ok(v)) => v.clone(),
                    _ => GraphView::from_store(&store),
                };
                let layers = visible_layers_of(&view_now);
                let pairs = display_edge_pairs(&edges.peek());
                let pairs_xy: Vec<(NodeKey, NodeKey)> =
                    pairs.iter().map(|&(u, l, _)| (u, l)).collect();
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
                        // 悬空边（端点不在任何层→无 position）跳过, 避免索引 panic。
                        let (Some(&pu), Some(&pl)) = (ps.get(&u), ps.get(&l)) else {
                            continue;
                        };
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
    // 退出时才需要它们的位置。事实源优先级同 ticker:真实数据 > store。
    let view_now = match net_state() {
        Some(Ok(ref v)) => v.clone(),
        _ => GraphView::from_store(&store),
    };
    let mut layers = visible_layers_of(&view_now);
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
                    if let Some(m) = next
                        && seen.insert(m)
                    {
                        queue.push_back(m);
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
        if let (Some(Drag::Wire { src }), Some(t)) = (drag_now, hover_now)
            && let Some(pair) = normalize(src, t)
            && !edges_read(&edges).contains(&pair)
        {
            return anchor(t, t.layer() == 0 || pair.0 == t);
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
                let view_m = match &*net_state.peek() {
                    Some(Ok(v)) => v.clone(),
                    _ => GraphView::from_store(&store),
                };
                let layers = visible_layers_of(&view_m);
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
        _ if cone_now.is_some() => SEC_HINT_FOCUS,
        Some(Drag::Wire { .. }) => SEC_HINT_WIRING,
        Some(Drag::Move { .. }) => SEC_HINT_MOVING,
        _ => SEC_HINT_IDLE,
    };

    let hint_right = if drawer_tab() != DrawerTab::Node || inspect().is_some() {
        332
    } else {
        12
    };
    rsx! {
        div { class: "flex h-full min-h-[480px] flex-col",
            // 画布与抽屉同层：抽屉 absolute 覆盖右侧，画布尺寸恒定，
            // 开合不引起任何重排。图例与按钮都做 HUD 浮在画布上。
            div { class: "relative min-h-0 flex-1",
                // 左上：分组图例（浮层）
                div { class: "pointer-events-none absolute left-3 top-3 z-10 flex flex-wrap items-center gap-2",
                    for (i, g) in view_now.groups.iter().enumerate() {
                        span { class: "pointer-events-auto inline-flex items-center gap-1.5 rounded-full border border-border bg-card/85 px-2.5 py-1 {ui::TYPE_DESC} backdrop-blur",
                            span { class: "h-2 w-2 rounded-full", style: "background: {GROUP_PALETTE[i % GROUP_PALETTE.len()]}" }
                            "{g}"
                        }
                    }
                }
                // 右上：设置/导入/适配；抽屉开着时向右让出 320px
                {
                    let hud_right = if drawer_tab() != DrawerTab::Node || inspect().is_some() {
                        332
                    } else {
                        12
                    };
                    rsx! {
                        div {
                            class: "absolute top-3 z-10 flex items-center gap-1.5",
                            style: "right: {hud_right}px",
                    button {
                        class: "rounded-md border border-border bg-card/85 px-2.5 py-1 {ui::TYPE_DESC} backdrop-blur hover:border-border hover:text-foreground",
                        onclick: move |_| drawer_tab.set(DrawerTab::Settings),
                        {BTN_SETTINGS}
                    }
                    button {
                        class: "rounded-md border border-border bg-card/85 px-2.5 py-1 {ui::TYPE_DESC} backdrop-blur hover:border-border hover:text-foreground",
                        onclick: move |_| drawer_tab.set(DrawerTab::Import),
                        {BTN_IMPORT}
                    }
                    button {
                        class: "rounded-md border border-border bg-card/85 px-2.5 py-1 {ui::TYPE_DESC} backdrop-blur hover:border-border hover:text-foreground",
                        onclick: move |_| {
                            let pts: Vec<(f64, f64)> = layers_fit
                                .iter()
                                .flatten()
                                .filter_map(|k| positions.peek().get(k).copied())
                                .collect();
                            if pts.is_empty() {
                                return;
                            }
                            let in_focus = focus_space.peek().is_some();
                            let ((px, py), z) = if in_focus {
                                fit_view_into(&pts, *rect.peek(), true)
                            } else {
                                fit_view(&pts)
                            };
                            pan.set((px, py));
                            zoom.set(z);
                        },
                        {BTN_FIT}
                    }
                }
                    }
                }
                // 右下：操作提示；抽屉开着时同样向右让
                span {
                    class: "pointer-events-none absolute bottom-3 z-10 {ui::TYPE_LABEL}",
                    style: "right: {hint_right}px",
                    "{hint}"
                }
                // 三态浮层:加载中 skeleton / 错误柔和红边 / 全空占位。
                // 提取为局部 let 避免在 rsx! 里嵌 match(各臂类型不一致)。
                {
                    let net_overlay = match &net_state() {
                        None => rsx! {
                            div { class: "absolute inset-0 z-20 flex items-center justify-center",
                                "data-testid": "net-loading",
                                "role": "status",
                                "aria-label": MSG_LOADING_ARIA,
                                div { class: "flex flex-col items-center gap-2",
                                    div {
                                        class: "h-4 w-40 animate-pulse rounded-full bg-secondary",
                                    }
                                    div {
                                        class: "h-4 w-24 animate-pulse rounded-full bg-secondary/70",
                                    }
                                    p { class: "{ui::TYPE_LABEL}", {MSG_LOADING} }
                                }
                            }
                        },
                        Some(Err(msg)) => {
                            let m = msg.clone();
                            rsx! {
                                div {
                                    class: "absolute inset-x-4 top-4 z-20 flex items-center gap-2 rounded-lg border border-destructive bg-destructive px-3 py-2",
                                    "data-testid": "net-error",
                                    "role": "alert",
                                    p { class: "text-[11px] {ui::C_DANGER}", "{m}" }
                                }
                            }
                        }
                        Some(Ok(view)) if view.groups.is_empty()
                            && view.aliases.is_empty()
                            && view.channels.is_empty() =>
                        {
                            rsx! {
                                div { class: "absolute inset-0 z-20 flex items-center justify-center",
                                    "data-testid": "net-empty",
                                    "role": "status",
                                    "aria-label": MSG_EMPTY,
                                    p { class: "{ui::TYPE_DESC}", {MSG_EMPTY} }
                                }
                            }
                        }
                        _ => {
                            rsx! { Fragment {} }
                        }
                    };
                    {net_overlay}
                }
                div {
                    class: match &net_state() {
                        Some(Err(_)) => "h-full overflow-hidden rounded-xl border border-destructive bg-background",
                        _ => "h-full overflow-hidden rounded-xl border border-border bg-background",
                    },
                svg {                    view_box: "0 0 {VIEW_W:.0} {VIEW_H:.0}",
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
                    // 右键空白：退出焦点，还原全图
                    oncontextmenu: move |e| {
                        e.prevent_default();
                        if focus_space.peek().is_none() {
                            return;
                        }
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
                    },
                    onmouseup: move |_| {
                        let current = *drag.peek();
                        match current {
                            Some(Drag::Pan { moved: false, .. }) => {
                                // 左键点空白：只清选择；抽屉与焦点都保留。
                                // 退出焦点走右键（oncontextmenu）或再点同节点。
                                selected.set(HashSet::new());
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
                    for (label, y) in [(LBL_GROUP, ROW_Y[0]), (LBL_ALIAS, ROW_Y[1]), (LBL_DISPATCH, ROW_Y[2])] {
                        text {
                            class: "select-none",
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
                                let wire_color = view_color(&view_now, du);
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
                                        // 左键留给框选/拖拽；右键才删线，防误点
                                        oncontextmenu: move |e| {
                                            e.prevent_default();
                                            e.stop_propagation();
                                            if edges.write().remove(&raw) {
                                                history.write().push((false, vec![raw]));
                                            }
                                        },
                                        title { {MSG_WIRE_DELETE_HINT} }
                                    }
                                }
                            }
                        }

                        // ---- Dangling wire ----
                        if let Some(Drag::Wire { src }) = drag_now {
                            path {
                                d: "{bezier(anchor(src, true), temp_end)}",
                                fill: "none",
                                stroke: "{view_color(&view_now, src)}",
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
                                    let node_color = view_color(&view_now, key);
                                    let title_text = view_title(&view_now, key);
                                    let sub_text = view_subtitle(&view_now, key);
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
                                                        // 收集这次 gesture 实际插入的边，一次入栈
                                                        let mut batch: Vec<(NodeKey, NodeKey)> = Vec::new();
                                                        {
                                                            let mut ew = edges.write();
                                                            for s in sources {
                                                                if let Some(pair) = normalize(s, key)
                                                                    && ew.insert(pair) {
                                                                        batch.push(pair);
                                                                    }
                                                            }
                                                        }
                                                        if !batch.is_empty() {
                                                            history.write().push((true, batch));
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
                                                                drawer_tab.set(DrawerTab::Node);
                                                            } else {
                                                                // 进入焦点空间：算连通锥 → 一次性排布 → 补间
                                                                inspect.set(Some(key));
                                                                let all = edges_read(&edges);
                                                                let ev: Vec<(NodeKey, NodeKey)> =
                                                                    all.iter().copied().collect();
                                                                let cone = focus_cone(key, &ev);
                                                                let cur = positions.peek().clone();
                                                                let target =
                                                                    layout_subgraph(&cone, &ev, &cur);
                                                                let pts: Vec<(f64, f64)> =
                                                                    target.values().copied().collect();
                                                                let (to_pan, to_zoom) =
                                                                    fit_view_into(&pts, *rect.peek(), true);
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
                                                class: "select-none",
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
                                                    class: "select-none",
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
                // 设置抽屉:页签条(DrawerTabs)+ 实体面板(EntitiesPanel)+ 滚动钉(ScrollSpyNav)。
                // 拉取失败时 store 是旧/空快照,顶部红条避免对着错误数据编辑。
                if drawer_tab() == DrawerTab::Settings {
                    aside { class: "absolute inset-y-0 right-0 z-20 flex w-full flex-col border-l border-border bg-card/97 backdrop-blur sm:w-[320px]",
                        DrawerTabs {
                            active: DrawerTab::Settings,
                            on_tab: move |t: DrawerTab| drawer_tab.set(t),
                        }
                        div { class: "relative min-h-0 flex-1",
                            // 数据供给:拉取失败时实体卡片读到的 store 是旧/空快照,
                            // 顶部提示避免对着错误数据编辑。
                            if matches!(&net_state(), Some(Err(_))) {
                                div {
                                    class: "px-4 py-2",
                                    "data-testid": "ent-blocked",
                                    p { class: "text-[11px] {ui::C_DANGER}", {SEC_SETTINGS_STALE} }
                                }
                            }
                            // 导航钉在抽屉上，不随内容滚动
                            ScrollSpyNav {
                                container: "ent-scroll",
                                items: vec![
                                    ({LBL_GROUP}.to_string(), "ent-card-0".to_string()),
                                    ({LBL_ALIAS}.to_string(), "ent-card-1".to_string()),
                                    ({LBL_CHANNELS}.to_string(), "ent-card-2".to_string()),
                                ],
                            }
                            div {
                                id: "ent-scroll",
                                class: "h-full overflow-y-auto scroll-hidden p-3 pl-8",
                                EntitiesPanel {}
                            }
                        }
                    }
                // 导入抽屉:头部(DrawerHeader 复用页签条)+ ImportPanel(JSON 批量导入渠道)。
                } else if drawer_tab() == DrawerTab::Import {
                    aside { class: "absolute inset-y-0 right-0 z-20 flex w-full flex-col border-l border-border bg-card/97 backdrop-blur sm:w-[320px]",
                        DrawerHeader {
                            tab: drawer_tab(),
                            title: BTN_IMPORT.to_string(),
                            subtitle: LBL_IMPORT_SUBTITLE.to_string(),
                            on_tab: move |t: DrawerTab| drawer_tab.set(t),
                            on_close: move |_| { drawer_tab.set(DrawerTab::Node); inspect.set(None) },
                        }
                        div { class: "min-h-0 flex-1 p-3",
                            ImportPanel {}
                        }
                    }
                // 节点检视抽屉:点节点进入;按节点类型分流 Group/Alias/Dispatch 三种检视体。
                // 关闭时还原 saved_positions(进出焦点的位移动画走 tween)。
                } else if let Some(node) = inspect() {
                    NodeInspector {
                        node: node,
                        on_tab: move |t: DrawerTab| drawer_tab.set(t),
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
