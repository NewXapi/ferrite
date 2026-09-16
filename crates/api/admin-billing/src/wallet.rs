//! 用户货币钱包 — 扣费/入账/冻结解冻/余额视图（平表直连 sqlx）。
//!
//! 表（0007 建，0011/0012 扩展）：`currency_defs` + `user_balances`（多货币系统）。
//! - `currency_defs.group_rates`（0011）：组名 → 倍率 JSONB，缺组/NULL 组 = 1.0。
//!   内部单位口径 = `amount × internal_rate × group_multiplier`（available 与
//!   deduct 同口径，见下）。
//! - `user_balances.frozen_amount`（0012）：冻结额度，
//!   `available = amount - frozen_amount`；冻结部分不参与可用折算、不可被扣减。
//! - 到期粒度在 `affiliate_rewards.frozen_until`（按奖励行，非按余额行），
//!   `thaw_frozen` 把已到期奖励行的额度从 frozen 搬回可用。
//!
//! 并发语义：
//! - 入账（credit_*）：`INSERT ... ON CONFLICT DO UPDATE amount = amount + EXCLUDED.amount`。
//!   单条语句原子：同一用户单货币的并发入账串行化在 (user_key,currency_code) 行锁上，
//!   各自叠加不丢失（READ COMMITTED 下 CAS 无 ABA 风险，PG 行级原子写）。
//! - 扣费（deduct_by_cost*）：`FOR UPDATE` 行锁读 + `UPDATE ... WHERE
//!   amount - frozen_amount >= n` CAS；余额不足时 clamp 到 0 并返回实扣
//!   （内部单位）而非 Err — 网关语义是「尽力扣，余账下请求准入拦截」。
//! - 跨多货币扣费在同一事务内按 internal_rate 从高到低顺序扣，原子提交。
//! - 解冻（thaw_frozen）：先 `FOR UPDATE` 锁到期的 affiliate_rewards 行（并发
//!   thaw 第二个事务重读谓词时 frozen_until 已被清 NULL，选不中 → 不双搬），
//!   再扣减 user_balances.frozen_amount（UPDATE 自带行锁，与扣费不构成交叉锁序
//!   死锁：thaw 锁序恒为 rewards→balances，deduct 只碰 balances）。

use axum::{Router, extract::State, http::HeaderMap, response::Json, routing::get};
use contract::api::billing::{UserBalanceDto, WalletView};
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

use crate::currency::BillingErr;

#[derive(Clone)]
pub struct WalletService {
    pool: PgPool,
}

impl WalletService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 折算综合可用值（内部单位 i64）：组倍率 + 冻结感知。
    ///
    /// `available = Σ floor((amount − frozen_amount) × internal_rate × mult)`，
    /// `mult = currency_defs.group_rates->>group`，缺组/None = 1.0。
    /// `group: Option<&str>` = 用户组（auth_users.group_id）；None = 不区分组
    /// （TODO(#188): apps 侧接线传真实 group_id 后可收紧为必填）。
    pub async fn available_i64(
        &self,
        user_key: Uuid,
        group: Option<&str>,
    ) -> Result<i64, BillingErr> {
        let rows = self.load_balances(user_key, group).await?;
        Ok(Self::available_of(&rows))
    }

    /// 从货币余额扣 cost_i64（内部单位，500_000 = $1），不区分组（倍率 1.0）。
    ///
    /// 冻结部分（frozen_amount）不可扣：可用口径 = `amount − frozen_amount`。
    ///
    /// TODO(#188): apps/api PgSettleSink 接线组参数后，此兼容入口并入
    /// [`deduct_by_cost_group`](Self::deduct_by_cost_group)。
    pub async fn deduct_by_cost(
        &self,
        user_key: Uuid,
        cost_i64: i64,
    ) -> Result<(i64, bool), BillingErr> {
        self.deduct_by_cost_group(user_key, cost_i64, None).await
    }

    /// 组倍率版扣费：`deduct_by_cost` + `(currency, group)` 差异化折算。
    ///
    /// 扣法：遍历用户启用货币（internal_rate 降序，大额优先减少扣减行数），
    /// 把 cost 按 `internal_rate × group_multiplier` 折算成该货币单位后
    /// `UPDATE ... WHERE amount - frozen_amount >= n` CAS。
    /// - 单货币（FREE rate=1、组缺省 1.0）即 `amount - cost_i64`。
    /// - 组倍率 < 1（折扣语义，如 vip 0.8）：同样内部单位要花**更多**货币单位
    ///   （`units = ceil(内部单位 / (rate × mult))`），与 available 同口径——
    ///   倍率既缩小可见余额、也放大单位消耗，两者必须一致，否则扣费能绕过
    ///   available 校验凭空印钱。
    /// - 不足：clamp 到 0，返回实扣内部单位 + `fully_deducted=false`。
    ///
    /// 返回 `(实扣内部单位, 是否扣够)`。
    pub async fn deduct_by_cost_group(
        &self,
        user_key: Uuid,
        cost_i64: i64,
        group: Option<&str>,
    ) -> Result<(i64, bool), BillingErr> {
        if cost_i64 <= 0 {
            return Ok((0, true));
        }
        let mut tx = self.pool.begin().await?;
        // 行级锁读：FOR UPDATE 串行化并发扣费（READ COMMITTED 下防两事务同读旧余额）。
        let rows = sqlx::query_as::<_, BalanceRow>(
            r#"
            SELECT ub.currency_code, ub.amount, ub.frozen_amount, cd.internal_rate,
                   COALESCE((cd.group_rates ->> $2)::float8, 1.0) AS group_mult
            FROM user_balances ub
            JOIN currency_defs cd ON cd.code = ub.currency_code
            WHERE ub.user_key = $1 AND cd.enabled AND cd.kind = 'points'
              AND ub.amount > ub.frozen_amount
            ORDER BY cd.internal_rate DESC
            FOR UPDATE
            "#,
        )
        .bind(user_key)
        .bind(group)
        .fetch_all(&mut *tx)
        .await?;

        let mut remaining = cost_i64;
        let mut total_deducted = 0i64;
        for row in &rows {
            if remaining <= 0 {
                break;
            }
            // 有效折算率 = internal_rate × 组倍率；可用单位排除冻结部分。
            // ponytail: 全仓货币换算走 f64（pricing::price_of 同款）；rate 为有限小数、
            // 金额 i64 上限内无浮点误差（单货币 rate=1 时纯整数运算）。
            // 升级路径：需精确十进制时改 NUMERIC + 定点，见 todo/billing-implementation.md 阶段 2。
            let eff_rate = row.internal_rate * row.group_mult;
            if eff_rate.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
                // 配置错（倍率 0/负/NaN）的货币不参与扣减，防除零/负扣。
                continue;
            }
            let spendable = (row.amount - row.frozen_amount).max(0);
            // 该货币可覆盖的内部单位（向下取整，余数留给下一货币）。
            let available_internal = (spendable as f64 * eff_rate).floor() as i64;
            let take_internal = remaining.min(available_internal);
            if take_internal <= 0 {
                continue;
            }
            // 折算回货币单位（向上取整，保证扣够 take_internal 内部单位）。
            let deduct_units = (take_internal as f64 / eff_rate).ceil() as i64;
            let n = sqlx::query(
                r#"
                UPDATE user_balances
                SET amount = amount - $1, updated_at = now()
                WHERE user_key = $2 AND currency_code = $3
                  AND amount - frozen_amount >= $1
                "#,
            )
            .bind(deduct_units)
            .bind(user_key)
            .bind(&row.currency_code)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if n == 1 {
                total_deducted += take_internal;
                remaining -= take_internal;
            }
        }
        tx.commit().await?;
        Ok((total_deducted, remaining == 0))
    }

    /// 兑换码入账 FREE（redeem 核销事务内调用 `credit_redeem_in_tx`；独立调用走 pool）。
    /// 幂等：ON CONFLICT DO UPDATE 叠加。返回入账后 FREE 余额（货币单位）。
    pub async fn credit_redeem(&self, user_key: Uuid, amount_i64: i64) -> Result<i64, BillingErr> {
        if amount_i64 <= 0 {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await?;
        let v = Self::credit_in_tx(&mut tx, user_key, "FREE", amount_i64).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// 事务内入账 FREE（redeem 的 CAS 核销与入账同事务原子提交）。
    /// 返回入账后 FREE 余额（货币单位）。
    pub async fn credit_redeem_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_key: Uuid,
        amount_i64: i64,
    ) -> Result<i64, BillingErr> {
        Self::credit_in_tx(tx, user_key, "FREE", amount_i64).await
    }

    /// 充值指定货币。金额语义 = 该货币自己的单位（非内部单位）。
    pub async fn credit_topup(
        &self,
        user_key: Uuid,
        currency_code: &str,
        amount: i64,
    ) -> Result<i64, BillingErr> {
        if amount <= 0 {
            return Ok(0);
        }
        if !Self::currency_enabled(&self.pool, currency_code).await? {
            return Err(BillingErr::BadRequest(format!(
                "currency {currency_code} not found or disabled"
            )));
        }
        let mut tx = self.pool.begin().await?;
        let v = Self::credit_in_tx(&mut tx, user_key, currency_code, amount).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// 拉人/活动奖励入账 FREE（kind 记日志）。立即可用，不冻结。
    pub async fn credit_reward(
        &self,
        kind: &str,
        user_key: Uuid,
        amount: i64,
    ) -> Result<i64, BillingErr> {
        if amount <= 0 {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await?;
        let v = Self::credit_in_tx(&mut tx, user_key, "FREE", amount).await?;
        tracing::info!(%user_key, %kind, amount, "reward credited to FREE");
        tx.commit().await?;
        Ok(v)
    }

    /// 冻结时长奖励入账 FREE：`amount` 与 `frozen_amount` 同加（可用不变），
    /// 并在 `affiliate_rewards` 写跟踪行（kind、inviter=用户、invitee NULL、
    /// frozen_until = now() + hours）——**跟踪行是 thaw 的到期依据，缺了它
    /// 冻结永远搬不回可用**，所以本方法同事务写两边。
    ///
    /// `frozen_hours` None/≤0 = 不冻结，退化为 [`credit_reward`]（不写跟踪行）。
    /// 返回入账后 FREE 余额（货币单位，含冻结）。
    pub async fn credit_reward_frozen(
        &self,
        kind: &str,
        user_key: Uuid,
        amount: i64,
        frozen_hours: Option<i32>,
    ) -> Result<i64, BillingErr> {
        if amount <= 0 {
            return Ok(0);
        }
        let Some(hours) = frozen_hours.filter(|h| *h > 0) else {
            return self.credit_reward(kind, user_key, amount).await;
        };
        let mut tx = self.pool.begin().await?;
        let v = Self::credit_reward_frozen_in_tx(&mut tx, user_key, amount).await?;
        sqlx::query(
            r#"
            INSERT INTO affiliate_rewards (key, inviter_key, invitee_key, kind, amount, frozen_until)
            VALUES ($1, $2, NULL, $3, $4, now() + make_interval(hours => $5))
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_key)
        .bind(kind)
        .bind(amount)
        .bind(hours)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        tracing::info!(%user_key, %kind, amount, hours, "frozen reward credited to FREE");
        Ok(v)
    }

    /// 事务内冻结入账（调用方自己写带 frozen_until 的 affiliate_rewards 审计行
    /// 时用它——如 [`AffiliateService::reward_invite_referral`]，避免跟踪行重复）。
    /// 返回入账后 FREE 余额（货币单位，含冻结）。
    pub async fn credit_reward_frozen_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_key: Uuid,
        amount: i64,
    ) -> Result<i64, BillingErr> {
        let v = Self::credit_in_tx(tx, user_key, "FREE", amount).await?;
        sqlx::query(
            r#"
            UPDATE user_balances
            SET frozen_amount = frozen_amount + $1, updated_at = now()
            WHERE user_key = $2 AND currency_code = 'FREE'
            "#,
        )
        .bind(amount)
        .bind(user_key)
        .execute(&mut **tx)
        .await?;
        Ok(v)
    }

    /// 解冻该用户全部**已到期**的冻结奖励：frozen_amount 搬回可用
    /// （amount 不变、frozen 减），并把跟踪行 frozen_until 清 NULL（幂等标记：
    /// 再跑一次选不中该行，不会双搬）。返回本次解冻的 FREE 货币单位数。
    ///
    /// 并发语义：第一步 `FOR UPDATE` 锁到期奖励行——并发双 thaw 的第二个事务
    /// 在 READ COMMITTED 下拿到锁后重评谓词（frozen_until 已 NULL）选不中行，
    /// 搬运只发生一次；user_balances 的扣减走算术 UPDATE（行锁天然串行），
    /// `GREATEST(.., 0)` 只防历史脏数据，正常路径 frozen ≥ 到期行合计。
    pub async fn thaw_frozen(&self, user_key: Uuid) -> Result<i64, BillingErr> {
        let mut tx = self.pool.begin().await?;
        let due: Vec<(Uuid, i64)> = sqlx::query_as(
            r#"
            SELECT key, amount FROM affiliate_rewards
            WHERE inviter_key = $1 AND frozen_until IS NOT NULL AND frozen_until <= now()
            FOR UPDATE
            "#,
        )
        .bind(user_key)
        .fetch_all(&mut *tx)
        .await?;
        if due.is_empty() {
            return Ok(0); // 无到期行，tx 回滚即退出
        }
        let total: i64 = due.iter().map(|(_, a)| a).sum();
        let keys: Vec<Uuid> = due.iter().map(|(k, _)| *k).collect();
        sqlx::query(
            r#"
            UPDATE user_balances
            SET frozen_amount = GREATEST(frozen_amount - $1, 0), updated_at = now()
            WHERE user_key = $2 AND currency_code = 'FREE'
            "#,
        )
        .bind(total)
        .bind(user_key)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE affiliate_rewards SET frozen_until = NULL WHERE key = ANY($1)")
            .bind(&keys)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        tracing::info!(%user_key, total, "frozen rewards thawed");
        Ok(total)
    }

    /// 各货币余额 + 折算可用 i64（组倍率 + 冻结感知；contract DTO 无冻结字段，
    /// 面向前端的逐行 frozenAmount 见 [`balance_view_json`](Self::balance_view_json)）。
    pub async fn balance_view(
        &self,
        user_key: Uuid,
        group: Option<&str>,
    ) -> Result<WalletView, BillingErr> {
        let rows = self.load_balances(user_key, group).await?;
        let available_i64 = Self::available_of(&rows);
        let items = rows
            .iter()
            .map(|b| UserBalanceDto {
                currency_code: b.currency_code.clone(),
                amount: b.amount,
                symbol: b.symbol.clone(),
            })
            .collect();
        Ok(WalletView {
            user_key: user_key.to_string(),
            aff_code: self.fetch_aff_code(user_key).await?,
            balances: items,
            available_i64,
        })
    }

    /// GET /api/user/wallet 响应体：contract WalletView 字段 + 每行 `frozenAmount`
    /// （JSON 层扩展，消费方 serde 忽略未知字段，向后兼容）。
    pub async fn balance_view_json(
        &self,
        user_key: Uuid,
        group: Option<&str>,
    ) -> Result<Value, BillingErr> {
        let rows = self.load_balances(user_key, group).await?;
        let available_i64 = Self::available_of(&rows);
        Ok(json!({
            "userKey": user_key.to_string(),
            "affCode": self.fetch_aff_code(user_key).await?,
            "balances": rows.iter().map(|b| json!({
                "currencyCode": b.currency_code,
                "symbol": b.symbol,
                "amount": b.amount,
                "frozenAmount": b.frozen_amount,
            })).collect::<Vec<_>>(),
            "availableI64": available_i64,
        }))
    }

    /// 用户邀请短码（`auth_users.aff_code`，0013）；NULL/未生成 = None。
    ///
    /// 与余额行分表而查：用户可能一行余额都没有（未 seed），JOIN 会丢短码，
    /// 所以独立按 PK 取（索引命中，一次查询）。
    async fn fetch_aff_code(&self, user_key: Uuid) -> Result<Option<String>, BillingErr> {
        Ok(sqlx::query_scalar("SELECT aff_code FROM auth_users WHERE key = $1")
            .bind(user_key)
            .fetch_optional(&self.pool)
            .await?)
    }

    /// 可用余额行（启用货币 × 组倍率），balance_view/available_i64 共用口径。
    async fn load_balances(
        &self,
        user_key: Uuid,
        group: Option<&str>,
    ) -> Result<Vec<BalanceRow>, BillingErr> {
        Ok(sqlx::query_as::<_, BalanceRow>(
            r#"
            SELECT ub.currency_code, ub.amount, ub.frozen_amount, cd.internal_rate,
                   cd.symbol,
                   COALESCE((cd.group_rates ->> $2)::float8, 1.0) AS group_mult
            FROM user_balances ub
            JOIN currency_defs cd ON cd.code = ub.currency_code AND cd.enabled
            WHERE ub.user_key = $1 AND cd.kind = 'points'
            ORDER BY ub.currency_code
            "#,
        )
        .bind(user_key)
        .bind(group)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Σ floor((amount − frozen) × rate × mult)；扣减/可用/视图三处同一实现。
    fn available_of(rows: &[BalanceRow]) -> i64 {
        rows.iter()
            .map(|b| {
                let spendable = (b.amount - b.frozen_amount).max(0);
                (spendable as f64 * b.internal_rate * b.group_mult).floor() as i64
            })
            .sum()
    }

    async fn credit_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_key: Uuid,
        currency_code: &str,
        amount: i64,
    ) -> Result<i64, BillingErr> {
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO user_balances (user_key, currency_code, amount)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_key, currency_code) DO UPDATE SET
                amount = user_balances.amount + EXCLUDED.amount,
                updated_at = now()
            RETURNING amount
            "#,
        )
        .bind(user_key)
        .bind(currency_code)
        .bind(amount)
        .fetch_one(&mut **tx)
        .await?;
        Ok(row.0)
    }

    /// 入账公共校验：货币存在、启用且 **kind='points'**（0014）。
    /// fiat 只是计价展示单位，进 user_balances 会造出"法币余额"语义污染，
    /// 所以 credit_topup 等 card 入账路径在此被拦。
    async fn currency_enabled(pool: &PgPool, code: &str) -> Result<bool, BillingErr> {
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM currency_defs WHERE code = $1 AND enabled AND kind = 'points')",
        )
        .bind(code)
        .fetch_one(pool)
        .await?)
    }
}

#[derive(Debug, Clone, FromRow)]
struct BalanceRow {
    currency_code: String,
    amount: i64,
    /// 冻结额度（0012）；available 口径 = amount − frozen_amount。
    frozen_amount: i64,
    internal_rate: f64,
    /// 该用户组对本货币的倍率（0011 group_rates，缺省 1.0），SQL 侧 COALESCE。
    group_mult: f64,
    /// 展示符号（0014 currency_defs.symbol）。
    symbol: String,
}

// ---------- axum 路由（对齐 redeem.rs 鉴权/err_json 约定）----------

#[derive(Clone)]
pub struct WalletAppState {
    pub svc: std::sync::Arc<WalletService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: WalletAppState) -> Router {
    Router::new()
        .route("/api/user/wallet", get(balance_view))
        .with_state(state)
}

type ErrResp = (axum::http::StatusCode, Json<serde_json::Value>);
fn err_json(e: impl Into<AuthError>) -> ErrResp {
    let e = e.into();
    (
        e.status(),
        Json(json!({ "code": e.code(), "message": e.to_string() })),
    )
}

/// 当前用户钱包 — GET /api/user/wallet（self）。
/// 组倍率按当前用户 group_id 生效（UserView.group）；响应每行含 frozenAmount。
async fn balance_view(
    State(s): State<WalletAppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    let u = bearer_user(&s.auth, &h).await.map_err(err_json)?;
    let user_key = Uuid::parse_str(&u.key).map_err(|_| err_json(AuthError::InvalidToken))?;
    let view = s
        .svc
        .balance_view_json(user_key, Some(u.group.as_str()))
        .await
        .map_err(err_json)?;
    Ok(Json(json!({ "wallet": view })))
}
