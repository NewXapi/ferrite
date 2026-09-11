//! `manager` —— 按 channel 租约选出口 Client + 进程内健康
//!
//! 对齐 grok2api-sing 的 `Acquire` / `Feedback` / per-node client 缓存。
//! HTTP CONNECT / SOCKS5 握手仍交给 reqwest（`socks` feature），本模块不实现
//! vless/vmess/ss/trojan。无节点或全部冷却时退回直连 Client（`node_id = 0`）。
//!
//! 协议节点（Vless/Vmess/Shadowsocks/Trojan）由 `crate::adapter::adapter_for`
//! 映射成 meow `ProxyAdapter`（缓存于 connector_cache）；映射失败回落直连。
//! ponytail: 健康表只活在进程内；affinity / DB 持久化等需要时再加。
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
        }
    }

    /// 全量替换节点快照。健康 / inflight / client 缓存不随快照清空。
    pub fn install(&self, snap: ProxySnapshot) {
        self.pool.install(snap);
    }

    /// 为 `channel_key` 租一个出口。
    ///
    /// 无节点或全部冷却 → `node_id = 0` 的直连 Client。
    /// 否则：跳过冷却 → 跳过不支持的协议 (Vless/Vmess/Shadowsocks/Trojan) → 最高 priority 层 → 层内 least-inflight → 并列随机。
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

        let mut rng = rand::thread_rng();
        let selected = eligible
            .choose(&mut rng)
            .cloned()
            .expect("eligible non-empty");
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

    /// 并发探测当前快照全部节点。
    ///
    /// 默认关闭（原因见 rustdoc）：主动探测会给机场带流量。只有当用户显式启用时才调用。
    /// `target` 是探测目标 host:port（缺省建议渠道 base_url 的 host，实现时定）。
    /// `timeout` 单位秒，默认 5s（实现时可设）。并发度由实现者决定（建议 3-5）。
    ///
    /// **Note**：probe_all 不修改 nodes 状态；探测失败写回 adapter.health().record_delay()
    /// 供 `node_stats` 展示 last_delay。
    pub async fn probe_all(&self, target: &str, timeout: Duration) -> Vec<ProbeResult> {
        let _ = (target, timeout);
        todo!("TODO(#111): 并发探测当前快照全部节点")
    }

    /// 节点运行时状态视图：把进程内私有状态导出给管理面。
    ///
    /// `inflight` / `health` 是私有字段，管理台此前看不到任何运行时信息。
    /// 这是 `GET /api/proxy_nodes/report` 的数据源。
    pub fn node_stats(&self) -> Vec<NodeStats> {
        todo!("TODO(#111): 导出进程内节点状态")
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
