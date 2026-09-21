//! Provider 配置与注册表。
//!
//! [`IdentityProvider`] 与 `db/migrations/0019_oidc_identities.sql` 的
//! `identity_providers` 表逐列对应；[`ProviderRegistry`] 负责从 DB 加载
//! enabled 的 provider 并缓存，reload 时刷新。

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use sqlx::PgPool;

/// OIDC provider 配置行 —— 对应 `identity_providers` 表。
///
/// `slug` 同时是 URL 路径段（`/api/auth/oidc/{slug}/authorize`），须稳定；
/// `issuer_url` 用于 OIDC discovery（`{issuer}/.well-known/openid-configuration`）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IdentityProvider {
    /// 稳定标识 + URL 路径段，如 `google` / `github` / `keycloak`。
    pub slug: String,
    /// 登录按钮显示名。
    pub display_name: String,
    /// OIDC issuer，discovery 的起点。
    pub issuer_url: String,
    /// OAuth client_id。
    pub client_id: String,
    /// OAuth client_secret（不明文返回给前端）。
    pub client_secret: String,
    /// 授权后回跳地址，须与 provider 后台登记一致。
    pub redirect_uri: String,
    /// 申请的 scope 列表，缺省 `['openid','email','profile']`。
    pub scopes: Vec<String>,
    /// 首次登录是否自动建号；false 时仅允许已绑定用户登录。
    pub auto_create_user: bool,
    /// 自动建号时的默认角色（1=user / 10=admin / 100=root）。
    pub default_role: i16,
    /// 是否启用；false 的 provider 不出现在登录页也不接受回调。
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// OIDC 协议层的错误类型。
///
/// 不含用户解析错误——那属于 [`auth::AuthError`]，在本 crate 的调用方
/// （路由层）处转换。
#[derive(Debug, thiserror::Error)]
pub enum OidcError {
    /// discovery document 拉取/解析失败（issuer 配错、网络不通）。
    #[error("oidc discovery failed: {0}")]
    Discovery(String),

    /// token 端点换取失败（code 过期、client_secret 错、PKCE 不匹配）。
    #[error("oidc token exchange failed: {0}")]
    Token(String),

    /// ID token 验签或 claims 校验失败（签名/iss/aud/exp/nonce）。
    #[error("oidc id token verification failed: {0}")]
    Verification(String),

    /// state 查不到或已过期（重放 / CSRF / 跨请求串号）。
    #[error("oidc state invalid or expired: {0}")]
    State(String),

    /// provider 不存在或未启用。
    #[error("oidc provider not available: {0}")]
    Provider(String),

    /// DB 故障。
    #[error("oidc db error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Provider 注册表 —— DB 加载 + `ArcSwap` 缓存。
///
/// 缓存而非每请求查库：登录是低频操作，但 discovery document 可能被
/// 同一瞬间的多个回调并发触发。`reload()` 由管理端改配置后调用。
pub struct ProviderRegistry {
    pool: sqlx::PgPool,
    cache: ArcSwap<HashMap<String, IdentityProvider>>,
}

impl ProviderRegistry {
    /// 建空注册表（尚未加载）。装配侧应紧接着调 [`Self::reload`]。
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            pool,
            cache: ArcSwap::from_pointee(HashMap::new()),
        }
    }

    /// 从 DB 重载全部 enabled provider 到缓存。
    ///
    /// 失败时**保留旧缓存**不清空——DB 抖动不该让已配置的 provider 集体失效。
    pub async fn reload(&self) -> Result<(), OidcError> {
        let rows: Vec<IdentityProvider> =
            sqlx::query_as("SELECT * FROM identity_providers WHERE enabled ORDER BY slug")
                .fetch_all(&self.pool)
                .await?;
        let map: HashMap<String, IdentityProvider> =
            rows.into_iter().map(|p| (p.slug.clone(), p)).collect();
        self.cache.store(Arc::new(map));
        Ok(())
    }

    /// 按 slug 找 enabled provider；不存在或未启用 → `None`。
    pub fn find(&self, slug: &str) -> Option<IdentityProvider> {
        self.cache.load().get(slug).cloned()
    }

    /// 全部 enabled provider（登录页枚举按钮用）。
    pub fn list(&self) -> Vec<IdentityProvider> {
        let mut v: Vec<IdentityProvider> = self.cache.load().values().cloned().collect();
        v.sort_by(|a, b| a.slug.cmp(&b.slug));
        v
    }
}
