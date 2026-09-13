//! 用户货币钱包 — 扣费/三个发钱入口/余额视图（平表直连 sqlx）。
//!
//! 表（0007 建）：`currency_defs` + `user_balances`（多货币系统）。
//! - 入账（credit_*）：`INSERT ... ON CONFLICT DO UPDATE amount = user_balances.amount + EXCLUDED.amount`。
//!   单条语句原子：同一用户单货币的并发入账串行化在 (user_key,currency_code) 行锁上，
//!   各自叠加不丢失（READ COMMITTED 下 CAS 无 ABA 风险，PG 行级原子写）。
//! - 扣费（deduct_by_cost）：`UPDATE ... WHERE amount >= $n` CAS 行锁，
//!   并发扣费只会扣到实际可用余额，不会把金额打到负数；
//!   余额不足时 clamp 到 0 并返回实扣（内部单位）而非 Err — 网关语义是「尽力扣，余账下请求准入拦截」。
//! - 跨多货币扣费在同一事务内按 internal_rate 从高到低顺序扣，原子提交。

use axum::{Router, extract::State, http::HeaderMap, response::Json, routing::get};
use contract::api::billing::{UserBalanceDto, WalletView};
use serde_json::json;
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

    /// 从货币余额扣 cost_i64（内部单位，500_000 = $1）。
    ///
    /// 扣法：遍历用户启用货币（internal_rate 降序，大额优先减少扣减行数），
    /// 把 cost 按 internal_rate 折算成该货币单位后 `UPDATE ... WHERE amount >= n` CAS。
    /// - 单货币（FREE rate=1）即 `amount - cost_i64`。
    /// - 不足：clamp 到 0，返回实扣内部单位 + `fully_deducted=false`（调用方决定 402/记账缺口）。
    ///
    /// 返回 `(实扣内部单位, 是否扣够)`。
    pub async fn deduct_by_cost(
        &self,
        user_key: Uuid,
        cost_i64: i64,
    ) -> Result<(i64, bool), BillingErr> {
        if cost_i64 <= 0 {
            return Ok((0, true));
        }
        let mut tx = self.pool.begin().await?;
        // 行级锁读：FOR UPDATE 串行化并发扣费（READ COMMITTED 下防两事务同读旧余额）。
        let rows = sqlx::query_as::<_, BalanceRow>(
            r#"
            SELECT ub.currency_code, ub.amount, cd.internal_rate
            FROM user_balances ub
            JOIN currency_defs cd ON cd.code = ub.currency_code
            WHERE ub.user_key = $1 AND cd.enabled AND ub.amount > 0
            ORDER BY cd.internal_rate DESC
            FOR UPDATE
            "#,
        )
        .bind(user_key)
        .fetch_all(&mut *tx)
        .await?;

        let mut remaining = cost_i64;
        let mut total_deducted = 0i64;
        for row in &rows {
            if remaining <= 0 {
                break;
            }
            // 该货币可覆盖的内部单位（向下取整，余数留给下一货币）。
            // ponytail: 全仓货币换算走 f64（pricing::price_of 同款）；rate 为有限小数、
            // 金额 i64 上限内无浮点误差（单货币 rate=1 时纯整数运算）。
            // 升级路径：需精确十进制时改 NUMERIC + 定点，见 todo/billing-implementation.md 阶段 2。
            let available_internal = (row.amount as f64 * row.internal_rate).floor() as i64;
            let take_internal = remaining.min(available_internal);
            if take_internal <= 0 {
                continue;
            }
            // 折算回货币单位（向上取整，保证扣够 take_internal 内部单位）。
            let deduct_units = (take_internal as f64 / row.internal_rate).ceil() as i64;
            let n = sqlx::query(
                r#"
                UPDATE user_balances
                SET amount = amount - $1, updated_at = now()
                WHERE user_key = $2 AND currency_code = $3 AND amount >= $1
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

    /// 拉人/活动奖励入账 FREE（kind 记日志，后续可扩展审计表/冻结额度）。
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

    /// 各货币余额 + 折算可用 i64（GET /api/user/wallet）。
    pub async fn balance_view(&self, user_key: Uuid) -> Result<WalletView, BillingErr> {
        let balances = sqlx::query_as::<_, BalanceRow>(
            r#"
            SELECT ub.currency_code, ub.amount, cd.internal_rate
            FROM user_balances ub
            JOIN currency_defs cd ON cd.code = ub.currency_code
            WHERE ub.user_key = $1 AND cd.enabled
            ORDER BY ub.currency_code
            "#,
        )
        .bind(user_key)
        .fetch_all(&self.pool)
        .await?;

        let mut items: Vec<UserBalanceDto> = Vec::new();
        let mut available_i64 = 0i64;
        for b in balances {
            available_i64 += (b.amount as f64 * b.internal_rate).floor() as i64;
            items.push(UserBalanceDto {
                currency_code: b.currency_code.clone(),
                amount: b.amount,
            });
        }
        Ok(WalletView {
            user_key: user_key.to_string(),
            balances: items,
            available_i64,
        })
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

    async fn currency_enabled(pool: &PgPool, code: &str) -> Result<bool, BillingErr> {
        Ok(sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM currency_defs WHERE code = $1 AND enabled)",
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
    internal_rate: f64,
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
async fn balance_view(
    State(s): State<WalletAppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    let u = bearer_user(&s.auth, &h).await.map_err(err_json)?;
    let user_key = Uuid::parse_str(&u.key).map_err(|_| err_json(AuthError::InvalidToken))?;
    let view = s.svc.balance_view(user_key).await.map_err(err_json)?;
    Ok(Json(json!({ "wallet": view })))
}
