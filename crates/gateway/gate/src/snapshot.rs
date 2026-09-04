//! `snapshot` —— 数据源：identity / quota / ip-policy / pricing 持有
//!
//! 所有 gate 的只读快照都来自这里，由 `service::sync` 推送更新。
//!
//! `TokenEntry` 包装 contract 的 [`TokenRecord`] + gate 自己关心的 `allowed_models`；
//! 之所以不复用 contract TokenRecord，是因为后者没列允许的模型（那是 gate-only 概念）。

use std::net::IpAddr;
use std::sync::Arc;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use contract::records::{TokenRecord, UserRecord};

pub mod adapt;
pub use adapt::{TokenView, UserView, now_unix, to_unix};

// ============================================================================
// TokenEntry —— gate 视角的 token 记录（sha256 索引）
// ============================================================================

/// gate 用到的 token 视图：contract record + 私有 `allowed_models`。
#[derive(Debug, Clone)]
pub struct TokenEntry {
    pub record: TokenRecord,
    /// 允许访问的模型名；None = 全部允许。
    /// ponytail: 这里是 gate 局部状态，没污染 contract。
    pub allowed_models: Option<Vec<String>>,
}

impl TokenEntry {
    pub fn new(record: TokenRecord, allowed_models: Option<Vec<String>>) -> Self {
        Self {
            record,
            allowed_models,
        }
    }
}

// ============================================================================
// IpPolicy — CIDR 白名单（IPv4 / IPv6 前缀）
// ============================================================================

/// IP 白名单（CIDR 前缀集合）。空集合 = 全部允许。
///
/// 内部以原始 CIDR 字符串储存（serde 友好），查询时按需解析。
/// ponytail: 字符串解析比 CidrRange 直接序列化省一堆 derive；列表不长，性能可忽略。
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct IpPolicy {
    cidrs: Vec<String>,
}

impl IpPolicy {
    pub fn allows_everyone() -> Self {
        Self { cidrs: vec![] }
    }

    pub fn from_cidrs(cidrs: &[String]) -> Self {
        Self {
            cidrs: cidrs.to_vec(),
        }
    }

    pub fn cidrs(&self) -> &[String] {
        &self.cidrs
    }

    pub fn allows(&self, ip: &IpAddr) -> bool {
        if self.cidrs.is_empty() {
            return true;
        }
        self.cidrs
            .iter()
            .any(|c| parse_cidr(c).map(|r| r.contains(ip)).unwrap_or(false))
    }
}

/// CIDR 前缀：IPv4 / IPv6 通用，按位前缀匹配。
/// ponytail: 手写前缀匹配，避开 ipnet 依赖；用例就是白名单对比。
#[derive(Debug, Clone)]
struct CidrRange {
    bytes: [u8; 16],
    prefix_len: u8, // 0..=128
    is_v4: bool,
}

fn parse_cidr(s: &str) -> Result<CidrRange, String> {
    let (addr_part, plen_part) = match s.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (s, None),
    };
    let ip: IpAddr = addr_part.parse().map_err(|e| format!("bad ip: {e}"))?;
    let plen: u8 = match plen_part {
        Some(p) => p.parse().map_err(|e| format!("bad prefix: {e}"))?,
        None => match ip {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        },
    };
    let mut bytes = [0u8; 16];
    let is_v4 = match ip {
        IpAddr::V4(v4) => {
            bytes[0..4].copy_from_slice(&v4.octets());
            true
        }
        IpAddr::V6(v6) => {
            bytes.copy_from_slice(&v6.octets());
            false
        }
    };
    let max_plen = if is_v4 { 32 } else { 128 };
    if plen > max_plen {
        return Err(format!("prefix > {max_plen}"));
    }
    Ok(CidrRange {
        bytes,
        prefix_len: plen,
        is_v4,
    })
}

impl CidrRange {
    fn contains(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => {
                let mut b = [0u8; 16];
                b[0..4].copy_from_slice(&v4.octets());
                self.contains_bytes(&b, true)
            }
            IpAddr::V6(v6) => self.contains_bytes(&v6.octets(), false),
        }
    }

    fn contains_bytes(&self, other: &[u8; 16], other_is_v4: bool) -> bool {
        if self.is_v4 != other_is_v4 {
            return false;
        }
        let full_bytes = (self.prefix_len / 8) as usize;
        let rem_bits = self.prefix_len % 8;
        if full_bytes > 0 && self.bytes[..full_bytes] != other[..full_bytes] {
            return false;
        }
        if rem_bits == 0 {
            return true;
        }
        let mask = !((1u8 << (8 - rem_bits)) - 1);
        self.bytes[full_bytes] & mask == other[full_bytes] & mask
    }
}

// ============================================================================
// TokenSnapshot — sha256(raw_key) → TokenEntry
// ============================================================================

/// Token 快照（来自 `service::sync`）。
#[derive(Debug, Clone, Default)]
pub struct TokenSnapshot {
    by_hash: DashMap<[u8; 32], TokenEntry>,
}

impl TokenSnapshot {
    pub fn upsert(&self, hash: [u8; 32], entry: TokenEntry) {
        self.by_hash.insert(hash, entry);
    }

    pub fn remove(&self, hash: &[u8; 32]) -> Option<TokenEntry> {
        self.by_hash.remove(hash).map(|(_, v)| v)
    }

    pub fn lookup(&self, hash: &[u8; 32]) -> Option<TokenEntry> {
        self.by_hash.get(hash).map(|r| r.clone())
    }
}

// ============================================================================
// UserSnapshot — UserRecord 按 user_key 字符串索引
// ============================================================================

/// User 快照（key = contract `UserRecord.meta.key`，字符串形式）。
#[derive(Debug, Clone, Default)]
pub struct UserSnapshot {
    by_key: DashMap<String, UserRecord>,
}

impl UserSnapshot {
    pub fn upsert(&self, rec: UserRecord) {
        let key = rec.meta.key.clone();
        self.by_key.insert(key, rec);
    }

    pub fn remove(&self, user_key: &str) -> Option<UserRecord> {
        self.by_key.remove(user_key).map(|(_, v)| v)
    }

    pub fn lookup(&self, user_key: &str) -> Option<UserRecord> {
        self.by_key.get(user_key).map(|r| r.clone())
    }
}

// ============================================================================
// PricingSnapshot — (model, group) → PriceRow，含 model-only 回退
// ============================================================================

/// Pricing 快照。查询：(model, group) → (model, "default") → None。
#[derive(Debug, Clone, Default)]
pub struct PricingSnapshot {
    rows: DashMap<(String, String), PriceRow>,
}

impl PricingSnapshot {
    pub fn upsert(&self, model: String, group: String, price: PriceRow) {
        self.rows.insert((model, group), price);
    }

    pub fn remove(&self, model: &str, group: &str) -> Option<PriceRow> {
        self.rows
            .remove(&(model.to_string(), group.to_string()))
            .map(|(_, v)| v)
    }

    pub fn lookup(&self, model: &str, group: &str) -> Option<PriceRow> {
        if let Some(r) = self.rows.get(&(model.to_string(), group.to_string())) {
            return Some(r.clone());
        }
        if group != "default"
            && let Some(r) = self.rows.get(&(model.to_string(), "default".to_string()))
        {
            return Some(r.clone());
        }
        None
    }
}

/// 单条计费行。单位：每 1M token 的"内部单位"（new-api 500_000 = $1）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRow {
    pub input_per_m: f64,
    pub output_per_m: f64,
    /// cached/read prompt 折价（缓存命中读取）。
    pub cache_per_m: f64,
}

// ============================================================================
// QuotaSnapshot — token_key → 剩余额度（i64，内部单位）
// ============================================================================

/// Quota 快照（key = `TokenRecord.meta.key`，字符串）。
///
/// ponytail: 单一余额表，按 token 维度；user 维度需要时再拆。
#[derive(Debug, Clone, Default)]
pub struct QuotaSnapshot {
    by_token: DashMap<String, i64>,
}

impl QuotaSnapshot {
    pub fn upsert(&self, token_key: String, remaining: i64) {
        self.by_token.insert(token_key, remaining);
    }

    pub fn add(&self, token_key: &str, delta: i64) {
        let mut e = self.by_token.entry(token_key.to_string()).or_insert(0);
        *e = e.saturating_add(delta);
    }

    pub fn remove(&self, token_key: &str) -> Option<i64> {
        self.by_token.remove(token_key).map(|(_, v)| v)
    }

    /// 查 token 剩余额度；缺失 → 0（保守拒绝 = 视作不够）。
    pub fn remaining(&self, token_key: &str) -> i64 {
        self.by_token
            .get(token_key)
            .map(|r| *r.value())
            .unwrap_or(0)
    }
}

// ============================================================================
// ArcSwap 句柄别名
// ============================================================================

pub type SharedTokenSnapshot = Arc<ArcSwap<TokenSnapshot>>;
pub type SharedUserSnapshot = Arc<ArcSwap<UserSnapshot>>;
pub type SharedIpPolicy = Arc<ArcSwap<IpPolicy>>;
pub type SharedPricing = Arc<ArcSwap<PricingSnapshot>>;
pub type SharedQuota = Arc<ArcSwap<QuotaSnapshot>>;
