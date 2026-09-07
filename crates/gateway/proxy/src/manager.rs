//! `manager` —— 按 channel 租约选出口 Client + 进程内健康
//!
//! 对齐 grok2api-sing 的 `Acquire` / `Feedback` / per-node client 缓存。
//! HTTP CONNECT / SOCKS5 握手仍交给 reqwest（`socks` feature），本模块不实现
//! vless/vmess/ss/trojan。无节点或全部冷却时退回直连 Client（`node_id = 0`）。
//!
//! 协议实现落地前，`acquire` 会跳过 Vless/Vmess/Shadowsocks/Trojan 节点（视同冷却），
//! 回落直连。PR2/3 填入 proto::ProxyConnector 实现后自动生效。
//! ponytail: 健康表只活在进程内；affinity / DB 持久化等需要时再加。
use super::node::{ProxyNode, ProxyScheme};
use super::pool::{ProxyPool, ProxySnapshot};
use crate::proto::address::NetLocation;
use crate::proto::proxy_connector::ProxyConnector;
use crate::proto::shadowsocks::ShadowsocksCipher;
use rand::seq::SliceRandom;
use reqwest::redirect::Policy as RedirectPolicy;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::warn as tracing_warn;

fn is_supported_scheme(scheme: ProxyScheme) -> bool {
    matches!(
        scheme,
        ProxyScheme::Direct
            | ProxyScheme::Http
            | ProxyScheme::Socks5
            | ProxyScheme::Shadowsocks
            | ProxyScheme::Trojan
    )
}

/// 出口形态：reqwest 原生（direct/http/socks5）或协议连接器（ss/trojan）。
///
/// forward 层当前只消费 [`ProxyClient::Reqwest`]；Connector 变体供后续
/// egress 桥接（握手建流 → hyper connector），本 PR 先保证类型正确与
/// reqwest 路径零回归。
#[derive(Clone)]
pub enum ProxyClient {
    /// reqwest 原生路径（Direct/Http/Socks5）。
    Reqwest(Arc<reqwest::Client>),
    /// 协议连接器路径（Shadowsocks/Trojan），拨号时由连接器完成握手。
    Connector(Arc<dyn ProxyConnector>),
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
    /// Connector 变体返回 `None`——协议握手不走 reqwest，forward 层接入
    /// connector 拨号属下一链路 PR。
    pub fn reqwest_client(&self) -> Option<Arc<reqwest::Client>> {
        match self.client.as_ref() {
            ProxyClient::Reqwest(c) => Some(Arc::clone(c)),
            ProxyClient::Connector(_) => None,
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

/// 渠道出口管理器：选节点、缓存 Client、按反馈冷却。
pub struct ProxyManager {
    pool: ProxyPool,
    direct_client: Arc<reqwest::Client>,
    #[allow(clippy::type_complexity)]
    connector_cache: Mutex<HashMap<(i64, u64), Arc<dyn ProxyConnector>>>,
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
            // 协议连接器路径（SS/Trojan）：构造并缓存连接器实例
            ProxyScheme::Shadowsocks | ProxyScheme::Trojan => {
                let fingerprint = fingerprint_of(node);
                {
                    let cache = self
                        .connector_cache
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    if let Some(conn) = cache.get(&(node.id, fingerprint)) {
                        return Arc::new(ProxyClient::Connector(Arc::clone(conn)));
                    }
                }
                // ss:// URL 语义：auth.user = cipher 方法名，auth.pass = 密码
                // （parse_url 从 ss://method:password@host 提取）。缺省 aes-128-gcm + 空密码
                // 只在节点配置不完整时发生，交由握手期报错而非此处 panic。
                let conn: Arc<dyn ProxyConnector> = match node.scheme {
                    ProxyScheme::Shadowsocks => {
                        let (cipher_str, password) = match node.auth.as_ref() {
                            Some(a) if !a.user.is_empty() => (a.user.as_str(), a.pass.as_str()),
                            _ => ("aes-128-gcm", ""),
                        };
                        let cipher = ShadowsocksCipher::try_from(cipher_str).unwrap_or_else(|_| {
                            // 非法 cipher 名回落 AES-128-GCM（panic 不该出现在节点配置错误路径）
                            tracing_warn!(cipher = %cipher_str, "非法 SS cipher，回落 aes-128-gcm");
                            ShadowsocksCipher::try_from("aes-128-gcm")
                                .expect("aes-128-gcm is a valid cipher")
                        });
                        let location = NetLocation::new(
                            crate::proto::Address::Hostname(node.host.clone()),
                            node.port,
                        );
                        Arc::new(crate::proto::ShadowsocksProxyConnector::new_client(
                            location, cipher, password, false,
                        ))
                    }
                    ProxyScheme::Trojan => {
                        let password = node
                            .auth
                            .as_ref()
                            .map(|a| a.pass.as_str())
                            .unwrap_or_default();
                        let location = NetLocation::new(
                            crate::proto::Address::Hostname(node.host.clone()),
                            node.port,
                        );
                        Arc::new(crate::proto::TrojanProxyConnector::new(location, password))
                    }
                    _ => unreachable!("guarded by match above"),
                };
                let mut cache = self
                    .connector_cache
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                cache.insert((node.id, fingerprint), Arc::clone(&conn));
                Arc::new(ProxyClient::Connector(conn))
            }
            // 未实现协议（Vless/Vmess）：is_supported_scheme 已挡住，防御性直连
            ProxyScheme::Vless | ProxyScheme::Vmess => {
                Arc::new(ProxyClient::Reqwest(Arc::clone(&self.direct_client)))
            }
        }
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
