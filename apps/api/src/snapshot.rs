//! Ferrite — API snapshot loader
//! 加载 PG 管理表快照供 dispatch/gate 使用
//!
//! # 数据来源
//! - api_channels (status=1) → contract::records::ChannelRecord
//! - api_channels.models JSONB → RouteUnitRecord 数组展开
//! - api_tokens (status=1) → TokenRecord + gate::snapshot::TokenSnapshot
//! - auth_users (status=1) → UserRecord + gate::snapshot::UserSnapshot
//!
//! # 桥接形状
//! 所有输出类型均来自 `contract::records::*` 或 gate 快照类型。
//!
//! # 快照结构
//! ```rust
//! pub struct Snapshots {
//!     pub dispatch: dispatch::Snapshot,
//!     pub token_snapshot: gateway_gate::snapshot::SharedTokenSnapshot,
//!     pub user_snapshot: gateway_gate::snapshot::SharedUserSnapshot,
//!     pub quota_snapshot: gateway_gate::snapshot::SharedQuota,
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use sqlx::{postgres::PgPool, Row};

use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, TokenRecord, UserRecord};
use contract::SCHEMA_VERSION;

use gateway_gate::snapshot::{
    QuotaSnapshot, SharedQuota, SharedTokenSnapshot, SharedUserSnapshot, TokenEntry, TokenSnapshot,
    UserSnapshot,
};

use dispatch::Snapshot as DispatchSnapshot;

/// 从 admin-catalog 表加载快照
pub async fn load_snapshots(pool: &PgPool) -> anyhow::Result<Snapshots> {
    // 1. 加载渠道数据
    let (channels, route_units) = load_channels_and_units(pool).await?;
    let mut channel_map: HashMap<String, ChannelRecord> = HashMap::new();
    for ch in channels {
        channel_map.insert(ch.meta.key.clone(), ch);
    }

    // 2. 加载令牌数据
    let (token_records, token_snapshot) = load_tokens(pool).await?;

    // 3. 加载用户数据
    let user_snapshot = load_users(pool).await?;

    // 4. 构建 quota 快照（token_key → 剩余额度）
    let quota_snapshot = build_quota_snapshot(&token_records);

    // 5. 构建 dispatch 快照
    let dispatch_snapshot = DispatchSnapshot {
        units: route_units,
        channels: channel_map,
    };

    Ok(Snapshots {
        dispatch: dispatch_snapshot,
        token_snapshot,
        user_snapshot,
        quota_snapshot,
    })
}

/// 加载渠道记录并展开路由单元
async fn load_channels_and_units(
    pool: &PgPool,
) -> anyhow::Result<(Vec<ChannelRecord>, Vec<RouteUnitRecord>)> {
    let rows = sqlx::query(
        r#"
        SELECT key, name, channel_type, base_url, keys, models, group_name, priority, weight, status
        FROM api_channels
        WHERE status = 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut channels = Vec::new();
    let mut route_units = Vec::new();

    for row in rows {
        let channel_key: uuid::Uuid = row.try_get("key")?;
        let channel_key_str = channel_key.to_string();
        let name: String = row.try_get("name")?;
        let channel_type: String = row.try_get("channel_type")?;
        let base_url: String = row.try_get("base_url")?;
        let keys_json: Value = row.try_get("keys")?;
        let models_json: Value = row.try_get("models")?;
        let group_name: String = row.try_get("group_name")?;
        let priority: i32 = row.try_get("priority")?;
        let weight: i32 = row.try_get("weight")?;
        let status: i16 = row.try_get("status")?;

        // Build ChannelKey array from JSONB string array
        let channel_keys: Vec<ChannelKey> = keys_json
            .as_array()
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .map(|(idx, v)| ChannelKey {
                        index: idx as u32,
                        secret: v.as_str().unwrap_or("").to_string(),
                        rpm_limit: 0,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Build SyncMeta for channel
        let channel_meta = contract::records::SyncMeta {
            key: channel_key_str.clone(),
            schema_version: SCHEMA_VERSION,
            logical_version: 1,
            origin: "admin".into(),
            updated_at: Utc::now(),
        };

        // Build ChannelRecord
        let channel = ChannelRecord {
            meta: channel_meta.clone(),
            name,
            provider_type: channel_type,
            base_url,
            keys: channel_keys,
            max_concurrency: 8, // ponytail: 固定值，避免额外配置开销
            status: status as u8,
            groups: vec![group_name.clone()],
            settings: Value::Null,
        };

        channels.push(channel);

        // Expand models JSONB to RouteUnitRecord
        let units = expand_models_json(
            &models_json,
            &channel_key_str,
            &group_name,
            priority,
            weight,
        );
        route_units.extend(units);
    }

    Ok((channels, route_units))
}

/// 从 models JSONB 展开 RouteUnitRecord
/// - 字符串数组 ["m1"] → public_model=upstream_model="m1"
/// - 对象数组 [{"alias":"public","upstream":"upstream"}] → 映射对
/// - 其他形状 → warn! + 跳过
fn expand_models_json(
    models_json: &Value,
    channel_key: &str,
    group_name: &str,
    priority: i32,
    weight: i32,
) -> Vec<RouteUnitRecord> {
    let mut units = Vec::new();

    if let Some(arr) = models_json.as_array() {
        for (idx, item) in arr.iter().enumerate() {
            let (public_model, upstream_model) = match item {
                Value::String(s) => (s.clone(), s.clone()),
                Value::Object(obj) => {
                    let public = obj.get("alias").and_then(|v| v.as_str()).unwrap_or("");
                    let upstream = obj.get("upstream").and_then(|v| v.as_str()).unwrap_or("");
                    if public.is_empty() || upstream.is_empty() {
                        tracing::warn!("skip invalid model object at index {}: missing alias/upstream", idx);
                        continue;
                    }
                    (public.to_string(), upstream.to_string())
                }
                _ => {
                    tracing::warn!("skip unsupported model type at index {}", idx);
                    continue;
                }
            };

            let unit_key = format!("{}:{}", channel_key, public_model);
            let unit_meta = contract::records::SyncMeta {
                key: unit_key,
                schema_version: SCHEMA_VERSION,
                logical_version: 1,
                origin: "admin".into(),
                updated_at: Utc::now(),
            };

            units.push(RouteUnitRecord {
                meta: unit_meta,
                group: group_name.to_string(),
                public_model,
                channel_key: channel_key.to_string(),
                key_index: 0,
                upstream_model,
                priority,
                weight: weight as u32,
                status: 1,
            });
        }
    }

    units
}

/// 加载令牌记录并构建 gate TokenSnapshot
async fn load_tokens(pool: &PgPool) -> anyhow::Result<(Vec<TokenRecord>, SharedTokenSnapshot)> {
    let rows = sqlx::query(
        r#"
        SELECT key, user_key, name, key_hash, key_preview, group_id, quota, unlimited_quota, used_quota, expires_at, status
        FROM api_tokens
        WHERE status = 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut token_records = Vec::new();
    let snapshot = TokenSnapshot::default();

    for row in rows {
        let key: uuid::Uuid = row.try_get("key")?;
        let key_str = key.to_string();
        let user_key: uuid::Uuid = row.try_get("user_key")?;
        let name: String = row.try_get("name")?;
        let key_hash: String = row.try_get("key_hash")?;
        let key_preview: String = row.try_get("key_preview")?;
        let group_id: Option<String> = row.try_get("group_id")?;
        let quota: i64 = row.try_get("quota")?;
        let unlimited_quota: bool = row.try_get("unlimited_quota")?;
        let used_quota: i64 = row.try_get("used_quota")?;
        let expires_at: Option<chrono::DateTime<Utc>> = row.try_get("expires_at")?;
        let status: i16 = row.try_get("status")?;

        let token_meta = contract::records::SyncMeta {
            key: key_str.clone(),
            schema_version: SCHEMA_VERSION,
            logical_version: 1,
            origin: "admin".into(),
            updated_at: Utc::now(),
        };

        let token = TokenRecord {
            meta: token_meta,
            user_key: user_key.to_string(),
            name,
            key_hash: key_hash.clone(),
            key_preview,
            group: group_id,
            quota,
            unlimited_quota,
            used_quota,
            expires_at,
            status: status as u8,
        };

        // Build TokenSnapshot
        let entry = TokenEntry::new(token.clone(), None); // ponytail: 暂无模型白名单，默认全部允许
        let hash_bytes = hex::decode(&key_hash)?;
        let hash_arr: [u8; 32] = hash_bytes[..].try_into()?;
        snapshot.upsert(hash_arr, entry);
        token_records.push(token);
    }

    let shared_snapshot = Arc::new(arc_swap::ArcSwap::from_pointee(snapshot));
    Ok((token_records, shared_snapshot))
}

/// 加载用户记录并构建 gate UserSnapshot
async fn load_users(pool: &PgPool) -> anyhow::Result<SharedUserSnapshot> {
    let rows = sqlx::query(
        r#"
        SELECT key, username, display_name, email, quota, used_quota, group_id, role, status, created_at
        FROM auth_users
        WHERE status = 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    let snapshot = UserSnapshot::default();

    for row in rows {
        let key: uuid::Uuid = row.try_get("key")?;
        let username: String = row.try_get("username")?;
        let display_name: String = row.try_get("display_name")?;
        let email: String = row.try_get("email")?;
        let quota: i64 = row.try_get("quota")?;
        let used_quota: i64 = row.try_get("used_quota")?;
        let group_id: String = row.try_get("group_id")?;
        let role: i16 = row.try_get("role")?;
        let status: i16 = row.try_get("status")?;
        let created_at: chrono::DateTime<Utc> = row.try_get("created_at")?;

        snapshot.upsert(UserRecord {
            meta: contract::records::SyncMeta {
                key: key.to_string(),
                schema_version: SCHEMA_VERSION,
                logical_version: 1,
                origin: "admin".into(),
                updated_at: Utc::now(),
            },
            username,
            display_name,
            email,
            quota,
            used_quota,
            request_count: 0,
            group: group_id,
            status: status as u8,
            role: role as u16,
            created_at,
        });
    }

    Ok(Arc::new(arc_swap::ArcSwap::from_pointee(snapshot)))
}

/// 构建 quota 快照 (token_key → 剩余额度)
/// 直接使用 token row 中的 used_quota，不需要额外查询
fn build_quota_snapshot(token_records: &[TokenRecord]) -> SharedQuota {
    let quota_snapshot = QuotaSnapshot::default();

    for token in token_records {
        let remaining = if token.unlimited_quota {
            i64::MAX
        } else {
            (token.quota - token.used_quota).max(0)
        };
        quota_snapshot.upsert(token.meta.key.clone(), remaining);
    }

    let shared = Arc::new(arc_swap::ArcSwap::from_pointee(quota_snapshot));
    shared
}

/// 快照容器
#[derive(Debug, Clone)]
pub struct Snapshots {
    pub dispatch: DispatchSnapshot,
    pub token_snapshot: SharedTokenSnapshot,
    pub user_snapshot: SharedUserSnapshot,
    pub quota_snapshot: SharedQuota,
}