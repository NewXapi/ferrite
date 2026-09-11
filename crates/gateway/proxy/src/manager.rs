//! `manager` —— 按 channel 租约选出口 Client + 进程内健康
//!
//! 对齐 grok2api-sing 的 `Acquire` / `Feedback` / per-node client 缓存。
//! HTTP CONNECT / SOCKS5 握手仍交给 reqwest（`socks` feature），本模块不实现
//! vless/vmess/ss/trojan。无节点或全部冷却时退回直连 Client（`node_id = 0`）。
//!
//! 协议节点（Vless/Vmess/Shadowsocks/Trojan）由 `crate::adapter::adapter_for`
//! 映射成 meow `ProxyAdapter`（缓存于 connector_cache）；映射失败回落直连。
//! ponytail: 健康表与软亲和只活在进程内；DB 持久化等需要时再加。
use super::node::{ProxyNode, ProxyScheme};
use super::pool::{ProxyPool, ProxySnapshot};
use super::probe::ProbeResult;
use meow_common::ProxyAdapter;
use rand::seq::SliceRandom;
use reqwest::redirect::Policy as RedirectPolicy;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 软亲和 TTL：渠道的亲和记录距上次使用超过此时长即视为过期（换机窗口），
/// 下次并列时重新随机打散。持续有流量时时间戳随每次选中刷新，活跃节点
/// 不会因 TTL 过期；流量停摆 >10s（连接大概率已闲置关闭）才允许换机。
/// ponytail: 固定值；要可配再加 options 项。
const AFFINITY_TTL: Duration = Duration::from_secs(10);

fn is_supported_scheme(_scheme: ProxyScheme) -> bool {
    // 所有协议已实现（PR3/4 落地 VLESS/VMess + WS 客户端链），临时闸门移除
    true
}

/// 出口形态：reqwest 原生（direct/http/socks5）或协议适配器（meow）。
///
/// `Reqwest` 变体由 forward 直接当 `reqwest::Client` 用；`Adapter` 变体
/// 经 `forward::adapter_egress` 桥成 hyper connector——协议握手发生在
/// `dial_tcp`（meow），桥负责在其上叠加 TLS 并承载 HTTP。
#[derive(Clone)]
pub enum ProxyClient {
    /// reqwest 原生路径（Direct/Http/Socks5）。
    Reqwest(Arc<reqwest::Client>),
    /// 协议适配器路径（SS/Trojan/VLESS/VMess，meow 实现）。
    Adapter(Arc<dyn ProxyAdapter>),
}

/// 一次上游尝试占用的出口 Client。
///
/// `Drop` 时把对应节点的 inflight 减一。`node_id == 0` 表示直连，无释放动作。
pub struct Lease {
    /// 选中的节点 id；`0` = 直连（无代理节点或全部冷却）。
    pub node_id: i64,
    /// 出口形态。
    pub client: Arc<ProxyClient>,
    release: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Lease {
    /// 返回 reqwest Client（仅 [`ProxyClient::Reqwest`] 变体）。
    ///
    /// `Adapter` 变体返回 `None`——协议出口走 [`Self::adapter`]。
    pub fn reqwest_client(&self) -> Option<Arc<reqwest::Client>> {
        match self.client.as_ref() {
            ProxyClient::Reqwest(c) => Some(Arc::clone(c)),
            ProxyClient::Adapter(_) => None,
        }
    }

    /// 返回协议适配器（仅 [`ProxyClient::Adapter`] 变体）。
    pub fn adapter(&self) -> Option<Arc<dyn ProxyAdapter>> {
        match self.client.as_ref() {
            ProxyClient::Adapter(a) => Some(Arc::clone(a)),
            ProxyClient::Reqwest(_) => None,
        }
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            release();
        }
    }
}

#[derive(Default)]
struct NodeHealth {
    failure_count: u32,
    cooldown_until: Option<Instant>,
}

/// 节点运行时状态视图：把进程内私有状态导出给管理面。
///
/// `inflight` / `health` 是私有字段，管理台此前看不到任何运行时信息。
/// 这是 `GET /api/proxy_nodes/report` 的数据源。
pub struct NodeStats {
    pub node_id: i64,
    pub inflight: u32,
    pub failure_count: u32,
    /// 冷却剩余秒数；未冷却为 0
    pub cooldown_remaining_secs: u64,
    /// 最近一次探测延迟（来自 adapter 的 ProxyHealth），无记录为 None
    pub last_delay_ms: Option<u16>,
}

/// 渠道出口管理器：选节点、缓存 Client、按反馈冷却。
pub struct ProxyManager {
    pool: ProxyPool,
    direct_client: Arc<reqwest::Client>,
    #[allow(clippy::type_complexity)]
    connector_cache: Mutex<HashMap<(i64, u64), Arc<dyn ProxyAdapter>>>,
    inflight: Arc<Mutex<HashMap<i64, u32>>>,
    client_cache: Mutex<HashMap<(i64, u64), Arc<reqwest::Client>>>,
    health: Mutex<HashMap<i64, NodeHealth>>,
    /// 软亲和：channel_key → (上次选中的节点 id, 上次选中时间)。
    /// 仅在并列最闲组 >1 时参与决策；指针始终指向本渠道最近实际承接流量的节点。
    affinity: Mutex<HashMap<String, (i64, Instant)>>,
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyManager {
    /// 空池 + 共享直连 Client（禁重定向、connect 5s、每 host 8 idle）。
    pub fn new() -> Self {
        Self {
            pool: ProxyPool::new(),
            direct_client: Arc::new(build_client(None)),
            inflight: Arc::new(Mutex::new(HashMap::new())),
            client_cache: Mutex::new(HashMap::new()),
            connector_cache: Mutex::new(HashMap::new()),
            health: Mutex::new(HashMap::new()),
            affinity: Mutex::new(HashMap::new()),
        }
    }

    /// 全量替换节点快照。健康 / inflight / client 缓存不随快照清空。
    pub fn install(&self, snap: ProxySnapshot) {
        self.pool.install(snap);
    }

    /// 为 `channel_key` 租一个出口。
    ///
    /// 无节点或全部冷却 → `node_id = 0` 的直连 Client。
    /// 否则：跳过冷却 → 跳过不支持的协议 (Vless/Vmess/Shadowsocks/Trojan) → 最高 priority 层 → 层内 least-inflight → 并列时软亲和（TTL 内粘住上次节点，过期或落选则随机 rebalance）→ 单选直通。
    ///
    /// 协议实现落地前的临时闸门（PR2/3 移除）：未实现的协议节点视同冷却，强制回落直连。
    pub fn acquire(&self, channel_key: &str) -> Lease {
        let now = Instant::now();
        let candidates = self.pool.candidates(channel_key);
        let health = self.health.lock().unwrap_or_else(|e| e.into_inner());
        let inflight = self.inflight.lock().unwrap_or_else(|e| e.into_inner());

        let mut eligible: Vec<Arc<ProxyNode>> = candidates
            .into_iter()
            .filter(|node| {
                is_supported_scheme(node.scheme)
                    && true
                    && health
                        .get(&node.id)
                        .and_then(|h| h.cooldown_until)
                        .is_none_or(|until| until <= now)
            })
            .collect();
        drop(health);

        if eligible.is_empty() {
            // 无节点 / 全冷却 / 全是未实现协议 → 直连。
            return self.direct_lease();
        }

        let max_priority = eligible.iter().map(|n| n.priority).max().unwrap_or(0);
        eligible.retain(|n| n.priority == max_priority);

        let min_inflight = eligible
            .iter()
            .map(|n| inflight.get(&n.id).copied().unwrap_or(0))
            .min()
            .unwrap_or(0);
        eligible.retain(|n| inflight.get(&n.id).copied().unwrap_or(0) == min_inflight);
        drop(inflight);

        // 软亲和选点：并列最闲候选 >1 时优先粘住 TTL 内的亲和节点，
        // 否则组内随机并写回指针；单选直通。亲和节点负载拉开后落出并列组
        // 即自动 rebalance，无需额外代码。
        // 锁序：health / inflight 已在此前释放；affinity 锁只覆盖本决策块，
        // 不持锁跨候选构建，也不与后续 inflight 自增嵌套。
        let selected = {
            let mut affinity = self.affinity.lock().unwrap_or_else(|e| e.into_inner());
            let chosen = if eligible.len() > 1 {
                let sticky = affinity
                    .get(channel_key)
                    .filter(|(_, last_used)| last_used.elapsed() <= AFFINITY_TTL)
                    .map(|(id, _)| *id)
                    .filter(|id| eligible.iter().any(|n| n.id == *id));
                let chosen = match sticky {
                    // 亲和未过期且仍在最闲并列组内 → 粘住它，不打随机
                    Some(id) => eligible
                        .iter()
                        .find(|n| n.id == id)
                        .cloned()
                        .expect("sticky id present in eligible"),
                    // 首次 / 已过期 / 亲和节点落选 → 并列组内随机（rebalance）
                    None => eligible
                        .choose(&mut rand::thread_rng())
                        .cloned()
                        .expect("eligible non-empty"),
                };
                affinity.insert(channel_key.to_string(), (chosen.id, now));
                chosen
            } else {
                eligible[0].clone()
            };
            chosen
        };
        let node_id = selected.id;

        {
            let mut inflight = self.inflight.lock().unwrap_or_else(|e| e.into_inner());
            *inflight.entry(node_id).or_insert(0) += 1;
        }

        let inflight = Arc::clone(&self.inflight);
        let release: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let mut map = inflight.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(count) = map.get_mut(&node_id) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    map.remove(&node_id);
                }
            }
        });

        let client = self.client_for(&selected);
        Lease {
            node_id,
            client,
            release: Some(release),
        }
    }

    /// 按这次尝试的 HTTP 状态 / 是否传输失败更新节点健康。
    ///
    /// - `node_id == 0`：直连，忽略。
    /// - `401` / `429`：账号问题，不冷却代理。
    /// - 成功（无传输错误且 2xx/3xx）：清冷却、失败计数归零。
    /// - 否则：失败计数 +1，冷却 `min(10min, 30s << min(n-1, 4))`。
    pub fn feedback(&self, node_id: i64, status: u16, transport_err: bool) {
        if node_id == 0 || status == 401 || status == 429 {
            return;
        }
        let mut health = self.health.lock().unwrap_or_else(|e| e.into_inner());
        let node = health.entry(node_id).or_default();
        if !transport_err && (200..400).contains(&status) {
            node.failure_count = 0;
            node.cooldown_until = None;
            return;
        }
        node.failure_count = node.failure_count.saturating_add(1);
        let shift = (node.failure_count - 1).min(4);
        let secs = (30u64 << shift).min(10 * 60);
        node.cooldown_until = Some(Instant::now() + Duration::from_secs(secs));
    }

    fn direct_lease(&self) -> Lease {
        Lease {
            node_id: 0,
            client: Arc::new(ProxyClient::Reqwest(Arc::clone(&self.direct_client))),
            release: None,
        }
    }

    fn client_for(&self, node: &ProxyNode) -> Arc<ProxyClient> {
        match node.scheme {
            // reqwest 原生路径
            ProxyScheme::Direct | ProxyScheme::Http | ProxyScheme::Socks5 => {
                if node.scheme == ProxyScheme::Direct {
                    return Arc::new(ProxyClient::Reqwest(Arc::clone(&self.direct_client)));
                }
                let fingerprint = fingerprint_of(node);
                {
                    let cache = self.client_cache.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(client) = cache.get(&(node.id, fingerprint)) {
                        return Arc::new(ProxyClient::Reqwest(Arc::clone(client)));
                    }
                }
                let proxy = node.to_reqwest_proxy().ok().flatten();
                let client = Arc::new(build_client(proxy));
                let mut cache = self.client_cache.lock().unwrap_or_else(|e| e.into_inner());
                cache.insert((node.id, fingerprint), Arc::clone(&client));
                Arc::new(ProxyClient::Reqwest(client))
            }
            // 协议适配器路径（SS/Trojan/VLESS/VMess/Hysteria2/AnyTLS/Snell）：meow 适配器 + 缓存
            ProxyScheme::Shadowsocks
            | ProxyScheme::Trojan
            | ProxyScheme::Vless
            | ProxyScheme::Vmess
            | ProxyScheme::Hysteria2
            | ProxyScheme::AnyTls
            | ProxyScheme::Snell => {
                let fingerprint = fingerprint_of(node);
                {
                    let cache = self
                        .connector_cache
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    if let Some(adapter) = cache.get(&(node.id, fingerprint)) {
                        return Arc::new(ProxyClient::Adapter(Arc::clone(adapter)));
                    }
                }
                // 映射失败（认证缺失 / UUID 非法 / cipher 不识别）时 adapter_for
                // 返回 None 并已 warn。这里回落直连而非 502：配置错误的节点没有
                // 再试的价值，直接让流量走直连路径。
                let Some(adapter) = crate::adapter::adapter_for(node) else {
                    return Arc::new(ProxyClient::Reqwest(Arc::clone(&self.direct_client)));
                };
                let adapter: Arc<dyn ProxyAdapter> = adapter;
                let mut cache = self
                    .connector_cache
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                cache.insert((node.id, fingerprint), Arc::clone(&adapter));
                Arc::new(ProxyClient::Adapter(adapter))
            }
        }
    }

    /// 并发探测当前快照的全部节点（按 id 去重，绑多渠道的节点只探一次）。
    ///
    /// **调用方负责决定是否调用**：主动探测会给机场带真实流量，所以不在
    /// `install` 或 `acquire` 里自动触发，也不设内置定时器。
    ///
    /// 并发度固定 4：探测是运维动作而非请求路径，4 条并发足以让几十个节点在几秒内
    /// 走完，又不会一瞬间对同一机场开几十条连接（那本身像攻击流量）。
    ///
    /// 探测**不**改节点冷却状态：`feedback` 的冷却反映真实请求的成败，探测失败不该
    /// 把正在服务的节点踢下线（探测目标与真实上游可以不同）。成功的延迟写进
    /// adapter 自己的 `ProxyHealth`，由 [`Self::node_stats`] 读出。
    pub async fn probe_all(&self, target: &str, timeout: Duration) -> Vec<ProbeResult> {
        const CONCURRENCY: usize = 4;
        let nodes = self.pool.all_nodes();
        let mut results = Vec::with_capacity(nodes.len());
        for chunk in nodes.chunks(CONCURRENCY) {
            let probes = chunk
                .iter()
                .map(|node| crate::probe::probe_node(node, target, timeout));
            results.extend(futures_util::future::join_all(probes).await);
        }
        results
    }

    /// 节点运行时状态视图：把进程内私有状态导出给管理面。
    /// `inflight` / `health` 是私有字段，管理台此前看不到任何运行时信息。
    /// 这是 `GET /api/proxy_nodes/report` 的数据源。
    ///
    /// `last_delay_ms` 来自 adapter 的 `ProxyHealth`，因此只对**已装配过**的协议节点
    /// 有值：没探测过、或走 reqwest 的 http/socks5 节点都是 `None`。这是准确的——
    /// 没测过就是没数据，不要拿 0 冒充。
    pub fn node_stats(&self) -> Vec<NodeStats> {
        let now = Instant::now();
        let inflight = self.inflight.lock().unwrap_or_else(|e| e.into_inner());
        let health = self.health.lock().unwrap_or_else(|e| e.into_inner());
        let inflight_map: HashMap<i64, u32> = inflight.clone();
        let health_data: HashMap<i64, (u32, Option<Instant>)> = health
            .iter()
            .map(|(&id, h)| (id, (h.failure_count, h.cooldown_until)))
            .collect();
        drop(inflight);
        drop(health);

        let adapters = self
            .connector_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let delay_by_node: HashMap<i64, u16> = adapters
            .iter()
            .filter_map(|((node_id, _fp), adapter)| {
                let delay = adapter.health().last_delay();
                (delay > 0).then_some((*node_id, delay))
            })
            .collect();
        drop(adapters);

        self.pool
            .all_nodes()
            .iter()
            .map(|node| {
                let inflight = inflight_map.get(&node.id).copied().unwrap_or(0);
                let (failure_count, cooldown_until) =
                    health_data.get(&node.id).copied().unwrap_or((0, None));
                NodeStats {
                    node_id: node.id,
                    inflight,
                    failure_count,
                    cooldown_remaining_secs: cooldown_until
                        .map_or(0, |until| until.saturating_duration_since(now).as_secs()),
                    last_delay_ms: delay_by_node.get(&node.id).copied(),
                }
            })
            .collect()
    }
}

fn build_client(proxy: Option<reqwest::Proxy>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .redirect(RedirectPolicy::none())
        .connect_timeout(Duration::from_secs(5))
        .pool_max_idle_per_host(8);
    if let Some(proxy) = proxy {
        builder = builder.proxy(proxy);
    }
    builder.build().expect("reqwest client build")
}

fn fingerprint_of(node: &ProxyNode) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    node.scheme.hash(&mut hasher);
    node.host.hash(&mut hasher);
    node.port.hash(&mut hasher);
    if let Some(auth) = &node.auth {
        auth.user.hash(&mut hasher);
        auth.pass.hash(&mut hasher);
    }
    hasher.finish()
}
