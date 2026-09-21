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
//! # reload 热更
//! [`PgPriceTable`] 持有的价格行、[`PgSettleSink`] 持有的渠道名映射与
//! 用户/令牌名单目录都是 `Arc<ArcSwap<T>>` 共享句柄（`Snapshots.price_rows` /
//! `channel_names` / `name_directory`）：reload 向同一批句柄 store 新值后，
//! lookup / submit 现读即生效——**改价、渠道改名、用户改名都免重启**。

use std::collections::HashMap;
use std::sync::Arc;

use contract::records::{TokenRecord, UserRecord};
// 以下导入只在 billing feature 下被使用（PgPriceTable / PgSettleSink /
// record_settlement 耦合可选的 metering 与 admin-billing crate）；
// NameDirectory / RecordJob / build_consume_event 是纯本地类型，保持常编
// （snapshot.rs 的名单目录与事件构造依赖它们）。
#[cfg(feature = "billing")]
use contract::records::UsageEventRecord;
#[cfg(feature = "billing")]
use gateway_gate::snapshot::{SharedGroupSnapshot, SharedQuota};
#[cfg(feature = "billing")]
use metering::SettleSink;
#[cfg(feature = "billing")]
use metering::pricing::{ModelPrice, PriceTable};

#[cfg(feature = "billing")]
use crate::PgPool;

/// [`NameDirectory`] 的共享句柄（`Arc<ArcSwap<T>>`）：reload 向同一实例
/// store 新名单后，持句柄的 [`PgSettleSink`] 下次 submit 即读到新数据。
pub type SharedNameDirectory = Arc<arc_swap::ArcSwap<NameDirectory>>;

// ============================================================================
// PgPriceTable — model_prices 表 + 组倍率折算
// ============================================================================

#[cfg(feature = "billing")]
/// PG 价格表 + 组倍率折算的 [`PriceTable`] 实现。
///
/// 价格行来自 `model_prices`，持 [`crate::snapshot::SharedPriceRows`] 共享句柄：
/// reload store 新行后 lookup 即读到新价，**改价不需要重启**。
/// 组倍率来自 `api_groups.ratio`（#159 已装进组快照）。
pub struct PgPriceTable {
    rows: crate::snapshot::SharedPriceRows,
    groups: SharedGroupSnapshot,
}

#[cfg(feature = "billing")]
impl PgPriceTable {
    /// 从价格行共享句柄与组快照构建。
    ///
    /// `group_multiplier` 基值恒 1.0：迁移 0003 起分组倍率唯一来源是
    /// `api_groups.ratio`（见该迁移头注释「双轨断裂收敛为单轨」），
    /// 在 [`PriceTable::lookup`] 时经组快照折算，不在价格行重复存放。
    pub fn new(rows: crate::snapshot::SharedPriceRows, groups: SharedGroupSnapshot) -> Self {
        Self { rows, groups }
    }
}

#[cfg(feature = "billing")]
impl PriceTable for PgPriceTable {
    /// 查 `(model, group)` 价：命中 → 价格并把组倍率乘进 `group_multiplier`；
    /// 未命中 → `None`（库层按缺价免费落账）。
    ///
    /// 价格行是几十量级的小 vec，线性扫即可，免去每请求重建 HashMap。
    ///
    /// 组不存在时 [`gateway_gate::snapshot::GroupSnapshot::multiplier`] 回落
    /// 中性 1.0，不放大也不拒绝计费。
    fn lookup(&self, model: &str, group: &str) -> Option<ModelPrice> {
        // 热路径：两个 ArcSwap guard 只 load 一次（每次请求都会走这里）。
        let rows = self.rows.load();
        let groups = self.groups.load();
        let (_, input, output, cache) = rows.iter().find(|(m, ..)| m == model)?;
        let mut price = ModelPrice {
            input: *input,
            output: *output,
            cache: *cache,
            group_multiplier: 1.0,
        };
        price.group_multiplier *= groups.multiplier(group);
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
    /// 上游最终状态码；0 = 连接失败（`UsageEventRecord` 同名字段透传）。
    pub status_code: u16,
    /// 失败摘要；成功为 None。与 `status_code ≥ 400` 共同判定错误观测行
    /// （#166 的零成本结算事件）。
    pub error: Option<String>,
}

impl RecordJob {
    /// 错误观测行判定：上游已应答 ≥400 且带失败摘要。
    ///
    /// 成功但免费的行（cost=0、status=200）不落入此类，仍是 consume。
    pub fn is_error(&self) -> bool {
        self.status_code >= 400 && self.error.is_some()
    }
}

/// 把一次请求的用量载荷翻译成 observe 的 [`observe::logs::UsageEvent`]。
///
/// `log_type` 由 [`observe::logs::UsageEvent::consume`] /
/// [`observe::logs::UsageEvent::error`] 构造函数内部设为
/// [`observe::logs::LOG_TYPE_CONSUME`]（= 2）/ [`observe::logs::LOG_TYPE_ERROR`]
/// （= 5），本函数不得手写字面量：观测侧的排行榜 `/api/log/top` 与趋势
/// `/api/log/trend` 都按 `log_type = 2` 过滤，这里曾手写 `log_type: 1`
/// （1=充值，见 `db/migrations/0002_usage_logs.sql`），导致每条真实消费都被
/// 记成充值并从两个总览查询里整体消失。
///
/// [`RecordJob::is_error`]（上游 ≥400 且带失败摘要，#166 的零成本观测事件）
/// 决定骨架走 error() 还是 consume()；错误行的摘要进 `content` 列。
///
/// `channel_key` / `channel_name` 来自 pipeline 结算事件的渠道归因；
/// 归因缺失（dispatch 前短路等）时留空，与列默认值一致。
/// `channel_key` 落库前解析成 UUID（`usage_logs.channel_key` 是 UUID 列），
/// 解析失败按缺失处理而不是让整条 INSERT 报类型错。
/// `ip` / `request_id` 同理留空。
pub fn build_consume_event(job: &RecordJob) -> observe::logs::UsageEvent {
    let mut event = if job.is_error() {
        let mut e = observe::logs::UsageEvent::error(job.user_uuid, &job.username, &job.model_name);
        // 错误摘要进 content（列注释：扩展信息）；status 一并带上便于排障过滤
        e.content = format!(
            "upstream {}: {}",
            job.status_code,
            job.error.as_deref().unwrap_or("")
        );
        e
    } else {
        observe::logs::UsageEvent::consume(job.user_uuid, &job.username, &job.model_name)
    };
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

#[cfg(feature = "billing")]
/// pipeline 结算事件的 PG 落地通道：写 usage_logs（权威账本）+ 增量维护
/// `api_tokens.used_quota`（缓存态）+ 扣货币余额 `user_balances`（钱包层）+
/// 扣内存 quota 快照（与 QuotaGate 同桶，user 级折算值）。
///
/// 写点与退役的 usage 中间件一致，新增第 4 处货币扣费。变了的只是
/// **事件来源**：中间件靠缓冲响应体自算 usage（双写双扣），现在统一吃
/// pipeline 在唯一提交点产出的 [`UsageEventRecord`]。
pub struct PgSettleSink {
    pool: PgPool,
    quota_snapshot: SharedQuota,
    /// 渠道 UUID 字符串 → 展示名的共享句柄：reload store 新映射后，
    /// 下次 submit 现读即生效（渠道改名免重启）。
    channel_names: crate::snapshot::SharedChannelNames,
    names: SharedNameDirectory,
    /// 钱包服务：settle 时按 cost 扣 user 级货币余额（货币层唯一扣费写点）。
    wallet: billing::WalletService,
}

#[cfg(feature = "billing")]
impl PgSettleSink {
    /// 组装 sink。`names` / `channel_names` 传共享句柄：reload 换新后
    /// submit 即读到新值。`wallet` 与 admin-router 共享同一 PG 池即可
    /// （无共享可变状态，WalletService 每请求走 DB 事务）。
    pub fn new(
        pool: PgPool,
        quota_snapshot: SharedQuota,
        channel_names: crate::snapshot::SharedChannelNames,
        names: SharedNameDirectory,
        wallet: billing::WalletService,
    ) -> Self {
        Self {
            pool,
            quota_snapshot,
            channel_names,
            names,
            wallet,
        }
    }
}

#[cfg(feature = "billing")]
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
            .load()
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
            // is_stream 透传事件记录的流式意图（UsageEventRecord 自带该字段）；
            // usage_logs.is_stream 只是展示维度，计费口径不依赖它。
            is_stream: event.is_stream,
            token_key: event.token_key.clone(),
            status_code: event.status_code,
            error: event.error.clone(),
        };
        let pool = self.pool.clone();
        let quota_snapshot = self.quota_snapshot.clone();
        let wallet = self.wallet.clone();
        tokio::spawn(async move {
            record_settlement(&pool, &quota_snapshot, &wallet, job).await;
        });
    }
}

#[cfg(feature = "billing")]
/// 后台落地：写 usage_logs → 增 `api_tokens.used_quota` → 扣货币余额
/// `user_balances` → 扣内存 quota 快照（user 级）。
///
/// 任一步失败只 warn 不炸：usage_logs 是权威账本（写入失败有 warn 可追），
/// used_quota 是缓存态（可由账本重算），货币扣减失败有 warn 且快照同步扣
/// （余额由 center 侧 available_i64 校正），内存 quota 扣减失败影响的是下次
/// 预检精度，都不该让已经完成的转发请求报错。
async fn record_settlement(
    pool: &PgPool,
    quota_snapshot: &SharedQuota,
    wallet: &billing::WalletService,
    job: RecordJob,
) {
    let is_error = job.is_error();
    let event = build_consume_event(&job);
    let RecordJob {
        cost,
        token_key,
        user_uuid,
        ..
    } = job;
    let svc = observe::logs::LogService::new(pool.clone());
    match svc.record(&event).await {
        Ok(id) => tracing::debug!(usage_id = %id, "usage recorded"),
        Err(e) => tracing::warn!(error = %e, "failed to record usage"),
    }
    if is_error {
        // 错误观测行零成本：跳过 used_quota 递增与内存扣减，省一次 UPDATE。
        return;
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
    // 货币扣费（钱包层，#179 多货币 + #188 阶段 2 组倍率）：按
    // internal_rate × 用户组倍率折算后扣 user_balances。组取自 auth_users
    // （gate 侧快照的 group 同源）；查不到组的用户按 None（缺省 1.0）扣，
    // 不因组查询失败漏扣费。不足时 clamp 到 0 并返回实扣——网关语义是
    // "尽力扣，余账由下次请求的 prehold 拦截兜底"，不追讨已转发 token。
    // 用户分组是多值数组（迁移 0016）：生效分组取 groups[1]。
    let group: Option<String> = match sqlx::query_scalar::<_, Vec<String>>(
        "SELECT groups FROM auth_users WHERE key = $1",
    )
    .bind(user_uuid)
    .fetch_one(pool)
    .await
    {
        Ok(g) => g.into_iter().next(),
        Err(sqlx::Error::RowNotFound) => None,
        Err(e) => {
            tracing::warn!(error = %e, user_key = %user_uuid, "group lookup failed; deducting at default rate");
            None
        }
    };
    if cost > 0 {
        match wallet
            .deduct_by_cost_group(user_uuid, cost, group.as_deref())
            .await
        {
            Ok((deducted, fully)) => {
                if !fully {
                    tracing::warn!(
                        user_key = %user_uuid,
                        cost,
                        deducted,
                        "wallet balance insufficient; deducted as much as possible"
                    );
                }
            }
            Err(e) => tracing::warn!(error = %e, user_key = %user_uuid, "wallet deduction failed"),
        }
    }
    // 内存 quota 快照扣减：打在**本事件的 token 桶**上。
    //
    // 桶键必须是 token_key（QuotaGate 查询键 TokenInfo.id = token UUID；
    // build_quota_snapshot 按 token 建桶、灌用户级值）。曾误扣 user_key
    // 桶——那是无人读的桶，settle 后网关余额虚高到 reload 才校正（#187
    // 引入、本 PR 修复；CI 未抓到是 e2e 库不可达时测试整体 skip）。
    // 同用户其余 token 的桶不即时联动 = 既有 prehold 漂移语义（reload 收敛）。
    quota_snapshot.load().add(&token_key, -cost);
}
