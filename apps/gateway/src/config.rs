//! 网关配置 — 从 `config/config.toml` 加载。
//!
//! 配置结构在此，各 crate 的运行期类型（`HealthSetting` / `ConfigPriceTable` /
//! `ConfigRetryLoop`）由 `lib.rs` 的 `build_app` 组装，配置结构不反向依赖它们的语义。
//!
//! 单机模式的数据面来源也在此：`[[channels]]` 与 `[[keys]]` 经
//! [`build_route_snapshot`] / [`build_token_snapshot`] 变成运行期快照；
//! `[[proxy_nodes]]` 经 [`build_proxy_snapshot`] 注入 `ProxyManager`；
//! vless/vmess/ss/trojan 由 meow 适配器在拨号时完成协议握手。
use gateway_gate::snapshot::{TokenEntry, TokenSnapshot, UserSnapshot};

use contract::records::{
    ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta, TokenRecord, UserRecord,
};
use dispatch::Snapshot;

use metering::pricing::ModelPrice;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// 调度健康参数；字段名对齐 `dispatch::health::HealthSetting`。
#[derive(Debug, Deserialize, Clone)]
pub struct DispatchConfig {
    /// 连续失败达此数进入冷却。
    #[serde(default = "default_cooldown_threshold")]
    pub cooldown_threshold: u32,
    /// 冷却基础时长（秒）。
    #[serde(default = "default_cooldown_base_seconds")]
    pub cooldown_base_seconds: u64,
    /// 冷却最大时长（秒）；连续冷却时长递增到此上限。
    #[serde(default = "default_cooldown_max_seconds")]
    pub cooldown_max_seconds: u64,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            cooldown_threshold: default_cooldown_threshold(),
            cooldown_base_seconds: default_cooldown_base_seconds(),
            cooldown_max_seconds: default_cooldown_max_seconds(),
        }
    }
}

fn default_cooldown_threshold() -> u32 {
    5
}
fn default_cooldown_base_seconds() -> u64 {
    10
}
fn default_cooldown_max_seconds() -> u64 {
    60
}

/// 计量配置；`prices` 为空 = 不计费（本地单机默认）。
#[derive(Debug, Deserialize, Clone, Default)]
pub struct MeteringConfig {
    /// model 名 → 价格；键是对外模型名。
    #[serde(default)]
    pub prices: HashMap<String, ModelPrice>,
}

/// 重试策略。
///
/// 只有尝试预算：可重试的状态码由 `forward::egress::classify_status` 判定
/// （429 / 5xx 可重试，其余 4xx 致命），不在配置里开第二个判定口。
#[derive(Debug, Deserialize, Clone)]
pub struct RetryConfig {
    /// 单请求最大尝试次数（含首次）。
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
        }
    }
}

fn default_max_attempts() -> u32 {
    3
}

/// 一个上游渠道 —— 单机模式下 `ChannelRecord` 与其路由单元的配置来源。
///
/// `name` 同时是渠道的稳定标识：它成为 `ChannelRecord.meta.key`，也是
/// `RouteUnitRecord.channel_key` 的取值，dispatch 靠它把单元连回渠道。
/// 因此同一份配置里 `name` 必须唯一。
#[derive(Debug, Deserialize, Clone)]
pub struct ChannelConfig {
    /// 渠道稳定标识；同时用作 `meta.key` 与 `channel_key`。
    pub name: String,
    /// 协议族："openai" / "claude" / "gemini" / "passthrough"。
    /// 决定鉴权头与上游路径模板（见 `forward::adapter`）。
    #[serde(default = "default_provider_type")]
    pub provider_type: String,
    /// 上游基础地址，如 `https://api.example.com`。
    pub base_url: String,
    /// 上游凭据（明文）。配置文件已 gitignore，不要提交真实值。
    pub api_key: String,
    /// 该渠道对外提供的公开模型名；每个生成一条路由单元。
    #[serde(default)]
    pub models: Vec<String>,
    /// 公开名 → 上游真名的映射；未列出的模型上游名等于公开名。
    #[serde(default)]
    pub upstream_models: HashMap<String, String>,
    /// 同优先级内的加权轮询权重。
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// 优先级；数字越大越优先，dispatch 先按它分层。
    #[serde(default)]
    pub priority: i32,
    /// 渠道并发上限；0 = 不限。
    #[serde(default)]
    pub max_concurrency: u32,
}

impl ChannelConfig {
    /// 校验必填字段，避免非法配置到运行时才爆炸。
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("ChannelConfig.name 不能为空".into());
        }
        if self.base_url.is_empty() {
            return Err(format!(
                "ChannelConfig \"{}\" 的 base_url 不能为空",
                self.name
            ));
        }
        if self.api_key.is_empty() {
            return Err(format!(
                "ChannelConfig \"{}\" 的 api_key 不能为空",
                self.name
            ));
        }
        if !["openai", "claude", "gemini", "passthrough"].contains(&self.provider_type.as_str()) {
            return Err(format!(
                "ChannelConfig \"{}\" 的 provider_type 必须是 openai/claude/gemini/passthrough，当前值: {}",
                self.name, self.provider_type
            ));
        }
        Ok(())
    }
}

fn default_provider_type() -> String {
    "openai".to_string()
}

fn default_weight() -> u32 {
    1
}

/// 一把本地 API key —— 单机模式下 `TokenRecord` 的配置来源。
///
/// 明文写在配置里、启动时算 sha256 建索引；网关不持久化明文之外的形态。
#[derive(Debug, Deserialize, Clone)]
pub struct KeyConfig {
    /// 明文 key，客户端用它做 `Authorization: Bearer <key>`。
    pub key: String,
    /// 备注名，仅用于日志辨识。
    #[serde(default)]
    pub name: String,
    /// 所属分组；必须与渠道路由单元的 group 一致才能选到候选。
    #[serde(default = "default_group")]
    pub group: String,
    /// 允许访问的模型；空 = 不限制。支持 `gpt-4*` 通配（见 `gate::model::match_model`）。
    #[serde(default)]
    pub allowed_models: Vec<String>,
}

fn default_group() -> String {
    "default".to_string()
}

/// 一条出口节点。URL 支持 `http(s)://` / `socks5(h)://` / `ss://` / `trojan://` /
/// `vless://` / `vmess://`；协议握手由 meow 适配器在 dial 时完成。
#[derive(Debug, Deserialize, Clone)]
pub struct ProxyNodeConfig {
    /// 节点 id；0 表示由 [`build_proxy_snapshot`] 按配置顺序从 1 起编号。
    #[serde(default)]
    pub id: i64,
    /// 代理 URL，例如 `socks5://127.0.0.1:7890`。
    pub url: String,
    /// 绑定的渠道 `name` 列表；空则该节点不会被任何 `acquire` 选中。
    #[serde(default)]
    pub channel_keys: Vec<String>,
    /// 优先级，数字越大越优先。
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GatewayConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub dispatch: DispatchConfig,
    #[serde(default)]
    pub metering: MeteringConfig,
    #[serde(default)]
    pub retry: RetryConfig,
    /// 上游渠道；空 = 无可用候选，转发请求返回 404 no_route。
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    /// 本地 API key；空 = 所有请求 401。
    #[serde(default)]
    pub keys: Vec<KeyConfig>,
    /// 出口节点。空 = 模型请求直连。
    #[serde(default)]
    pub proxy_nodes: Vec<ProxyNodeConfig>,
}

fn default_listen() -> String {
    "0.0.0.0:3000".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl GatewayConfig {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

/// 单机模式的固定分组。渠道路由单元与本地 key 都落在它下面，
/// dispatch 按 `(group, public_model)` 选候选，两侧口径必须一致。
const LOCAL_GROUP: &str = "default";

/// 单机模式的固定用户 key；所有本地 token 都挂在这个用户上。
const LOCAL_USER_KEY: &str = "local";

/// 记录来源标记，写进 `SyncMeta.origin`，与 admin-sync 推送的记录区分。
const CONFIG_ORIGIN: &str = "config";

/// 构造 `SyncMeta`（带统一时间戳），确保快照内所有记录时间一致。
fn config_meta_with_time(
    key: impl Into<String>,
    updated_at: chrono::DateTime<chrono::Utc>,
) -> SyncMeta {
    SyncMeta {
        key: key.into(),
        schema_version: 1,
        logical_version: 1,
        origin: CONFIG_ORIGIN.to_string(),
        updated_at,
    }
}

/// `[[channels]]` → dispatch 路由快照。
///
/// 每个渠道生成一条 `ChannelRecord`（`meta.key` = `name`，单把 key 在 `index 0`），
/// 其 `models` 的每一项生成一条 `RouteUnitRecord`（`meta.key` = `"<name>:<model>"`）。
/// 上游真名取 `upstream_models` 的映射，缺省等于公开名。
///
/// 渠道与单元的 `status` 恒为 1（启用）：手动禁用请从配置里删除该条，
/// 自动熔断由 `dispatch::health` 的内存表判定，不写回快照。
///
/// 空输入产出空快照，`Dispatcher` 会对任何模型返回 `NoCandidate`（HTTP 404）。
pub fn build_route_snapshot(channels: &[ChannelConfig]) -> Snapshot {
    let mut units = Vec::new();
    let mut channel_map = HashMap::with_capacity(channels.len());

    // 校验渠道配置
    for cfg in channels {
        if let Err(e) = cfg.validate() {
            tracing::warn!(error = %e, "渠道配置校验失败，跳过该渠道");
            continue;
        }
    }

    // 校验渠道名唯一性
    let mut seen_names = HashSet::new();
    for cfg in channels {
        if !seen_names.insert(&cfg.name) {
            tracing::warn!(name = %cfg.name, "渠道名重复，后续同名渠道会被覆盖");
        }
    }

    // 统一时间戳，确保快照内所有记录时间一致
    let now = chrono::Utc::now();

    for cfg in channels {
        channel_map.insert(
            cfg.name.clone(),
            ChannelRecord {
                meta: config_meta_with_time(&cfg.name, now),
                name: cfg.name.clone(),
                provider_type: cfg.provider_type.clone(),
                base_url: cfg.base_url.clone(),
                keys: vec![ChannelKey {
                    index: 0,
                    secret: cfg.api_key.clone(),
                    rpm_limit: 0,
                }],
                max_concurrency: cfg.max_concurrency,
                status: 1,
                groups: vec![LOCAL_GROUP.to_string()],
                settings: serde_json::Value::Null,
            },
        );

        for public_model in &cfg.models {
            let upstream_model = cfg
                .upstream_models
                .get(public_model)
                .cloned()
                .unwrap_or_else(|| public_model.clone());
            units.push(RouteUnitRecord {
                meta: config_meta_with_time(format!("{}:{}", cfg.name, public_model), now),
                group: LOCAL_GROUP.to_string(),
                public_model: public_model.clone(),
                channel_key: cfg.name.clone(),
                key_index: 0,
                upstream_model,
                priority: cfg.priority,
                weight: cfg.weight,
                status: 1,
            });
        }
    }

    Snapshot {
        units,
        channels: channel_map,
    }
}

/// `[[keys]]` → gate 的 token 与 user 快照。
///
/// 每把 key 生成一条 `TokenRecord`，按 `sha256(明文)` 索引进 `TokenSnapshot`；
/// 全部 token 共享一条 `UserRecord`（`meta.key` = `"local"`，启用、`default` 组）。
/// `allowed_models` 为空时存 `None`（= 不限制），与 `ModelGate` 的语义一致。
///
/// `TokenRecord.meta.key` 用配置顺序的数字字符串（`"1"`、`"2"`…）：
/// gate 的 `TokenInfo.id` 是 String，直接承载它作为配额/限流分桶键，
/// 每把 key 稳定独立（不再经 i64 解析）。
///
/// 本地 token 一律 `unlimited_quota = true`：单机模式不计费，额度由
/// `build_gates` 决定是否挂 `QuotaGate` 来控制，不在记录里假造余额。
pub fn build_token_snapshot(keys: &[KeyConfig]) -> (TokenSnapshot, UserSnapshot) {
    let tokens = TokenSnapshot::default();
    let users = UserSnapshot::default();

    // 统一时间戳
    let now = chrono::Utc::now();

    users.upsert(UserRecord {
        meta: config_meta_with_time(LOCAL_USER_KEY, now),
        username: LOCAL_USER_KEY.to_string(),
        display_name: "local".to_string(),
        email: String::new(),
        quota: i64::MAX,
        used_quota: 0,
        request_count: 0,
        group: LOCAL_GROUP.to_string(),
        status: 1,
        role: 100,
        created_at: now,
    });

    for (idx, cfg) in keys.iter().enumerate() {
        let hash = gateway_gate::sha256(&cfg.key);
        let record = TokenRecord {
            meta: config_meta_with_time((idx + 1).to_string(), now),
            user_key: LOCAL_USER_KEY.to_string(),
            name: cfg.name.clone(),
            key_hash: hex::encode(hash),
            key_preview: key_preview(&cfg.key),
            group: Some(cfg.group.clone()),
            quota: 0,
            unlimited_quota: true,
            used_quota: 0,
            expires_at: None,
            status: 1,
        };
        let allowed_models = if cfg.allowed_models.is_empty() {
            None
        } else {
            Some(cfg.allowed_models.clone())
        };
        tokens.upsert(hash, TokenEntry::new(record, allowed_models));
    }

    (tokens, users)
}

/// 掩码预览 —— 只保留首尾各 4 字符，中间打星，日志里不泄明文。
fn key_preview(key: &str) -> String {
    let n = key.chars().count();
    if n <= 8 {
        return "*".repeat(n);
    }
    let head: String = key.chars().take(4).collect();
    let tail: String = key.chars().skip(n - 4).collect();
    format!("{head}****{tail}")
}

/// 代理 URL 掩码 —— userinfo 与 query（pbk/sid/密码）都含凭据，日志只保留
/// `scheme://***@host:port` 骨架，能定位节点但不泄密。
fn url_preview(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(u) => {
            let auth = if u.username().is_empty() && u.password().is_none() {
                String::new()
            } else {
                "***@".to_string()
            };
            let port = u.port().map(|p| format!(":{p}")).unwrap_or_default();
            format!(
                "{}://{}{}{}{}",
                u.scheme(),
                auth,
                u.host_str().unwrap_or("?"),
                port,
                {
                    // REALITY 的 pbk/sid 也算凭据，一并抹掉
                    if u.query().is_some() { "?***" } else { "" }
                }
            )
        }
        Err(_) => "***（URL 无法解析）***".to_string(),
    }
}

/// `[[proxy_nodes]]` → `ProxySnapshot`。非法 URL / 空 `channel_keys` 跳过并打 warn。
///
/// scheme 不再过滤：http/socks5 走 reqwest，ss/trojan/vless/vmess 走
/// `gateway_proxy::adapter_for` 的 meow 适配器（配置有误时 `adapter_for`
/// 自己 warn 并回落直连，不需要在这里预筛）。
pub fn build_proxy_snapshot(nodes: &[ProxyNodeConfig]) -> gateway_proxy::ProxySnapshot {
    use gateway_proxy::{ProxyNode, ProxySnapshot};

    let mut out = Vec::new();
    for (idx, cfg) in nodes.iter().enumerate() {
        // URL 里的 userinfo / ss-vless query 都带凭据，日志只留骨架。
        let preview = url_preview(&cfg.url);
        if cfg.channel_keys.is_empty() {
            tracing::warn!(url = %preview, "proxy_nodes 缺少 channel_keys，跳过");
            continue;
        }
        let mut node = match ProxyNode::parse_url(&cfg.url) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(url = %preview, error = %e, "proxy_nodes URL 解析失败，跳过");
                continue;
            }
        };
        node.id = if cfg.id == 0 {
            (idx as i64).saturating_add(1)
        } else {
            cfg.id
        };
        node.channel_keys = cfg.channel_keys.clone();
        node.priority = cfg.priority;
        out.push(node);
    }
    ProxySnapshot { nodes: out }
}
