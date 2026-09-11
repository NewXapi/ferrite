//! admin-proxy —— 出口代理节点管理（`[[proxy_nodes]]` 的 DB 化写侧）。
//!
//! 表 `proxy_nodes` 与 gateway `ProxyNode` 一一对应：URL 解析、协议装配
//! （meow-config `parse_proxy`）都在 `gateway-proxy` 域；本 crate 只做
//! CRUD + 校验 + 测速端点 + 批量导入（见 [`subscription`]）。
//!
//! 安全：`url` 含凭据（userinfo / ss-vless query），**日志一律走掩码**；
//! DB 明文存储（与 `api_channels.keys` 的「密文或明文由 store 层决定」同语义，
//! 加密下沉到 store 层时一并处理）。

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;
use gateway_proxy::ProxySnapshot;
use gateway_proxy::node::ProxyNode;

pub mod subscription;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyNodeView {
    pub key: String,
    pub name: String,
    /// 掩码后的 URL（凭据不回传前端）：`scheme://***@host:port?***`
    pub url_masked: String,
    pub channel_keys: Vec<String>,
    pub priority: i32,
    pub enabled: bool,
    pub remark: String,
    pub created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    pub updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct ProxyNodeRow {
    key: Uuid,
    name: String,
    url: String,
    channel_keys: serde_json::Value,
    priority: i32,
    enabled: bool,
    remark: String,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

const COLS: &str =
    "key, name, url, channel_keys, priority, enabled, remark, created_at, updated_at";

fn row_to_view(r: ProxyNodeRow) -> ProxyNodeView {
    let channel_keys = r
        .channel_keys
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    ProxyNodeView {
        url_masked: mask_url(&r.url),
        channel_keys,
        key: r.key.to_string(),
        name: r.name,
        priority: r.priority,
        enabled: r.enabled,
        remark: r.remark,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

/// 代理 URL 掩码：userinfo 与 query（pbk/sid/密码）都含凭据，只留骨架。
fn mask_url(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(u) => {
            let auth = if u.username().is_empty() && u.password().is_none() {
                String::new()
            } else {
                "***@".to_string()
            };
            let port = u.port().map(|p| format!(":{p}")).unwrap_or_default();
            let query = if u.query().is_some() { "?***" } else { "" };
            format!(
                "{}://{}{}{}{}",
                u.scheme(),
                auth,
                u.host_str().unwrap_or("?"),
                port,
                query
            )
        }
        Err(_) => "***（URL 无法解析）***".to_string(),
    }
}

/// 从 DB 加载 enabled 节点 → 数据面 `ProxySnapshot`（M1-2 热更新入口）。
///
/// URL 解析失败/绑定空渠道的行跳过并 warn（与 config.toml 的
/// `build_proxy_snapshot` 同语义）；id 按行序连续编号（≥1，0 是直连哨兵）。
pub async fn load_proxy_snapshot(pool: &PgPool) -> Result<ProxySnapshot, sqlx::Error> {
    let rows: Vec<ProxyNodeRow> = sqlx::query_as(&format!(
        "SELECT {COLS} FROM proxy_nodes WHERE enabled = true ORDER BY priority DESC, created_at"
    ))
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for (idx, r) in rows.into_iter().enumerate() {
        let keys: Vec<String> = r
            .channel_keys
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let preview = mask_url(&r.url);
        if keys.is_empty() {
            tracing::warn!(url = %preview, "proxy_nodes 缺少 channel_keys，跳过");
            continue;
        }
        let mut node = match ProxyNode::parse_url(&r.url) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(url = %preview, error = %e, "proxy_nodes URL 解析失败，跳过");
                continue;
            }
        };
        node.id = (idx as i64).saturating_add(1);
        node.channel_keys = keys;
        node.priority = r.priority;
        out.push(node);
    }
    Ok(ProxySnapshot { nodes: out })
}

pub struct ProxyNodeService {
    pub(crate) pool: PgPool,
}

impl ProxyNodeService {
    /// 变更后从 DB 重建快照并原地 install——管理台改节点立即生效，不重启。
    pub async fn reload_into(&self, proxies: &gateway_proxy::ProxyManager) {
        match load_proxy_snapshot(&self.pool).await {
            Ok(snap) => proxies.install(snap),
            Err(e) => tracing::error!(error = %e, "proxy_nodes 快照重建失败，沿用旧快照"),
        }
    }
}

impl ProxyNodeService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, enabled_only: bool) -> Result<Vec<ProxyNodeView>, sqlx::Error> {
        let sql = if enabled_only {
            format!(
                "SELECT {COLS} FROM proxy_nodes WHERE enabled = true ORDER BY priority DESC, created_at"
            )
        } else {
            format!("SELECT {COLS} FROM proxy_nodes ORDER BY priority DESC, created_at")
        };
        let rows: Vec<ProxyNodeRow> = sqlx::query_as(&sql).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_view).collect())
    }

    /// 校验并创建。URL 必须能被 `ProxyNode::parse_url` 解析（协议白名单在此收紧）。
    pub async fn create(
        &self,
        name: &str,
        url: &str,
        channel_keys: &[String],
        priority: i32,
        remark: &str,
    ) -> Result<ProxyNodeView, ServiceError> {
        let node = validate(url, channel_keys).await?;
        // 渠道绑定引用完整性：channel_keys 必须是 api_channels 里真实存在的 name
        self.check_channels(channel_keys).await?;
        let key = Uuid::now_v7();
        let channels_json = serde_json::to_value(channel_keys).unwrap_or_default();
        let row: ProxyNodeRow = sqlx::query_as(&format!(
            "INSERT INTO proxy_nodes (key, name, url, channel_keys, priority, enabled, remark)
             VALUES ($1, $2, $3, $4, $5, true, $6) RETURNING {COLS}"
        ))
        .bind(key)
        .bind(name)
        .bind(url)
        .bind(channels_json)
        .bind(priority)
        .bind(remark)
        .fetch_one(&self.pool)
        .await?;
        let _ = node; // parse 已在校验中完成；node 仅为未来按协议写附加列预留
        Ok(row_to_view(row))
    }

    pub async fn update(
        &self,
        key: Uuid,
        req: &NodeRequest,
    ) -> Result<ProxyNodeView, ServiceError> {
        validate(&req.url, &req.channel_keys).await?;
        self.check_channels(&req.channel_keys).await?;
        let channels_json = serde_json::to_value(&req.channel_keys).unwrap_or_default();
        let row: ProxyNodeRow = sqlx::query_as(&format!(
            "UPDATE proxy_nodes SET name = $2, url = $3, channel_keys = $4, priority = $5,
             enabled = $6, remark = $7, updated_at = now()
             WHERE key = $1 RETURNING {COLS}"
        ))
        .bind(key)
        .bind(&req.name)
        .bind(&req.url)
        .bind(channels_json)
        .bind(req.priority)
        .bind(req.enabled)
        .bind(&req.remark)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(ServiceError::NotFound)?;
        Ok(row_to_view(row))
    }

    pub async fn delete(&self, key: Uuid) -> Result<(), ServiceError> {
        let n = sqlx::query("DELETE FROM proxy_nodes WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(ServiceError::NotFound);
        }
        Ok(())
    }

    /// 测速：URL 能否构造出可用的 meow 适配器（不真拨——真拨见 forward 测活）。
    /// 返回 (构造结果, 协议类型或错误说明)。
    pub fn probe_build(url: &str) -> Result<String, String> {
        let node = ProxyNode::parse_url(url).map_err(|e| format!("URL 解析失败: {e}"))?;
        match gateway_proxy::adapter_for(&node) {
            Some(a) => Ok(a.adapter_type().to_string()),
            None => {
                // None = http/socks5（reqwest 路径，合法）或配置错误
                match node.scheme {
                    gateway_proxy::ProxyScheme::Http | gateway_proxy::ProxyScheme::Socks5 => {
                        Ok("reqwest".to_string())
                    }
                    gateway_proxy::ProxyScheme::Direct => {
                        Err("direct 节点没有出口适配器".to_string())
                    }
                    _ => Err("配置非法（见服务端 warn 日志）：回落直连".to_string()),
                }
            }
        }
    }

    /// channel_keys 引用完整性：每个名字都必须在 api_channels 中存在。
    async fn check_channels(&self, channel_keys: &[String]) -> Result<(), ServiceError> {
        if channel_keys.is_empty() {
            return Err(ServiceError::BadRequest(
                "channel_keys 不能为空（空 = 永远不会被任何渠道选中）".into(),
            ));
        }
        for name in channel_keys {
            let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_channels WHERE name = $1")
                .bind(name)
                .fetch_one(&self.pool)
                .await?;
            if n.0 == 0 {
                return Err(ServiceError::BadRequest(format!("channel `{name}` 不存在")));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRequest {
    #[serde(default)]
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub channel_keys: Vec<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub remark: String,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("{0}")]
    BadRequest(String),
    #[error("proxy node not found")]
    NotFound,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

/// URL 校验：能解析 + 协议在白名单 + 绑定渠道非空。
async fn validate(url: &str, channel_keys: &[String]) -> Result<ProxyNode, ServiceError> {
    if channel_keys.is_empty() {
        return Err(ServiceError::BadRequest(
            "channel_keys 不能为空（空 = 永远不会被任何渠道选中）".into(),
        ));
    }
    ProxyNode::parse_url(url).map_err(|e| ServiceError::BadRequest(e.to_string()))
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct ProxyNodeAppState {
    pub svc: std::sync::Arc<ProxyNodeService>,
    pub auth: std::sync::Arc<AuthService>,
    /// 数据面共享的 ProxyManager：CRUD 变更后原地 install（热更新）。
    pub proxies: std::sync::Arc<gateway_proxy::ProxyManager>,
}

pub fn router(state: ProxyNodeAppState) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/proxy_nodes", get(list).post(create))
        .route(
            "/api/proxy_nodes/{key}",
            axum::routing::put(update).delete(remove),
        )
        .route("/api/proxy_nodes/{key}/probe", post(probe))
        .route("/api/proxy_nodes/subscription", post(import_subscription))
        .route("/api/proxy_nodes/batch", post(import_share_links))
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

type ErrResp = (StatusCode, Json<Value>);
fn err_json(e: AuthError) -> ErrResp {
    (
        e.status(),
        Json(json!({ "code": e.code(), "message": e.to_string() })),
    )
}

fn svc_err(e: ServiceError) -> ErrResp {
    let status = match &e {
        ServiceError::BadRequest(_) | ServiceError::NotFound => StatusCode::BAD_REQUEST,
        ServiceError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({ "code": status.as_u16(), "message": e.to_string() })),
    )
}

async fn list(State(s): State<ProxyNodeAppState>, h: HeaderMap) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let items = s
        .svc
        .list(false)
        .await
        .map_err(|e| svc_err(ServiceError::Db(e)))?;
    Ok(Json(json!({ "items": items })))
}

async fn create(
    State(s): State<ProxyNodeAppState>,
    h: HeaderMap,
    Json(req): Json<NodeRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let v = s
        .svc
        .create(
            &req.name,
            &req.url,
            &req.channel_keys,
            req.priority,
            &req.remark,
        )
        .await
        .map_err(svc_err)?;
    Ok(Json(json!(v)))
}

async fn update(
    State(s): State<ProxyNodeAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<NodeRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key =
        Uuid::parse_str(&key).map_err(|_| err_json(AuthError::BadRequest("invalid key".into())))?;
    let v = s.svc.update(key, &req).await.map_err(svc_err)?;
    s.svc.reload_into(&s.proxies).await;
    Ok(Json(json!(v)))
}

async fn remove(
    State(s): State<ProxyNodeAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key =
        Uuid::parse_str(&key).map_err(|_| err_json(AuthError::BadRequest("invalid key".into())))?;
    s.svc.delete(key).await.map_err(svc_err)?;
    s.svc.reload_into(&s.proxies).await;
    Ok(Json(json!({ "deleted": true })))
}

/// 测速（不真拨）：URL 能否装配出可用适配器 + 协议类型。
async fn probe(
    State(s): State<ProxyNodeAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key =
        Uuid::parse_str(&key).map_err(|_| err_json(AuthError::BadRequest("invalid key".into())))?;
    let row: Option<(String,)> = sqlx::query_as("SELECT url FROM proxy_nodes WHERE key = $1")
        .bind(key)
        .fetch_optional(&s.svc.pool)
        .await
        .map_err(|e| svc_err(ServiceError::Db(e)))?;
    let url = row
        .ok_or_else(|| err_json(AuthError::NotFound("proxy node not found".into())))?
        .0;
    match ProxyNodeService::probe_build(&url) {
        Ok(kind) => Ok(Json(json!({ "ok": true, "adapterType": kind }))),
        Err(e) => Ok(Json(json!({ "ok": false, "message": e }))),
    }
}

/// 订阅导入：拉订阅 URL 或吃粘贴的 YAML，批量入库并热更新。
async fn import_subscription(
    State(s): State<ProxyNodeAppState>,
    h: HeaderMap,
    Json(req): Json<subscription::ImportRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let _ = (&s, &req);
    todo!("TODO(#111): 调 subscription::import_subscription 后 reload_into 并回 ImportReport")
}

/// 分享链接批量导入：粘贴多行 `vless://` / `vmess://` / `ss://`…
async fn import_share_links(
    State(s): State<ProxyNodeAppState>,
    h: HeaderMap,
    Json(req): Json<subscription::ImportRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let _ = (&s, &req);
    todo!("TODO(#111): 调 subscription::import_share_links 后 reload_into 并回 ImportReport")
}
