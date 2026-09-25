//! state / nonce / PKCE 的一次性存储 —— 防重放与 CSRF。
//!
//! OIDC 授权码流程的安全前提：`authorize` 时生成随机 state + nonce +
//! PKCE verifier 落库，`callback` 时凭 state 取回并**取出即删**。
//! 取不到 = 重放、跨请求串号或过期，一律拒绝。
//!
//! 存 DB 而非纯内存：与仓库「PG 权威」架构一致，重启不丢，多实例也安全。

use sqlx::PgPool;

use crate::provider::OidcError;

/// 授权请求发起时登记的待验参数。
#[derive(Debug, Clone)]
pub struct PendingAuthState {
    /// 发起授权的 provider slug。
    pub provider_slug: String,
    /// 要求 ID token 必须带回的 nonce（防 token 重放）。
    pub nonce: String,
    /// PKCE verifier（callback 换 token 时提交原文）。
    pub pkce_verifier: String,
    /// 登录成功后要跳转的前端路径（相对路径，需调用方校验防开放重定向）。
    pub redirect_after: Option<String>,
}

/// state 存储 —— `oidc_auth_states` 表的薄封装。
pub struct AuthStateStore {
    pool: PgPool,
}

impl AuthStateStore {
    /// 建存储（与 provider 共用同一个 PG 池）。
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 登记一次授权请求。
    ///
    /// - `state` / `nonce` / `pkce_verifier`：调用方生成的随机值（见 [`new_state`]）
    /// - `ttl_secs`：有效期；过期行在 `take` 时顺带清理
    pub async fn insert(
        &self,
        state: &str,
        provider_slug: &str,
        nonce: &str,
        pkce_verifier: &str,
        redirect_after: Option<&str>,
        ttl_secs: u64,
    ) -> Result<(), OidcError> {
        sqlx::query(
            "INSERT INTO oidc_auth_states
                 (state, provider_slug, nonce, pkce_verifier, redirect_after, expires_at)
             VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6))",
        )
        .bind(state)
        .bind(provider_slug)
        .bind(nonce)
        .bind(pkce_verifier)
        .bind(redirect_after)
        .bind(ttl_secs as f64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 凭 state 取回参数并**立即删除**（防重放）。
    ///
    /// 过期行一并清理。返回 `None` = state 不存在或已过期，调用方必须拒绝。
    pub async fn take(&self, state: &str) -> Result<Option<PendingAuthState>, OidcError> {
        let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
            "DELETE FROM oidc_auth_states
             WHERE state = $1 AND expires_at > now()
             RETURNING provider_slug, nonce, pkce_verifier, redirect_after",
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await?;

        // 顺手清过期行（低频操作，全表扫可接受；行数随授权流量增长，
        // 若成负担再改定时任务，见 TODO(#245)）。
        let _ = sqlx::query("DELETE FROM oidc_auth_states WHERE expires_at < now()")
            .execute(&self.pool)
            .await;

        Ok(row.map(
            |(provider_slug, nonce, pkce_verifier, redirect_after)| PendingAuthState {
                provider_slug,
                nonce,
                pkce_verifier,
                redirect_after,
            },
        ))
    }
}

/// 生成随机 state / nonce / PKCE verifier。
///
/// - `state` / `nonce`：uuid v4（122 bit 随机）足够——被猜到即可伪造登录。
/// - `pkce_verifier`：**不用 uuid**。RFC 7636 要求 verifier 是 43-128 字符的
///   `[A-Za-z0-9-._~]`，uuid 连字符形式只有 36 字符，
///   `PkceCodeChallenge::from_code_verifier_sha256` 会因长度不合规 panic。
///   这里用两个 uuid 拼接去掉连字符（32+32=64 字符，全落在合法字符集内），
///   熵 244 bit，仍然不自造随机数发生器。
pub fn new_state() -> (String, String, String) {
    let state = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    let pkce_verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    (state, nonce, pkce_verifier)
}
