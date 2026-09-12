//! Ferrite — 计费权威装配（billing authority）
//!
//! # 权威裁决
//! **pipeline 结算是唯一扣费写点。** `forward::ForwardStage::with_price_table`
//! （#146）在流的唯一提交点（`commit_forwarded`）产出
//! [`contract::records::UsageEventRecord`] 交给本模块的 [`PgSettleSink`] 落地：
//! 流式（读完 200 / 中途出错 500）与非流式口径一致，事件在且只在提交点产出一次。
//!
//! 原 `apps/api/src/usage.rs` 用量中间件已**退役并整文件删除**，原因：
//! 1. **双写双扣**：中间件自己按 `cost = prompt + completion`（裸 token 计数）
//!    写 usage_logs + 扣 `used_quota`，与 pipeline sink 各记一笔、扣两次；
//! 2. **SSE 吞流**：中间件 `to_bytes(usize::MAX)` 缓冲整个响应体，把流式响应
//!    吞成末尾一次性返回，破坏 SSE 逐事件下发（pipeline 的扫描链是边透传
//!    边计数的正确姿势，不缓冲业务体）。
//!
//! # 定价
//! 价格源 = `model_prices` 表（迁移 0003：input/output/cache，单位 $/M tokens）。
//! 未知模型 → [`PriceTable::lookup`] 返回 `None` → 库层按缺价免费落账
//! （对齐 new-api 缺价 = 0 语义）。组倍率不落价格行：`api_groups.ratio` 经
//! [`gateway_gate::snapshot::GroupSnapshot::multiplier`] 在 [`PgPriceTable::lookup`]
//! 里折算进返回值的 `group_multiplier`；forward 库层传的 `group_ratio` 保持 1.0。
//!
//! # reload 陈旧性（Suspect）
//! [`PgPriceTable`] 持有的价格 HashMap 与 [`PgSettleSink`] 持有的渠道名映射
//! 都是 boot 时 clone 的快照：`Snapshots.price_rows` / `name_directory` 会随
//! reload 换新（名单目录是共享句柄，submit 时现读，改名即生效），但价格表
//! 不随 reload 换 —— ArcSwap 化留待后续（PR Suspect 已注明）。

use std::collections::HashMap;
use std::sync::Arc;

use contract::records::{TokenRecord, UsageEventRecord, UserRecord};
use gateway_gate::snapshot::{SharedGroupSnapshot, SharedQuota};
use metering::SettleSink;
use metering::pricing::{ModelPrice, PriceTable};

use crate::PgPool;

/// [`NameDirectory`] 的共享句柄（`Arc<ArcSwap<T>>`）：reload 向同一实例
/// store 新名单后，持句柄的 [`PgSettleSink`] 下次 submit 即读到新数据。
pub type SharedNameDirectory = Arc<arc_swap::ArcSwap<NameDirectory>>;

// ============================================================================
// PgPriceTable — model_prices 表 + 组倍率折算
// ============================================================================

/// PG 价格表 + 组倍率折算的 [`PriceTable`] 实现。
///
/// 价格行来自 `model_prices`（boot 时经 [`crate::snapshot::load_model_prices`]
/// 载入）；组倍率来自 `api_groups.ratio`（#159 已装进组快照）。
pub struct PgPriceTable {
    by_model: HashMap<String, ModelPrice>,
    groups: SharedGroupSnapshot,
}

impl PgPriceTable {
    /// 从 `model_prices` 行（`(model, input, output, cache)`，$/M tokens）与
    /// 组快照构建。
    ///
    /// `group_multiplier` 基值恒 1.0：迁移 0003 起分组倍率唯一来源是
    /// `api_groups.ratio`（见该迁移头注释「双轨断裂收敛为单轨」），
    /// 在 [`PriceTable::lookup`] 时经组快照折算，不在价格行重复存放。
    pub fn new(rows: &[(String, f64, f64, f64)], groups: SharedGroupSnapshot) -> Self {
        let by_model = rows
            .iter()
            .map(|(model, input, output, cache)| {
                (
                    model.clone(),
                    ModelPrice {
                        input: *input,
                        output: *output,
                        cache: *cache,
                        group_multiplier: 1.0,
                    },
                )
            })
            .collect();
        Self { by_model, groups }
    }
}

impl PriceTable for PgPriceTable {
    /// 查 `(model, group)` 价：命中 → 价格 clone 并把组倍率乘进
    /// `group_multiplier`；未命中 → `None`（库层按缺价免费落账）。
    ///
    /// 组不存在时 [`gateway_gate::snapshot::GroupSnapshot::multiplier`] 回落
    /// 中性 1.0，不放大也不拒绝计费。
    fn lookup(&self, model: &str, group: &str) -> Option<ModelPrice> {
        let mut price = *self.by_model.get(model)?;
        price.group_multiplier *= self.groups.load().multiplier(group);
        Some(price)
    }
}

// ============================================================================
// NameDirectory — UUID 字符串 → 展示名
// ============================================================================

/// 用户/令牌显示名目录：UUID 字符串 → 展示名（usage_logs 的冗余展示字段用）。
///
/// usage_logs 的 username / token_name 是**落库时快照**（用户改名不改历史
/// 日志，见 0002 迁移列注释），所以这里只做 O(1) 查表；名字随快照加载，
/// 查不到回落空串（与列默认值一致，不让整条账单失败）。
#[derive(Debug, Clone, Default)]
pub struct NameDirectory {
    by_user: HashMap<String, String>,
    by_token: HashMap<String, String>,
}

impl NameDirectory {
    /// 从 token / user 记录构建目录：`meta.key`（UUID 字符串）→ 展示名。
    pub fn new(token_records: &[TokenRecord], user_records: &[UserRecord]) -> Self {
        let by_user = user_records
            .iter()
            .map(|u| (u.meta.key.clone(), u.username.clone()))
            .collect();
        let by_token = token_records
            .iter()
            .map(|t| (t.meta.key.clone(), t.name.clone()))
            .collect();
        Self { by_user, by_token }
    }

    /// user UUID 字符串 → username；缺失回落空串。
    pub fn username(&self, user_key: &str) -> &str {
        self.by_user.get(user_key).map(String::as_str).unwrap_or("")
    }

    /// token UUID 字符串 → token 名；缺失回落空串。
    pub fn token_name(&self, token_key: &str) -> &str {
        self.by_token
            .get(token_key)
            .map(String::as_str)
            .unwrap_or("")
    }
}

// ============================================================================
// RecordJob / build_consume_event — 原 usage.rs 的落库载荷，原样搬移
// ============================================================================

/// 一次用量记录的全部载荷（打包成 struct 避免 13 参数函数）。
///
/// 对外公开只为让集成测试能直接喂 [`build_consume_event`]，不作为稳定 API。
/// （原属 `usage.rs` 中间件；中间件退役后由 [`PgSettleSink`] 从
/// `UsageEventRecord` 填充，字段语义保持不变，log_type 钉住回归见
/// `tests/usage_log_type.rs`。）
pub struct RecordJob {
    pub user_uuid: uuid::Uuid,
    pub username: String,
    /// token 的 UUID；`UsageEventRecord.token_key` 字符串解析失败时为 `None`，
    /// 账单仍落库但 usage_logs.token_key 记 NULL（不挂 token 维度统计），
    /// 与列可空语义一致。
    pub token_uuid: Option<uuid::Uuid>,
    pub token_name: String,
    pub model_name: String,
    /// 本请求实际命中的渠道（UUID 字符串）；pipeline 未带回（dispatch 前短路 /
    /// 响应体读失败前无 extensions）时为 None，落库为 NULL。
    pub channel_key: Option<String>,
    /// 命中渠道的展示名冗余；无归因时为空串，与列默认值一致。
    pub channel_name: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost: i64,
    pub use_time_ms: i32,
    pub is_stream: bool,
    pub token_key: String,
}

/// 把一次请求的用量载荷翻译成 observe 的 [`observe::logs::UsageEvent`]。
///
/// `log_type` 由 [`observe::logs::UsageEvent::consume`] 构造函数内部设为
/// [`observe::logs::LOG_TYPE_CONSUME`]（= 2），本函数不得手写字面量：观测侧的排行榜
/// `/api/log/top` 与趋势 `/api/log/trend` 都按 `log_type = 2` 过滤，这里曾手写
/// `log_type: 1`（1=充值，见 `db/migrations/0002_usage_logs.sql`），
/// 导致每条真实消费都被记成充值并从两个总览查询里整体消失。
///
/// `channel_key` / `channel_name` 来自 pipeline 结算事件的渠道归因；
/// 归因缺失（dispatch 前短路等）时留空，与列默认值一致。
/// `channel_key` 落库前解析成 UUID（`usage_logs.channel_key` 是 UUID 列），
/// 解析失败按缺失处理而不是让整条 INSERT 报类型错。
/// `ip` / `request_id` / `content` 同理留空。
pub fn build_consume_event(job: &RecordJob) -> observe::logs::UsageEvent {
    let mut event =
        observe::logs::UsageEvent::consume(job.user_uuid, &job.username, &job.model_name);
    event.token_key = job.token_uuid;
    event.token_name = job.token_name.clone();
    event.channel_key = job
        .channel_key
        .as_deref()
        .and_then(|k| uuid::Uuid::parse_str(k).ok());
    event.channel_name = job.channel_name.clone();
    // usage_logs 的 token 列是 i32（0002 迁移）；clamp 而非裸 as，异常大的计数
    // 截到 i32::MAX 而不是回绕成负数污染 sum 聚合。
    event.prompt_tokens = job.prompt_tokens.clamp(0, i32::MAX as i64) as i32;
    event.completion_tokens = job.completion_tokens.clamp(0, i32::MAX as i64) as i32;
    event.quota = job.cost;
    event.use_time_ms = job.use_time_ms;
    event.is_stream = job.is_stream;
    event
}

// ============================================================================
// PgSettleSink — pipeline 结算事件的 PG 落地
// ============================================================================

/// pipeline 结算事件的 PG 落地通道：写 usage_logs（权威账本）+ 增量维护
/// `api_tokens.used_quota`（缓存态）+ 扣内存 quota 快照（与 QuotaGate 同桶）。
///
/// 三处写点与退役的 usage 中间件完全一致——变了的只是**事件来源**：
/// 中间件靠缓冲响应体自算 usage（双写双扣），现在统一吃 pipeline
/// 在唯一提交点产出的 [`UsageEventRecord`]。
pub struct PgSettleSink {
    pool: PgPool,
    quota_snapshot: SharedQuota,
    /// 渠道 UUID 字符串 → 展示名（boot 渠道快照投影；与 `Snapshots.dispatch`
    /// 字段同款陈旧性，reload 不回写）。
    channel_names: HashMap<String, String>,
    names: SharedNameDirectory,
}

impl PgSettleSink {
    /// 组装 sink。`names` 传共享句柄：reload 换新名单后 submit 即读到新值。
    pub fn new(
        pool: PgPool,
        quota_snapshot: SharedQuota,
        channel_names: HashMap<String, String>,
        names: SharedNameDirectory,
    ) -> Self {
        Self {
            pool,
            quota_snapshot,
            channel_names,
            names,
        }
    }
}

impl SettleSink for PgSettleSink {
    /// 提交一条结算事件：热路径只做纯内存查表/解析，DB IO 全部 spawn 到后台
    /// （[`SettleSink`] 契约要求 submit 快速返回，不拖住转发管道）。
    fn submit(&self, event: UsageEventRecord) {
        // a) 无主账单不落库：user_key 必须是 UUID（gate 保证 token 归属真实
        //    用户；解析失败说明归因链有洞，宁可少记不可记脏账）。
        let user_uuid = match uuid::Uuid::parse_str(&event.user_key) {
            Ok(u) => u,
            Err(_) => {
                tracing::warn!(
                    user_key = %event.user_key,
                    model = %event.public_model,
                    cost = event.cost,
                    "usage event user_key is not a uuid; billing skipped"
                );
                return;
            }
        };
        // b) 从事件组装落库载荷：token UUID 解析失败 → None（账单仍落，
        //    只是不挂 token 维度，见 RecordJob::token_uuid 文档）。
        let token_uuid = uuid::Uuid::parse_str(&event.token_key).ok();
        let (username, token_name) = {
            let names = self.names.load();
            (
                names.username(&event.user_key).to_string(),
                names.token_name(&event.token_key).to_string(),
            )
        };
        let channel_name = self
            .channel_names
            .get(&event.channel_key)
            .cloned()
            .unwrap_or_default();
        let job = RecordJob {
            user_uuid,
            username,
            token_uuid,
            token_name,
            model_name: event.public_model.clone(),
            channel_key: Some(event.channel_key.clone()),
            channel_name,
            prompt_tokens: i64::try_from(event.prompt_tokens).unwrap_or(i64::MAX),
            completion_tokens: i64::try_from(event.completion_tokens).unwrap_or(i64::MAX),
            cost: event.cost,
            use_time_ms: i32::try_from(event.duration_ms).unwrap_or(i32::MAX),
            // UsageEventRecord 无流式标记字段；MVP 统一落 false——
            // usage_logs.is_stream 只是展示维度，计费口径不依赖它。
            is_stream: false,
            token_key: event.token_key.clone(),
        };
        let pool = self.pool.clone();
        let quota_snapshot = self.quota_snapshot.clone();
        tokio::spawn(async move {
            record_settlement(&pool, &quota_snapshot, job).await;
        });
    }
}

/// 后台落地：写 usage_logs → 增 `api_tokens.used_quota` → 扣内存 quota。
///
/// 任一步失败只 warn 不炸：usage_logs 是权威账本（写入失败有 warn 可追），
/// used_quota 是缓存态（可由账本重算），内存 quota 扣减失败影响的是下次
/// 预检精度，都不该让已经完成的转发请求报错。
async fn record_settlement(pool: &PgPool, quota_snapshot: &SharedQuota, job: RecordJob) {
    let event = build_consume_event(&job);
    let RecordJob {
        cost, token_key, ..
    } = job;
    let svc = observe::logs::LogService::new(pool.clone());
    match svc.record(&event).await {
        Ok(id) => tracing::debug!(usage_id = %id, "usage recorded"),
        Err(e) => tracing::warn!(error = %e, "failed to record usage"),
    }
    // api_tokens.key 是 UUID 列：必须绑 Uuid，绑 String 会类型不匹配导致 0 行更新
    match uuid::Uuid::parse_str(&token_key) {
        Ok(key_uuid) => {
            if let Err(e) =
                sqlx::query("UPDATE api_tokens SET used_quota = used_quota + $1 WHERE key = $2")
                    .bind(cost)
                    .bind(key_uuid)
                    .execute(pool)
                    .await
            {
                tracing::warn!(error = %e, "failed to update used_quota");
            }
        }
        Err(e) => tracing::warn!(error = %e, token_key = %token_key, "token key is not a uuid"),
    }
    // DB 用 UUID 主键，内存 quota 快照桶键同样是 token 的 UUID 字符串
    // （与 QuotaGate 查询键 TokenInfo.id 一致，见 snapshot::build_quota_snapshot）
    quota_snapshot.load().add(&token_key, -cost);
}
