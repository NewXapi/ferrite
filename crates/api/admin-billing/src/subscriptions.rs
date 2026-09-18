//! 订阅套餐 CRUD（平表直连 sqlx）。
//!
//! 表（迁移 0017 建）：`subscription_plans(key UUID PK, name UNIQUE, price TEXT,
//! currency, duration_days, quota BIGINT, upgrade_group, max_purchases,
//! enabled, sort_order)`。管理台「订阅」页的增删改查后端。
//!
//! ## 两套字段口径（风险点，务必读）
//!
//! 入库用 ferrite 形状的 [`SubscriptionUpsertRequest`]（name/price/currency/
//! duration_days/quota/upgrade_group/max_purchases/enabled），出库响应映射到
//! new-api 形状的 [`SubscriptionDto`]（`periodVal = duration_days`、
//! `periodUnit = "days"`、`group = upgrade_group`、`maxPerUser = max_purchases`，
//! 其余展示字段 None）。该映射语义与 contract 里已有的
//! `From<&SubscriptionPlanRecord> for SubscriptionDto` 一致，本模块直接
//! `Row → SubscriptionDto`（跳过 record 中转：record 没有 sort_order，
//! 直映才能把 sort_order 带到 DTO）。
//!
//! `SubscriptionDto` 是 new-api 形状（只有数字 `id`，无 UUID），而 DELETE
//! 路径要定位行，所以响应在 DTO 之上 flatten 补一个 `key`（见
//! [`SubscriptionView`]）。
//!
//! ## quota 口径
//!
//! 内部单位 500_000 = $1（同 [`crate::currency`]）；upsert 把请求的 f64
//! 展示值 ×500_000 四舍五入入库（[`quota_display_to_internal`]），读回侧
//! [`From<SubscriptionRow> for SubscriptionDto`] ÷500_000 还原展示值。
//! 两侧对称，API 往返 quota 不漂移（contract 的 DTO 注释明说 quota 是
//! 展示层口径）。

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
};
use contract::api::billing::{SubscriptionDto, SubscriptionUpsertRequest};
use serde::Serialize;
use serde_json::json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

use crate::currency::BillingErr;

/// 内部单位换算常数：500_000 = $1（new-api 语义，同 [`crate::currency`]）。
const UNITS_PER_DOLLAR: f64 = 500_000.0;

/// 订阅套餐服务：列表 / upsert / 删除（admin 控制面）。
pub struct SubscriptionService {
    pool: PgPool,
}

impl SubscriptionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 全量列表（套餐行数个位级，不分页），按 sort_order 升序。
    /// 同时返回总数，前端列表分页指示用。
    pub async fn list(&self) -> Result<(Vec<SubscriptionView>, i64), BillingErr> {
        let total: i64 = sqlx::query_scalar("SELECT count(*) FROM subscription_plans")
            .fetch_one(&self.pool)
            .await?;
        let rows: Vec<SubscriptionRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM subscription_plans ORDER BY sort_order ASC, created_at ASC"
        ))
        .fetch_all(&self.pool)
        .await?;
        Ok((rows.into_iter().map(row_to_view).collect(), total))
    }

    /// 新增/更新套餐（admin）：name 存在则更新（返回更新后的行），否则插入。
    ///
    /// 校验（非法输入 → [`BillingErr::BadRequest`]，绝不静默落库）：
    /// - name/currency 非空；
    /// - duration_days ≥ 1（0 天套餐无意义）；
    /// - price 能 parse 成 NUMERIC 且有限非负（TEXT 列的口径护栏）；
    /// - quota 经 [`quota_display_to_internal`] 换算，负值/溢出拒绝。
    ///
    /// sort_order：新行取 MAX+1（首个为 0）；命中既有 name 时不动位次。
    /// key 由应用侧生成（仓库迁移惯例，不依赖 gen_random_uuid）。
    pub async fn upsert(
        &self,
        req: &SubscriptionUpsertRequest,
    ) -> Result<SubscriptionView, BillingErr> {
        let name = req.name.trim();
        let currency = req.currency.trim();
        if name.is_empty() {
            return Err(BillingErr::BadRequest("name required".into()));
        }
        if currency.is_empty() {
            return Err(BillingErr::BadRequest("currency required".into()));
        }
        if req.duration_days == 0 {
            return Err(BillingErr::BadRequest("duration_days must be >= 1".into()));
        }
        // price 是 TEXT 列存 NUMERIC 语义：parse 失败/非有限/负值都说明调用方
        // 传了脏数据，返 BadRequest 而非把垃圾字符串写进库（风险点 3）。
        // 存 trim 后的**原字符串**而非 f64 的 to_string()：f64 回写会引入
        // 浮点漂移（如 "19.99" → 19.989999999999998），TEXT 列保原字面量最精确。
        // parse 的返回值只用于校验（非法即 BadRequest），入库用原字面量。
        let _price = parse_price(&req.price)?;
        let price_str = req.price.trim();
        // quota f64 展示口径 → i64 内部单位（×500_000 四舍五入，风险点 2）。
        let quota = quota_display_to_internal(req.quota)?;

        // u32 → INT 列：合法时长远小于 i32 域，夹到 i32::MAX 防溢出转换。
        let duration_days = req.duration_days.min(i32::MAX as u32) as i32;
        let max_purchases = req.max_purchases.map(|m| m.min(i32::MAX as u32) as i32);
        let enabled = req.enabled.unwrap_or(true);

        let row: SubscriptionRow = sqlx::query_as(&format!(
            r#"
            INSERT INTO subscription_plans
                (key, name, price, currency, duration_days, quota, upgrade_group, max_purchases, enabled, sort_order)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                    COALESCE((SELECT MAX(sort_order) + 1 FROM subscription_plans), 0))
            ON CONFLICT (name) DO UPDATE SET
                price = EXCLUDED.price,
                currency = EXCLUDED.currency,
                duration_days = EXCLUDED.duration_days,
                quota = EXCLUDED.quota,
                upgrade_group = EXCLUDED.upgrade_group,
                max_purchases = EXCLUDED.max_purchases,
                enabled = EXCLUDED.enabled,
                updated_at = now()
            RETURNING {COLS}
            "#
        ))
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(price_str)
        .bind(currency)
        .bind(duration_days)
        .bind(quota)
        .bind(req.upgrade_group.as_deref())
        .bind(max_purchases)
        .bind(enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row_to_view(row))
    }

    /// 删除套餐（admin）：UUID key，不存在 → [`BillingErr::NotFound`]。
    pub async fn delete(&self, key: &str) -> Result<(), BillingErr> {
        let key = Uuid::parse_str(key)
            .map_err(|_| BillingErr::BadRequest("invalid subscription key".into()))?;
        let n = sqlx::query("DELETE FROM subscription_plans WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(BillingErr::NotFound("subscription plan not found".into()));
        }
        Ok(())
    }
}

/// 私有行（与表一比一，`#[derive(FromRow)]`）。
#[derive(Debug, Clone, FromRow)]
struct SubscriptionRow {
    key: Uuid,
    name: String,
    price: String,
    currency: String,
    duration_days: i32,
    quota: i64,
    upgrade_group: Option<String>,
    max_purchases: Option<i32>,
    enabled: bool,
    sort_order: i32,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

const COLS: &str = "key, name, price, currency, duration_days, quota, upgrade_group, max_purchases, enabled, sort_order, created_at, updated_at";

/// 响应视图：new-api 形状的 [`SubscriptionDto`] 之上补表里有、DTO 表达不了的字段。
///
/// `SubscriptionDto` 只有数字 `id`（new-api 口径），而 DELETE 路径按 UUID key
/// 定位行——前端列表必须有 key 才能调删除；`currency` 决定价格符号（¥/$）；
/// 时间戳供管理台列表展示。flatten 使 JSON 是 DTO 的超集，不破坏 new-api
/// 形状的调用方。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionView {
    /// 行 key（UUID 字符串）——`DELETE /api/subscriptions/{key}` 的定位符。
    pub key: String,
    /// new-api 形状字段（contract 共享契约；sort_order 在此保留）。
    #[serde(flatten)]
    pub dto: SubscriptionDto,
    /// 计价货币（"CNY" | "USD"），决定价格展示符号。
    pub currency: String,
    /// 建立时间（RFC3339/ISO8601）。
    pub created_at: String,
    /// 最后修改时间（RFC3339/ISO8601）。
    pub updated_at: String,
}

fn row_to_view(r: SubscriptionRow) -> SubscriptionView {
    SubscriptionView {
        key: r.key.to_string(),
        // clone：currency 后面 r.into() 还要整行搬走，不 clone 会部分 move。
        currency: r.currency.clone(),
        created_at: r.created_at.to_rfc3339(),
        updated_at: r.updated_at.to_rfc3339(),
        dto: r.into(),
    }
}

/// 行 → new-api 形状 DTO（直映，不经 record 中转）。
///
/// 字段口径：`periodVal = duration_days`、`periodUnit = "days"`、
/// `group = upgrade_group`、`maxPerUser = max_purchases`——与 contract 里
/// `From<&SubscriptionPlanRecord> for SubscriptionDto` 逐字段一致；
/// 区别只在本侧多带 `sort_order`（record 没有该字段）。
///
/// quota 往返自洽：写入侧 [`quota_display_to_internal`] 把展示值 ×500_000
/// 入库，读回侧这里 ÷500_000 还原展示值（contract 的 DTO 注释明说 quota 是
/// 展示层口径）。两侧对称，API 往返 quota 不漂移。
impl From<SubscriptionRow> for SubscriptionDto {
    fn from(r: SubscriptionRow) -> Self {
        Self {
            id: None,
            name: r.name,
            description: None,
            price: r.price.parse().ok(),
            quota: Some(r.quota as f64 / UNITS_PER_DOLLAR),
            currency_price: None,
            payment_method: None,
            group: r.upgrade_group,
            downgrade_group: None,
            period_val: Some(r.duration_days as u32),
            period_unit: Some("days".into()),
            reset_cycle: None,
            priority: None,
            enabled: Some(r.enabled),
            allow_redeem: None,
            allow_wallet: None,
            max_per_user: r.max_purchases.map(|m| m as u32),
            sort_order: Some(r.sort_order as u32),
            stripe_price_id: None,
            creem_product_id: None,
            waffo_product_id: None,
        }
    }
}

/// price NUMERIC 语义字符串校验：parse 成 f64，要求有限且非负。
///
/// 返回值**仅用于校验合法性**（非法即 BadRequest，不把垃圾字符串写进 TEXT 列）；
/// 入库由调用方存 trim 后的**原字面量**（见 [`SubscriptionService::upsert`]）——
/// f64 的 `to_string()` 回写会引入浮点漂移（"19.99" → "19.989999999999998"），
/// TEXT 列保原字面量才精确。
pub fn parse_price(price: &str) -> Result<f64, BillingErr> {
    let v: f64 = price.trim().parse().map_err(|_| {
        BillingErr::BadRequest(format!("price must be a numeric string, got {price:?}"))
    })?;
    if !v.is_finite() || v < 0.0 {
        return Err(BillingErr::BadRequest(format!(
            "price must be finite and >= 0, got {v}"
        )));
    }
    Ok(v)
}

/// quota f64 展示口径 → i64 内部单位（×500_000，四舍五入）。
///
/// 负值/非有限值拒绝：非法输入静默成 0 会让运营改出负额度套餐。溢出
/// （超 i64 域）同样拒绝而非截断——截断会静默改写额度（风险点 2）。
///
/// 与读回侧对称：[`From<SubscriptionRow> for SubscriptionDto`] 里 quota
/// ÷500_000 还原展示值，API 往返不漂移。
pub fn quota_display_to_internal(quota: f64) -> Result<i64, BillingErr> {
    if !quota.is_finite() || quota < 0.0 {
        return Err(BillingErr::BadRequest(format!(
            "quota must be finite and >= 0, got {quota}"
        )));
    }
    let internal = quota * UNITS_PER_DOLLAR;
    if internal >= i64::MAX as f64 {
        return Err(BillingErr::BadRequest(format!(
            "quota {quota} overflows internal unit range"
        )));
    }
    Ok(internal.round() as i64)
}

// ---------- axum 路由（对齐 currency.rs 鉴权/err_json 约定）----------

#[derive(Clone)]
pub struct SubscriptionAppState {
    pub svc: std::sync::Arc<SubscriptionService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: SubscriptionAppState) -> axum::Router {
    Router::new()
        .route("/api/subscriptions", get(list).post(upsert))
        .route("/api/subscriptions/{key}", axum::routing::delete(remove))
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

/// 套餐列表 — GET /api/subscriptions（admin）。
async fn list(
    State(s): State<SubscriptionAppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let (items, total) = s.svc.list().await.map_err(err_json)?;
    Ok(Json(json!({ "items": items, "total": total })))
}

/// 新增/更新套餐 — POST /api/subscriptions（admin）。
async fn upsert(
    State(s): State<SubscriptionAppState>,
    h: HeaderMap,
    Json(req): Json<SubscriptionUpsertRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let view = s.svc.upsert(&req).await.map_err(err_json)?;
    Ok(Json(json!({ "subscription": view })))
}

/// 删除套餐 — DELETE /api/subscriptions/{key}（admin）。
///
/// handler 名取 `remove` 而非 `delete`：`axum::routing::delete` 与之同名会
/// 在模块作用域撞车（redeem.rs 同款处理）。
async fn remove(
    State(s): State<SubscriptionAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc.delete(&key).await.map_err(err_json)?;
    Ok(Json(json!({ "success": true })))
}
