//! Ferrite — API snapshot loader
//! 加载 PG 管理表快照供 dispatch/gate 使用
//!
//! # 数据来源
//! - api_channels (status=1) → contract::records::ChannelRecord
//! - api_channels.models JSONB × keys → RouteUnitRecord 数组展开（每 key 一个 unit）
//! - api_tokens (status=1) → TokenRecord + gate::snapshot::TokenSnapshot
//!   （含 api_tokens.allowed_models JSONB → `TokenEntry.allowed_models`）
//! - auth_users (status=1) → UserRecord + gate::snapshot::UserSnapshot
//! - api_groups (全量，含禁用) → gate::snapshot::GroupSnapshot
//!   （model_whitelist JSONB → 组级模型白名单；ratio → 分组倍率；
//!   status → `GroupEntry.enabled`，禁用组进快照供 gate 整组拒绝）
//! - model_prices → 计费价格行（billing::PgPriceTable 的数据源）
//!
//! # 桥接形状
//! 所有输出类型均来自 `contract::records::*` 或 gate 快照类型。
//!
//! # 快照结构
//! ```text
//! pub struct Snapshots {
//!     pub dispatch: dispatch::Snapshot,
//!     pub token_snapshot: gateway_gate::snapshot::SharedTokenSnapshot,
//!     pub user_snapshot: gateway_gate::snapshot::SharedUserSnapshot,
//!     pub quota_snapshot: gateway_gate::snapshot::SharedQuota,
//!     pub group_snapshot: gateway_gate::snapshot::SharedGroupSnapshot,
//!     pub price_rows: SharedPriceRows,
//!     pub name_directory: billing::SharedNameDirectory,
//!     pub channel_names: SharedChannelNames,
//! }
//! ```
//!
//! # reload 分层
//! 「加载纯值」（[`load_snapshot_data`]）与「包装 / store」（[`load_snapshots`] /
//! [`reload_snapshots`]）拆成两层：boot 路径新建 `Shared*` 实例；reload 路径向
//! **既有同一批** `Shared*` 实例 store 新值（gate、计费 sink 与 reload 路由持有
//! 的正是这些实例，store 后自动看到新数据），Dispatcher 侧走 `Dispatcher::set_snapshot`。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use sqlx::{Row, postgres::PgPool};

use contract::SCHEMA_VERSION;
use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, TokenRecord, UserRecord};

use gateway_gate::snapshot::{
    GroupSnapshot, QuotaSnapshot, SharedGroupSnapshot, SharedQuota, SharedTokenSnapshot,
    SharedUserSnapshot, TokenEntry, TokenSnapshot, UserSnapshot,
};

use dispatch::Dispatcher;
use dispatch::Snapshot as DispatchSnapshot;

use crate::billing::{NameDirectory, SharedNameDirectory};

/// 模型单价行集合的共享句柄（`Arc<ArcSwap<T>>`）：boot 新建、reload 原地
/// store 换新。行形状 = `(model, input, output, cache)`，单位 $/M tokens。
pub type SharedPriceRows = Arc<arc_swap::ArcSwap<Vec<(String, f64, f64, f64)>>>;

/// 渠道展示名映射的共享句柄（`Arc<ArcSwap<T>>`）：boot 新建、reload 原地
/// store 换新。键 = 渠道 UUID 字符串（`ChannelRecord.meta.key`，PG UUID 列
/// 的 `to_string()`），值 = 渠道展示名；结算 sink 在 submit 时现读，
/// **渠道改名免重启生效**。
pub type SharedChannelNames = Arc<arc_swap::ArcSwap<HashMap<String, String>>>;

/// 读模型单价表（迁移 0003）：每行 `(model, input, output, cache)`，$/M tokens。
///
/// 计费权威裁决见 [`crate::billing`] 模块文档：这是 pipeline 结算价格表的
/// 唯一数据源，未知模型（表里没有的行）→ 免费落账。boot 与 reload 共用。
pub async fn load_model_prices(pool: &PgPool) -> anyhow::Result<Vec<(String, f64, f64, f64)>> {
    let rows: Vec<(String, f64, f64, f64)> =
        sqlx::query_as("SELECT model, input, output, cache FROM model_prices")
            .fetch_all(pool)
            .await?;
    Ok(rows)
}

/// 一次 JOIN 算出全部启用货币用户的折算综合可用值（#179 多货币）。
///
/// `available_i64 = COALESCE(SUM(amount × internal_rate), 0)`，仅启用货币。
/// 返回 `user UUID 字符串 → i64`；user_balances 无行的用户不进 map
/// （消费方 `unwrap_or(0)`，语义 = 没充值就拦截）。
async fn load_user_quotas(pool: &PgPool) -> anyhow::Result<HashMap<String, i64>> {
    // LEAST 夹住 i64::MAX：amount 是 BIGINT、internal_rate 是 DOUBLE，
    // 乘积可能超出 BIGINT 域（PG 直接抛 numeric out of range，boot 会挂）。
    // 夹在 SQL 侧比 Rust 侧安全：转换前就不可能越界。
    let rows: Vec<(uuid::Uuid, i64)> = sqlx::query_as(
        r#"
        SELECT ub.user_key,
               LEAST(
                   COALESCE(SUM(ub.amount * cd.internal_rate), 0),
                   9223372036854775807::double precision
               )::BIGINT AS available
        FROM user_balances ub
        JOIN currency_defs cd ON cd.code = ub.currency_code AND cd.enabled
        GROUP BY ub.user_key
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

/// 从 admin-catalog 表加载快照（boot 路径：新建 `Shared*` 实例）。
pub async fn load_snapshots(pool: &PgPool) -> anyhow::Result<Snapshots> {
    let input = load_snapshot_data(pool).await?;
    let (dispatch_snapshot, quota_snapshot) = build_dispatch_and_quota(
        input.channels,
        input.route_units,
        &input.token_records,
        &input.user_quotas,
    );

    Ok(Snapshots {
        dispatch: dispatch_snapshot,
        token_snapshot: shared(input.token_snapshot),
        user_snapshot: shared(input.user_snapshot),
        quota_snapshot: shared(quota_snapshot),
        group_snapshot: shared(input.group_snapshot),
        price_rows: shared(input.price_rows),
        name_directory: shared(input.name_directory),
        channel_names: shared(input.channel_names),
    })
}

/// 一次 boot/reload 从 PG 加载出的**纯值**快照集合（未包 `Shared*`、未进 Dispatcher）。
///
/// 拆出纯值层的原因：boot 时是「纯值 → 新建 `Shared*`」，reload 时是
/// 「纯值 → store 进既有 `Shared*`」；只有把加载结果与包装方式分离，
/// 两条路径才能共用同一份加载逻辑。
#[derive(Debug)]
pub struct ReloadInput {
    pub channels: Vec<ChannelRecord>,
    pub route_units: Vec<RouteUnitRecord>,
    pub token_records: Vec<TokenRecord>,
    pub token_snapshot: TokenSnapshot,
    pub user_records: Vec<UserRecord>,
    pub user_snapshot: UserSnapshot,
    pub group_snapshot: GroupSnapshot,
    /// 本次载入的**启用**分组数（`status = 1`）。禁用组会进 `GroupSnapshot`
    /// 供 gate 整组拒绝，但不进对外计数——`ReloadCounts::groups` 口径与
    /// 改造前的"只收启用组"一致。`GroupSnapshot` 无 len 访问器（内部 HashMap 私有），
    /// 计数只能在加载侧（遍历 PG 行时）一并算出，供 [`ReloadCounts::groups`] 上报。
    pub group_count: usize,
    /// 模型单价行（`(model, input, output, cache)`，$/M tokens）——
    /// 计费价格表数据源，见 [`load_model_prices`]。
    pub price_rows: Vec<(String, f64, f64, f64)>,
    /// 用户/令牌展示名目录（usage_logs 冗余展示字段用）。
    pub name_directory: NameDirectory,
    /// 渠道 UUID 字符串 → 展示名（usage_logs.channel_name 冗余展示字段用）。
    /// 与 `channels` 同批加载逐条克隆而来，保证名单与渠道快照同一份数据。
    pub channel_names: HashMap<String, String>,
    /// user UUID 字符串 → 折算综合可用值 available_i64（user_balances ×
    /// currency_defs.internal_rate 求和，#179 多货币）。quota 快照按
    /// token→user 展开时灌这个值。
    pub user_quotas: HashMap<String, i64>,
}

/// reload 结果计数：`json!` 序列化后作为响应 `data` 字段（snake_case 键即字段名）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReloadCounts {
    pub channels: u64,
    pub route_units: u64,
    pub tokens: u64,
    pub users: u64,
    /// 启用的分组数（禁用组进快照但不计数，见 [`ReloadInput::group_count`]）。
    pub groups: u64,
}

/// 从 PG 管理表加载纯值快照数据（boot 与 reload 共用同一加载逻辑）。
async fn load_snapshot_data(pool: &PgPool) -> anyhow::Result<ReloadInput> {
    // 1. 加载渠道数据
    let (channels, route_units) = load_channels_and_units(pool).await?;

    // 渠道展示名映射：与 channels 同批逐条克隆（meta.key 是 UUID 字符串，
    // 与结算事件 UsageEventRecord.channel_key 的归因键同形，查表无需解析）。
    let channel_names: HashMap<String, String> = channels
        .iter()
        .map(|ch| (ch.meta.key.clone(), ch.name.clone()))
        .collect();

    // 2. 加载令牌数据（纯值：records + 纯 TokenSnapshot）
    let (token_records, token_snapshot) = load_tokens(pool).await?;

    // 3. 加载用户数据（纯值：records + 纯 UserSnapshot）
    let (user_records, user_snapshot) = load_users(pool).await?;

    // 4. 加载分组数据（纯值 GroupSnapshot：组级模型白名单 + 分组倍率）
    let (group_snapshot, group_count) = load_groups(pool).await?;

    // 5. 加载计费数据：模型单价行 + 用户/令牌展示名目录（token/user records
    //    已在手，目录纯构造不查库）
    let price_rows = load_model_prices(pool).await?;
    let name_directory = NameDirectory::new(&token_records, &user_records);

    // 6. 加载用户货币余额折算值（#179 多货币）：一次 JOIN 算出全部用户的
    //    available_i64；表缺列/查询失败 → 全 0（余额不足语义，不炸 boot）。
    let user_quotas = load_user_quotas(pool).await?;

    Ok(ReloadInput {
        channels,
        route_units,
        token_records,
        token_snapshot,
        user_records,
        user_snapshot,
        group_snapshot,
        group_count,
        price_rows,
        name_directory,
        channel_names,
        user_quotas,
    })
}

/// 从纯值构建 dispatch 快照与 quota 快照（boot 与 reload 共用的纯函数）。
fn build_dispatch_and_quota(
    channels: Vec<ChannelRecord>,
    route_units: Vec<RouteUnitRecord>,
    token_records: &[TokenRecord],
    user_quotas: &HashMap<String, i64>,
) -> (DispatchSnapshot, QuotaSnapshot) {
    let mut channel_map: HashMap<String, ChannelRecord> = HashMap::new();
    for ch in channels {
        channel_map.insert(ch.meta.key.clone(), ch);
    }

    let quota_snapshot = build_quota_snapshot(token_records, user_quotas);
    let dispatch_snapshot = DispatchSnapshot {
        units: route_units,
        channels: channel_map,
    };
    (dispatch_snapshot, quota_snapshot)
}

/// 把纯值包成 `Arc<ArcSwap<T>>`（即 gate / 计费组件持有的 `Shared*` 形状）。
fn shared<T>(value: T) -> Arc<arc_swap::ArcSwap<T>> {
    Arc::new(arc_swap::ArcSwap::from_pointee(value))
}

/// POST /api/gateway/reload 的热更实现：加载最新管理表数据并热更进运行时。
///
/// `target` 必须是 boot 时喂给 gate / 计费 sink 的**同一批 `Shared*` 实例**：
/// 它们是 `Arc<ArcSwap<T>>`，store 新值后所有持有者（AuthGate / StateGate /
/// QuotaGate / 结算 sink）自动看到新数据，无需重建任何组件。
/// Dispatcher 侧走 [`Dispatcher::set_snapshot`] 原地换快照。
///
/// # 非原子性
/// 五次 store（token / user / quota / group / dispatch）**不是一个原子事务**：两次 store
/// 之间在途请求可能短暂看到新旧混合视图（例如新 token 快照 + 旧渠道快照）。
/// 单机管理面 reload 的瞬时窗口可接受，不为一致性引入全局锁。
///
/// # `Snapshots.dispatch` 字段的陈旧性
/// [`Snapshots::dispatch`] 只是 boot 时喂给 `Dispatcher::new` 的普通值副本，
/// **boot 之后即陈旧**：reload 走 `dispatcher.set_snapshot` 换新，不会回写该字段。
/// 只有 boot 路径读它，运行期一律以 `Dispatcher` 内部快照为准。
pub async fn reload_snapshots(
    pool: &PgPool,
    target: &Snapshots,
    dispatcher: &Dispatcher,
) -> anyhow::Result<ReloadCounts> {
    let input = load_snapshot_data(pool).await?;
    Ok(apply_snapshot_reload(target, dispatcher, input))
}

/// reload 的纯逻辑部分：把 [`ReloadInput`] store 进 `target` 的 `Shared*`
/// 与 `dispatcher`，返回本次热更的计数。不碰 PG，可离线单测。
///
/// store 顺序：先 `Shared*`（token → user → quota → group），最后换 Dispatcher——
/// 让鉴权先看到新 token，紧随其后的请求用新渠道/路由调度。各次 store 之间
/// 存在非原子窗口（见 [`reload_snapshots`] 文档），此处刻意不加锁。
pub fn apply_snapshot_reload(
    target: &Snapshots,
    dispatcher: &Dispatcher,
    input: ReloadInput,
) -> ReloadCounts {
    let (dispatch_snapshot, quota_snapshot) = build_dispatch_and_quota(
        input.channels,
        input.route_units,
        &input.token_records,
        &input.user_quotas,
    );
    let counts = ReloadCounts {
        channels: dispatch_snapshot.channels.len() as u64,
        route_units: dispatch_snapshot.units.len() as u64,
        tokens: input.token_records.len() as u64,
        users: input.user_records.len() as u64,
        groups: input.group_count as u64,
    };

    target.token_snapshot.store(Arc::new(input.token_snapshot));
    target.user_snapshot.store(Arc::new(input.user_snapshot));
    target.quota_snapshot.store(Arc::new(quota_snapshot));
    target.group_snapshot.store(Arc::new(input.group_snapshot));
    // 计费快照随 reload 换新：价格行 / 展示名目录 / 渠道名映射。三者的消费方
    // （PgPriceTable、PgSettleSink）都持同一批 ArcSwap 句柄、读取时现 load，
    // store 后下一次 lookup / submit 即读到新值——改价、改名都免重启。
    target.price_rows.store(Arc::new(input.price_rows));
    target.name_directory.store(Arc::new(input.name_directory));
    target.channel_names.store(Arc::new(input.channel_names));
    dispatcher.set_snapshot(Arc::new(dispatch_snapshot));
    counts
}

/// 加载渠道记录并展开路由单元
async fn load_channels_and_units(
    pool: &PgPool,
) -> anyhow::Result<(Vec<ChannelRecord>, Vec<RouteUnitRecord>)> {
    let rows = sqlx::query(
        r#"
        SELECT key, name, channel_type, base_url, keys, models, groups, priority, weight, status
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
        // groups TEXT[]：一个渠道可服务多个分组（#105 迁移后 group_name 列已删）
        let groups: Vec<String> = row.try_get("groups")?;
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
        // key 数量先存下：channel.push 后借用不到了，而展开路由单元需要它
        let key_count = channel_keys.len();

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
            groups: groups.clone(),
            settings: Value::Null,
        };

        channels.push(channel);

        // Expand models JSONB to RouteUnitRecord（按渠道 key 数展开：一把 key 一个 unit）
        let units = expand_models_json(
            &models_json,
            &channel_key_str,
            &groups,
            key_count,
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
/// - 笛卡尔积 groups × models × keys 在内存展开（PG 不存派生路由）；
///   空 groups 的渠道不产生任何路由单元（不服务任何分组）；
///   `key_count == 0`（渠道没配 key）同样不产生单元——没有可用凭据，
///   展开了也永远过不了候选解析（候选凭据取自 `channel.keys[key_index]`）。
///
/// 为什么每把 key 一个 unit（而不是过去恒 `key_index = 0`）：
/// dispatch 的健康状态机以 `unit.meta.key` 为粒度熔断/加权（见 dispatch::health），
/// candidate 再按 `unit.key_index` 取回对应渠道的 key。把多 key 渠道摊成
/// 每 key 一个 unit 后，单 key 挂了只熔断它自己的 unit，选择器自然把流量
/// 轮换到同渠道其余健在 key 上，不再全押第 0 把。
/// unit_key 因此带 key_index 后缀（`{channel}:{group}:{model}:{key_index}`）保持唯一。
pub fn expand_models_json(
    models_json: &Value,
    channel_key: &str,
    groups: &[String],
    key_count: usize,
    priority: i32,
    weight: i32,
) -> Vec<RouteUnitRecord> {
    let mut units = Vec::new();

    if groups.is_empty() {
        tracing::warn!(
            channel = channel_key,
            "channel has no groups; no route units"
        );
        return units;
    }

    if key_count == 0 {
        tracing::warn!(channel = channel_key, "channel has no keys; no route units");
        return units;
    }

    if let Some(arr) = models_json.as_array() {
        for (idx, item) in arr.iter().enumerate() {
            let (public_model, upstream_model) = match item {
                Value::String(s) => (s.clone(), s.clone()),
                Value::Object(obj) => {
                    let public = obj.get("alias").and_then(|v| v.as_str()).unwrap_or("");
                    let upstream = obj.get("upstream").and_then(|v| v.as_str()).unwrap_or("");
                    if public.is_empty() || upstream.is_empty() {
                        tracing::warn!(
                            "skip invalid model object at index {}: missing alias/upstream",
                            idx
                        );
                        continue;
                    }
                    (public.to_string(), upstream.to_string())
                }
                _ => {
                    tracing::warn!("skip unsupported model type at index {}", idx);
                    continue;
                }
            };

            for group in groups {
                for key_index in 0..key_count as u32 {
                    let unit_key =
                        format!("{}:{}:{}:{}", channel_key, group, public_model, key_index);
                    let unit_meta = contract::records::SyncMeta {
                        key: unit_key,
                        schema_version: SCHEMA_VERSION,
                        logical_version: 1,
                        origin: "admin".into(),
                        updated_at: Utc::now(),
                    };

                    units.push(RouteUnitRecord {
                        meta: unit_meta,
                        group: group.clone(),
                        public_model: public_model.clone(),
                        channel_key: channel_key.to_string(),
                        key_index,
                        upstream_model: upstream_model.clone(),
                        priority,
                        weight: weight as u32,
                        status: 1,
                    });
                }
            }
        }
    }

    units
}

/// 加载令牌记录并构建**纯值** TokenSnapshot（不包 Shared；包装归 load_snapshots /
/// store 归 apply_snapshot_reload）。
async fn load_tokens(pool: &PgPool) -> anyhow::Result<(Vec<TokenRecord>, TokenSnapshot)> {
    let rows = sqlx::query(
        r#"
        SELECT key, user_key, name, key_hash, key_preview, group_id, quota, unlimited_quota, used_quota, allowed_models, expires_at, status
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
        let allowed_models_json: Value = row.try_get("allowed_models")?;
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

        // allowed_models JSONB（'[]' = 不限制）→ gate 的 Option<Vec<String>>；
        // ModelGate 只在 Some(非空) 且白名单不含请求模型时拒绝。
        let allowed_models = parse_token_allowed_models(&allowed_models_json);
        // Build TokenSnapshot
        let entry = TokenEntry::new(token.clone(), allowed_models);
        let hash_bytes = hex::decode(&key_hash)?;
        let hash_arr: [u8; 32] = hash_bytes[..].try_into()?;
        snapshot.upsert(hash_arr, entry);
        token_records.push(token);
    }

    Ok((token_records, snapshot))
}

/// 解析模型名 JSONB 数组（`["gpt-4", "gpt-4*"]`）为 `Vec<String>`。
///
/// 供 token 级（`api_tokens.allowed_models`）与组级（`api_groups.model_whitelist`）
/// 两处白名单共用：非数组 / 非字符串元素一律跳过（脏数据不炸快照加载），
/// 解析结果为空时由调用方按各自语义解释（token：None=不限；组：空=不限）。
pub fn parse_model_list_json(value: &Value) -> Vec<String> {
    match value.as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        None => {
            if !value.is_null() {
                tracing::warn!(?value, "模型白名单不是 JSON 数组，按空处理");
            }
            Vec::new()
        }
    }
}

/// 解析 `api_tokens.allowed_models` JSONB 为 gate 的 `Option<Vec<String>>`：
/// 空数组 / 非法形状 → `None`（零限制 = 不挡任何模型）；
/// 非空数组 → `Some(...)`，交给 ModelGate 做 `gpt-4*` 通配匹配。
pub fn parse_token_allowed_models(value: &Value) -> Option<Vec<String>> {
    let models = parse_model_list_json(value);
    if models.is_empty() {
        None
    } else {
        Some(models)
    }
}

/// 加载用户记录并构建**纯值** UserSnapshot（不包 Shared；包装归 load_snapshots /
/// store 归 apply_snapshot_reload）。records 一并返回供 reload 计数。
async fn load_users(pool: &PgPool) -> anyhow::Result<(Vec<UserRecord>, UserSnapshot)> {
    let rows = sqlx::query(
        r#"
        SELECT key, username, display_name, email, quota, used_quota, group_id, role, status, created_at
        FROM auth_users
        WHERE status = 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut user_records = Vec::new();
    let snapshot = UserSnapshot::default();

    for row in rows {
        let key: uuid::Uuid = row.try_get("key")?;
        let username: String = row.try_get("username")?;
        let display_name: String = row.try_get("display_name")?;
        // auth_users.email 可为 NULL（DDL 无 NOT NULL）；UserRecord.email 是 String
        let email: Option<String> = row.try_get("email")?;
        let email = email.unwrap_or_default();
        let quota: i64 = row.try_get("quota")?;
        let used_quota: i64 = row.try_get("used_quota")?;
        let group_id: String = row.try_get("group_id")?;
        let role: i16 = row.try_get("role")?;
        let status: i16 = row.try_get("status")?;
        let created_at: chrono::DateTime<Utc> = row.try_get("created_at")?;

        let rec = UserRecord {
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
        };
        user_records.push(rec.clone());
        snapshot.upsert(rec);
    }

    Ok((user_records, snapshot))
}

/// 加载分组记录（`api_groups`，**全量含禁用**）并构建**纯值** GroupSnapshot。
///
/// 禁用组（status ≠ 1）不再被 SQL 过滤掉——它们带着 `enabled = false` 进快照，
/// 由 `GroupModelGate` 据此对该组请求整组拒绝（迁移 0001 的 status 列契约）。
/// 返回 `(快照, 启用组数)`：`GroupSnapshot` 没有 len 访问器，reload 计数在此处
/// 顺手算出，且只数启用组（对外口径与改造前一致）。
/// PG 读取是薄壳，白名单/倍率/启用位的拼装语义全部在 [`build_group_snapshot`]（纯函数，可离线圈测）。
async fn load_groups(pool: &PgPool) -> anyhow::Result<(GroupSnapshot, usize)> {
    let rows = sqlx::query(
        r#"
        SELECT name, ratio, model_whitelist, status
        FROM api_groups
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut parsed = Vec::with_capacity(rows.len());
    for row in &rows {
        let name: String = row.try_get("name")?;
        let ratio: f64 = row.try_get("ratio")?;
        let model_whitelist: Value = row.try_get("model_whitelist")?;
        let status: i16 = row.try_get("status")?;
        parsed.push((name, ratio, model_whitelist, status == 1));
    }
    let count = parsed.iter().filter(|(_, _, _, enabled)| *enabled).count();
    Ok((build_group_snapshot(&parsed), count))
}

/// 从 api_groups 的 `(name, ratio, model_whitelist, enabled)` 行纯构建 [`GroupSnapshot`]。
///
/// - `model_whitelist` JSONB：`["gpt-4*"]` 形状；空数组 = 该组不限模型，
///   非数组/脏元素按空处理（不炸整个快照加载）。
/// - `ratio` 即分组倍率，原样写入 `GroupEntry::multiplier`——非法值（≤0/NaN/inf）
///   由 `GroupSnapshot::upsert` 兜底回落 1.0，本函数不重复校验。
/// - `enabled`（PG `status == 1`）写入 `GroupEntry::enabled`；false 的组由
///   `GroupModelGate` 整组拒绝。
pub fn build_group_snapshot(rows: &[(String, f64, Value, bool)]) -> GroupSnapshot {
    let mut snapshot = GroupSnapshot::default();
    for (name, ratio, model_whitelist, enabled) in rows {
        snapshot.upsert(
            name.clone(),
            gateway_gate::snapshot::GroupEntry {
                allowed_models: parse_model_list_json(model_whitelist),
                multiplier: *ratio,
                enabled: *enabled,
            },
        );
    }
    snapshot
}

/// 构建**纯值** quota 快照（包装成 `SharedQuota` 归调用方）。
///
/// 桶键 = token 的 UUID `meta.key`（与 `QuotaGate` 查询键 `TokenInfo.id`
/// 一致——预检与扣费同桶）。
///
/// **值 = min(token 剩余限额, 用户货币可用值)** —— 两层额度都要卡：
/// - token 层（`api_tokens.quota - used_quota`）：令牌自己的独立限额，
///   同一用户可以给不同 token 配不同上限（new-api token quota 语义）；
/// - user 层（`user_balances × internal_rate` 求和，#179 多货币）：真实
///   资金，同一用户所有 token 共享。
///
/// 取小值的理由：token 限额 500 但用户只剩 100 → 只能花 100；用户有 5000
/// 但该 token 限 500 → 只能花 500。任一层不足都该拦。
///
/// `token.unlimited_quota = true`：跳过 token 层，只受用户余额约束
/// （不限额 ≠ 不要钱；要完全免费需给用户灌足余额或单独建免费货币）。
/// 用户不在 `user_quotas`（没充值）→ 0（prehold 恒拦截）。
fn build_quota_snapshot(
    token_records: &[TokenRecord],
    user_quotas: &HashMap<String, i64>,
) -> QuotaSnapshot {
    let quota_snapshot = QuotaSnapshot::default();

    for token in token_records {
        let user_available = user_quotas.get(&token.user_key).copied().unwrap_or(0);
        let remaining = if token.unlimited_quota {
            user_available
        } else {
            let token_left = (token.quota - token.used_quota).max(0);
            token_left.min(user_available)
        };
        quota_snapshot.upsert(token.meta.key.clone(), remaining);
    }

    quota_snapshot
}

/// 运行时快照集合：boot 产出、reload 原地热更。
///
/// `token/user/quota/group_snapshot` 是 `Arc<ArcSwap<T>>`：reload 向**同一实例** store
/// 新值，gate 与计费 sink 等所有持有者自动看到新数据。
/// `dispatch` 只是喂给 `Dispatcher::new` 的 boot 副本，**boot 后即陈旧**——
/// reload 走 `Dispatcher::set_snapshot` 原地换新，不回写本字段；运行期以
/// Dispatcher 内部快照为准（详见 [`reload_snapshots`] 文档）。
/// `price_rows` / `name_directory` / `channel_names` 是计费快照
/// （`Arc<ArcSwap<T>>`，reload 换新）：消费方 [`crate::billing::PgPriceTable`] /
/// [`crate::billing::PgSettleSink`] 持句柄现读，改价与渠道改名都免重启生效。
#[derive(Debug, Clone)]
pub struct Snapshots {
    pub dispatch: DispatchSnapshot,
    pub token_snapshot: SharedTokenSnapshot,
    pub user_snapshot: SharedUserSnapshot,
    pub quota_snapshot: SharedQuota,
    pub group_snapshot: SharedGroupSnapshot,
    /// 模型单价行（计费价格表数据源，reload 换新）。
    pub price_rows: SharedPriceRows,
    /// 用户/令牌展示名目录（reload 换新；sink submit 时现读）。
    pub name_directory: SharedNameDirectory,
    /// 渠道 UUID 字符串 → 展示名（reload 换新；sink submit 时现读，
    /// 渠道改名免重启生效）。
    pub channel_names: SharedChannelNames,
}
