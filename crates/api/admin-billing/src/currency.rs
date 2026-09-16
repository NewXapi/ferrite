//! 货币定义 + 用户货币余额 seed/折算（平表直连 sqlx）。
//!
//! 表（T1 迁移 0007 建）：
//! - `currency_defs(code PK, name, internal_rate DOUBLE, enabled, remark)`
//! - `user_balances(user_key, currency_code PK, amount BIGINT)`
//!
//! 折算综合可用值：`available_i64 = COALESCE(SUM(amount * internal_rate), 0)::BIGINT WHERE enabled`。
//! 内部单位 500_000 = $1（new-api 语义）；cost 即内部单位 i64。
//!
//! 并发语义：seed 用 `ON CONFLICT DO NOTHING`（幂等）；upsert_def 单事务内
//! `ON CONFLICT DO UPDATE`（行锁）+ 补 seed，原子提交。

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
};
use contract::api::billing::CurrencyView;
use rand::Rng;
use serde::Deserialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;
use serde_json::json;

use crate::affiliate::AffiliateService;
use crate::wallet::WalletService;

/// 货币服务：注册 seed / 折算可用值 / 定义管理。
pub struct CurrencyService {
    pool: PgPool,
}

/// 注册后置 hook（auth::routes::OnUserRegistered 实现）：
/// 新用户注册成功 → seed 全部启用货币（幂等，amount=0）；生成本人邀请短码
/// （aff_code，0013）；若注册请求带邀请码（UUID 或短码），解析校验后绑定
/// 邀请归属（affiliate_links）。全部 fire-and-forget（seed 一个 spawn，
/// 短码+归属另一个），任一失败不回灌注册失败。
///
/// 放在 billing 而非 auth：依赖方向 billing→auth，trait 由 auth 定义、
/// 本侧实现并经 admin-router 注入（#179 多货币；#197 邀请链接闭环）。
pub struct WalletSeedHook {
    pool: PgPool,
}

impl WalletSeedHook {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl auth::routes::OnUserRegistered for WalletSeedHook {
    fn on_registered(&self, user_key: uuid::Uuid, invite: Option<&str>) {
        // fire-and-forget：注册路径不该被货币层拖慢/拖死，seed 失败有
        // warn 可追，钱包首次入账时 available_i64 查无行按 0 兜底。
        let currency = CurrencyService::new(self.pool.clone());
        tokio::spawn(async move {
            if let Err(e) = currency.seed_for_user(user_key).await {
                tracing::warn!(error = %e, user_key = %user_key, "currency seed for new user failed");
            }
        });

        // 邀请短码 + 邀请归属：一个 spawn 两个旁路写，互不拖注册。二者彼此
        // 独立（归属查的是邀请人的码，与本人的码无关），任一失败都只是少
        // 一个旁路增益（钱/归属/短码都能事后补），不回灌注册失败。
        let pool = self.pool.clone();
        let invite = invite.map(str::to_owned);
        tokio::spawn(async move {
            if let Err(e) = generate_aff_code(&pool, user_key).await {
                tracing::warn!(error = %e, user_key = %user_key, "aff_code generation failed");
            }
            if let Some(invite) = invite
                && let Some(inviter) = resolve_invite_code(&pool, Some(&invite)).await
            {
                bind_invite_relation(&pool, inviter, user_key).await;
            }
        });
    }
}

/// 邀请码 → 邀请人 user_key 的 UUID 分支：`None`、空串、非 UUID 文本一律 `None`。
///
/// 旧链接 `?invite=<uuid>` 直传邀请人的 user_key；短码（aff_code）分支在
/// [`resolve_invite_code`]（async，查 `auth_users.aff_code`），注册链接
/// `?invite=<short>` 新格式走那条——本函数保持同步纯函数（DB-free）不变。
/// 解析失败静默丢弃：邀请是注册的旁路增益，脏输入/手改链接不该阻断账号
/// 创建（恶意输入见 `bind_invite_relation`：连 DB 都不会碰）。
pub fn parse_invite_code(invite: Option<&str>) -> Option<Uuid> {
    Uuid::parse_str(invite?).ok()
}

/// 邀请码 → 邀请人 user_key（双格式）：
/// - UUID（旧链接 `?invite=<uuid>`）：走 [`parse_invite_code`]，纯解析不碰 DB；
/// - aff_code 短码（新链接 `?invite=<short>`）：查 `auth_users.aff_code`（0013）。
///
/// 必须 async 且只在 spawn 里调——短码分支要碰 DB，而注册路径上的
/// [`parse_invite_code`] 契约是同步纯函数（脏输入连 DB 都不碰），两者分离。
/// 任一失败（空串/非 UUID/不存在的短码/DB 错）一律 None：邀请是注册的
/// 旁路增益，脏输入/手改链接不该阻断账号创建（语义同 parse_invite_code）。
pub async fn resolve_invite_code(pool: &PgPool, invite: Option<&str>) -> Option<Uuid> {
    if let Some(inviter) = parse_invite_code(invite) {
        return Some(inviter);
    }
    let code = invite?;
    if code.is_empty() {
        return None;
    }
    let inviter: Option<Uuid> = match sqlx::query_scalar(
        "SELECT key FROM auth_users WHERE aff_code = $1",
    )
    .bind(code)
    .fetch_optional(pool)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, code = %code, "aff_code lookup failed");
            return None;
        }
    };
    inviter
}

/// 邀请短码长度（base62）：6 位 ≈ 568 亿空间，不可枚举（todo 项 2）。
pub const AFF_CODE_LEN: usize = 6;
/// 撞唯一索引的重试上限（唯一索引是最后护栏，重试只在极端巧合下触发）。
const AFF_CODE_RETRIES: usize = 8;
const AFF_CODE_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// 随机 base62 短码（`gen_range` 无模偏差；形状/熵落点见 `tests/aff_code.rs`）。
pub fn random_base62(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| AFF_CODE_ALPHABET[rng.gen_range(0..AFF_CODE_ALPHABET.len())] as char)
        .collect()
}

/// 为用户生成邀请短码（幂等：已有码不覆盖；撞唯一索引换码重试）。
///
/// `WHERE aff_code IS NULL` 使 hook 重复触发/并发生成时 0 行更新 = 已有码，
/// 幂等成功；唯一索引冲突（与存量回填码或并发新码撞上）换码重来，超过
/// [`AFF_CODE_RETRIES`] 次（连续 8 次生日冲突，实际不可达）才报错。
pub async fn generate_aff_code(pool: &PgPool, user_key: Uuid) -> Result<(), BillingErr> {
    for _ in 0..AFF_CODE_RETRIES {
        let code = random_base62(AFF_CODE_LEN);
        match sqlx::query(
            "UPDATE auth_users SET aff_code = $1 WHERE key = $2 AND aff_code IS NULL",
        )
        .bind(&code)
        .bind(user_key)
        .execute(pool)
        .await
        {
            Ok(_) => return Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => continue,
            Err(e) => return Err(BillingErr::Db(e)),
        }
    }
    Err(BillingErr::BadRequest(format!(
        "aff_code generation: {AFF_CODE_RETRIES} retries exhausted on unique collision"
    )))
}

/// 校验邀请人存在后绑定归属（fire-and-forget 任务载荷）。
///
/// 全程静默，理由见 trait 文档：邀请人不存在（脏/过期邀请码）→ debug，
/// 这是可预期的脏数据，不值得 warn 污染日志；被邀人已归属他人 → warn，
/// 先到先得是正常竞态但值得追查是否有重复发奖；绑定失败 → warn。
/// 自邀请（inviter == invitee）由 `bind_inviter` 拒绝，此处不重复拦。
async fn bind_invite_relation(pool: &PgPool, inviter: Uuid, invitee: Uuid) {
    let exists: bool =
        match sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_users WHERE key = $1)")
            .bind(inviter)
            .fetch_one(pool)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, inviter = %inviter, "inviter existence check failed");
                return;
            }
        };
    if !exists {
        tracing::debug!(inviter = %inviter, "invite code references nonexistent user, skipping bind");
        return;
    }
    let svc = AffiliateService::new(pool.clone(), WalletService::new(pool.clone()));
    match svc.bind_inviter(inviter, invitee).await {
        Ok(true) => {
            tracing::debug!(inviter = %inviter, invitee = %invitee, "invite attribution bound")
        }
        Ok(false) => {
            tracing::warn!(inviter = %inviter, invitee = %invitee, "invite bind skipped: invitee already attributed to another inviter")
        }
        Err(e) => {
            tracing::warn!(error = %e, inviter = %inviter, invitee = %invitee, "invite bind failed")
        }
    }
}

/// 本域统一错误（wallet/affiliate/topup 复用；handler 边界转 AuthError 响应）。
#[derive(Debug, thiserror::Error)]
pub enum BillingErr {
    #[error("db error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<BillingErr> for AuthError {
    fn from(e: BillingErr) -> Self {
        match e {
            BillingErr::Db(e) => AuthError::Db(e),
            BillingErr::BadRequest(msg) => AuthError::BadRequest(msg),
            BillingErr::NotFound(msg) => AuthError::NotFound(msg),
        }
    }
}

impl CurrencyService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 为注册用户 seed 所有 enabled 的 **points** 货币（amount = 0）。
    /// fiat 货币（0014）只计价展示、不进余额，因此不 seed——否则新用户会
    /// 长出 ¥0/$0 的"法币余额"行，前端展示造成语义污染。
    /// 幂等：`ON CONFLICT DO NOTHING`，重复调用无副作用。
    pub async fn seed_for_user(&self, user_key: Uuid) -> Result<(), BillingErr> {
        sqlx::query(
            r#"
            INSERT INTO user_balances (user_key, currency_code, amount)
            SELECT $1, code, 0 FROM currency_defs WHERE enabled AND kind = 'points'
            ON CONFLICT (user_key, currency_code) DO NOTHING
            "#,
        )
        .bind(user_key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 折算综合可用值（内部单位 i64），喂 QuotaGate/快照。
    /// 仅 points 货币计入：fiat（0014）只是计价单位，不是可扣费余额。
    pub async fn available_i64(&self, user_key: Uuid) -> Result<i64, BillingErr> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(ub.amount * cd.internal_rate), 0)::BIGINT
            FROM user_balances ub
            JOIN currency_defs cd ON cd.code = ub.currency_code AND cd.enabled
            WHERE ub.user_key = $1 AND cd.kind = 'points'
            "#,
        )
        .bind(user_key)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// 货币定义列表（admin 查看）。
    pub async fn list_defs(&self) -> Result<Vec<CurrencyView>, BillingErr> {
        let rows = sqlx::query_as::<_, CurrencyDefRow>(
            "SELECT code, name, internal_rate, enabled, remark, symbol, kind, precision FROM currency_defs ORDER BY code",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// 新增/更新货币定义（admin）。
    /// 单事务：`ON CONFLICT DO UPDATE` 行锁；enabled 且 points 时顺带为缺余额行的
    /// 现有用户补 seed 0（幂等）。fiat 货币不补余额行（法币不进 user_balances）。
    ///
    /// 校验：kind=fiat 时 symbol 非空、precision ≥ 1（法币必有符号与小数位）；
    /// USD 是基准货币（internal_rate 恒为 1），改它会让全盘换算失真，直接拒绝。
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_def(
        &self,
        code: &str,
        name: &str,
        internal_rate: f64,
        enabled: bool,
        remark: &str,
        symbol: &str,
        kind: &str,
        precision: i16,
    ) -> Result<CurrencyView, BillingErr> {
        if code.trim().is_empty() {
            return Err(BillingErr::BadRequest("code required".into()));
        }
        let internal_rate = if internal_rate.is_finite() && internal_rate > 0.0 {
            internal_rate
        } else {
            return Err(BillingErr::BadRequest(
                "internal_rate must be finite and > 0".into(),
            ));
        };
        if kind != "points" && kind != "fiat" {
            return Err(BillingErr::BadRequest(format!(
                "kind must be 'points' or 'fiat', got {kind}"
            )));
        }
        if kind == "fiat" {
            if symbol.trim().is_empty() {
                return Err(BillingErr::BadRequest(
                    "fiat currency requires symbol".into(),
                ));
            }
            if precision < 1 {
                return Err(BillingErr::BadRequest(
                    "fiat currency requires precision >= 1".into(),
                ));
            }
        }
        if code == "USD" && internal_rate != 1.0 {
            // 基准货币 rate 被改 = 全盘换算口径漂移（换算全部经 internal 单位中转）。
            return Err(BillingErr::BadRequest(
                "USD is the base currency; internal_rate is locked at 1".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row: CurrencyDefRow = sqlx::query_as(
            r#"
            INSERT INTO currency_defs (code, name, internal_rate, enabled, remark, symbol, kind, precision)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (code) DO UPDATE SET
                name = EXCLUDED.name,
                internal_rate = EXCLUDED.internal_rate,
                enabled = EXCLUDED.enabled,
                remark = EXCLUDED.remark,
                symbol = EXCLUDED.symbol,
                kind = EXCLUDED.kind,
                precision = EXCLUDED.precision,
                updated_at = now()
            RETURNING code, name, internal_rate, enabled, remark, symbol, kind, precision
            "#,
        )
        .bind(code)
        .bind(name)
        .bind(internal_rate)
        .bind(enabled)
        .bind(remark)
        .bind(symbol)
        .bind(kind)
        .bind(precision)
        .fetch_one(&mut *tx)
        .await?;
        // 启用 points 货币：现有用户缺余额行则补 0（幂等）。
        // fiat 不补——法币只是计价单位，user_balances 只存 points 余额（0014 口径）。
        if enabled && kind == "points" {
            sqlx::query(
                r#"
                INSERT INTO user_balances (user_key, currency_code, amount)
                SELECT u.key, $1, 0 FROM auth_users u
                WHERE NOT EXISTS (
                    SELECT 1 FROM user_balances ub
                    WHERE ub.user_key = u.key AND ub.currency_code = $1
                )
                "#,
            )
            .bind(code)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(row.into())
    }

    /// 金额换算：`amount` 个 `from` 货币单位 → 多少个 `to` 货币单位。
    ///
    /// 经内部单位中转（500_000 = $1）：`internal = amount × rate(from)`，
    /// `result = internal / rate(to)`。两货币只要都注册且启用即可互换，
    /// 无需两两直配汇率（0014 通用换算层核心）。
    ///
    /// 舍入：floor（保守口径——展示/计价宁少勿多）；扣费入账需要向上取整的
    /// 场景由调用方自行 ceil（wallet.rs 的折算已各自带取整纪律）。
    ///
    /// 错误：`from`/`to` 不存在或未启用 → `BadRequest`；`from == to` 直接返回。
    pub async fn convert(&self, amount: i64, from: &str, to: &str) -> Result<i64, BillingErr> {
        if amount <= 0 {
            return Ok(0);
        }
        if from == to {
            return Ok(amount);
        }
        let rates: Vec<(String, f64)> = sqlx::query_as(
            "SELECT code, internal_rate FROM currency_defs WHERE code IN ($1, $2) AND enabled",
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        let rate_of = |code: &str| -> Result<f64, BillingErr> {
            rates
                .iter()
                .find(|(c, _)| c == code)
                .map(|(_, r)| *r)
                .ok_or_else(|| {
                    BillingErr::BadRequest(format!("currency {code} not found or disabled"))
                })
        };
        let rate_from = rate_of(from)?;
        let rate_to = rate_of(to)?;
        // f64 中转（同 wallet.rs deduct 口径）；rate 已由 upsert 校验 > 0 且有限，
        // 但防御 partial_cmp，配置脏数据不 panic。
        if rate_from.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater)
            || rate_to.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater)
        {
            return Err(BillingErr::BadRequest("invalid currency rate".into()));
        }
        let internal = (amount as f64) * rate_from;
        let converted = (internal / rate_to).floor();
        // i64 域防护：转换结果超 BIGINT 时夹到 i64::MAX（与 snapshot.rs LEAST 同语义）。
        if converted >= i64::MAX as f64 {
            return Ok(i64::MAX);
        }
        Ok(converted as i64)
    }
}

#[derive(Debug, Clone, FromRow)]
struct CurrencyDefRow {
    code: String,
    name: String,
    internal_rate: f64,
    enabled: bool,
    remark: String,
    symbol: String,
    kind: String,
    precision: i16,
}

impl From<CurrencyDefRow> for CurrencyView {
    fn from(r: CurrencyDefRow) -> Self {
        CurrencyView {
            code: r.code,
            name: r.name,
            internal_rate: r.internal_rate,
            enabled: r.enabled,
            remark: r.remark,
            symbol: r.symbol,
            kind: r.kind,
            precision: r.precision,
        }
    }
}

// ---------- axum 路由（对齐 redeem.rs 鉴权/err_json 约定）----------

#[derive(Clone)]
pub struct CurrencyAppState {
    pub svc: std::sync::Arc<CurrencyService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: CurrencyAppState) -> axum::Router {
    Router::new()
        .route("/api/currency", get(list_defs).post(upsert_def))
        .with_state(state)
}

async fn require_admin(auth: &AuthService, h: &HeaderMap) -> Result<(), AuthError> {
    let u = bearer_user(auth, h).await?;
    if u.role >= auth::routes::ADMIN_ROLE_THRESHOLD {
        Ok(())
    } else {
        Err(AuthError::Forbidden)
    }
}

type ErrResp = (StatusCode, Json<serde_json::Value>);
fn err_json(e: impl Into<AuthError>) -> ErrResp {
    let e = e.into();
    (
        e.status(),
        Json(json!({ "code": e.code(), "message": e.to_string() })),
    )
}

/// 货币定义列表 — GET /api/currency（admin）。
async fn list_defs(
    State(s): State<CurrencyAppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let defs = s.svc.list_defs().await.map_err(err_json)?;
    Ok(Json(json!({ "items": defs })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertDefRequest {
    code: String,
    name: String,
    internal_rate: f64,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    remark: String,
    #[serde(default)]
    symbol: String,
    /// points（默认）| fiat。
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    precision: i16,
}

/// kind 的 serde 默认值：存量调用方不传 kind 视为 points（向后兼容）。
fn default_kind() -> String {
    "points".to_string()
}

/// 新增/更新货币 — POST /api/currency（admin）。
async fn upsert_def(
    State(s): State<CurrencyAppState>,
    h: HeaderMap,
    Json(req): Json<UpsertDefRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let view = s
        .svc
        .upsert_def(
            &req.code,
            &req.name,
            req.internal_rate,
            req.enabled,
            &req.remark,
            &req.symbol,
            &req.kind,
            req.precision,
        )
        .await
        .map_err(err_json)?;
    Ok(Json(json!({ "currency": view })))
}
