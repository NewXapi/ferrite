//! AuthService — login / register / refresh / logout / self。
//!
//! 直连 sqlx::PgPool，不走 store trait。

use std::time::Duration;

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::error::AuthError;
use crate::jwt;
use crate::password;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, FromRow)]
struct UserRow {
    key: Uuid,
    username: String,
    display_name: String,
    email: Option<String>,
    password_hash: String,
    role: i16,
    status: i16,
    quota: i64,
    used_quota: i64,
    group_id: String,
    auth_version: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
    pub key: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub role: u16,
    pub status: u8,
    pub quota: i64,
    pub used_quota: i64,
    pub group: String,
    pub auth_version: i64,
    pub created_at: DateTime<Utc>,
}

impl From<UserRow> for UserView {
    fn from(r: UserRow) -> Self {
        Self {
            key: r.key.to_string(),
            username: r.username,
            display_name: r.display_name,
            email: r.email.unwrap_or_default(),
            role: r.role as u16,
            status: r.status as u8,
            quota: r.quota,
            used_quota: r.used_quota,
            group: r.group_id,
            auth_version: r.auth_version,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub user: UserView,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

pub type SelfView = UserView;

pub struct AuthService {
    pool: PgPool,
    jwt_secret: Vec<u8>,
    refresh_secret: Vec<u8>,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

impl AuthService {
    /// `jwt_secret` 必须 >= 32 字节 (HS256 安全下限)。
    pub fn new(pool: PgPool, jwt_secret: Vec<u8>) -> Self {
        assert!(
            jwt_secret.len() >= 32,
            "jwt secret must be >= 32 bytes for HS256"
        );
        // refresh_secret 由 jwt_secret 派生: 轮换 jwt secret 会使全部 refresh 失效
        // (预期行为 — 换密钥 = 强制全员重新登录)。
        let mut mac = HmacSha256::new_from_slice(&jwt_secret).expect("HMAC accepts any key length");
        mac.update(b"refresh");
        let refresh_secret = mac.finalize().into_bytes().to_vec();

        Self {
            pool,
            jwt_secret,
            refresh_secret,
            access_ttl: Duration::from_secs(jwt::ACCESS_TOKEN_TTL_SECS),
            refresh_ttl: Duration::from_secs(jwt::REFRESH_TOKEN_TTL_SECS),
        }
    }

    pub fn access_ttl(&self) -> u64 {
        self.access_ttl.as_secs()
    }

    fn hash_refresh(&self, secret: &[u8]) -> String {
        let mut mac =
            HmacSha256::new_from_slice(&self.refresh_secret).expect("HMAC accepts any key");
        mac.update(secret);
        hex::encode(mac.finalize().into_bytes())
    }

    fn random_refresh_secret(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        buf
    }

    fn split_refresh(&self, raw: &str) -> Result<(Uuid, Vec<u8>), AuthError> {
        let (sid_s, secret_s) = raw.split_once('.').ok_or(AuthError::InvalidToken)?;
        let sid = Uuid::parse_str(sid_s).map_err(|_| AuthError::InvalidToken)?;
        let secret = hex::decode(secret_s).map_err(|_| AuthError::InvalidToken)?;
        if secret.len() != 32 {
            return Err(AuthError::InvalidToken);
        }
        Ok((sid, secret))
    }

    fn encode_refresh(&self, sid: Uuid, secret: &[u8]) -> String {
        format!("{}.{}", sid, hex::encode(secret))
    }

    pub async fn login(
        &self,
        username: &str,
        password_plain: &str,
        user_agent: &str,
        ip: &str,
    ) -> Result<LoginResult, AuthError> {
        let row: Option<UserRow> = sqlx::query_as::<_, UserRow>(
            r#"SELECT key, username, display_name, email, password_hash, role, status,
                      quota, used_quota, group_id, auth_version, created_at
               FROM auth_users WHERE username = $1"#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        // 防用户枚举: 用户不存在时也对 dummy hash 跑一次 argon2 verify,
        // 让"不存在"与"密码错"耗时一致。密码对但被禁用 → UserDisabled
        // (与 refresh 语义一致, 也不泄露用户是否存在 — 先验密码再报状态)。
        let (row, verified) = match row {
            Some(r) => {
                let ok = password::verify(password_plain, &r.password_hash);
                (Some(r), ok)
            }
            None => {
                let _ = password::verify(
                    password_plain,
                    "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$Zm9vYmFyYmF6cXV1eGZvb2Jhcg",
                );
                (None, false)
            }
        };

        let row = match (row, verified) {
            (Some(r), true) if r.status == 1 => r,
            (Some(_), true) => return Err(AuthError::UserDisabled),
            _ => return Err(AuthError::InvalidCredentials),
        };

        self.issue_session(row, user_agent, ip).await
    }

    pub async fn register(
        &self,
        username: &str,
        password_plain: &str,
        email: Option<&str>,
    ) -> Result<SelfView, AuthError> {
        if username.trim().is_empty() {
            return Err(AuthError::BadRequest("username required".into()));
        }
        if username.chars().count() > 32 {
            return Err(AuthError::BadRequest("username <= 32 chars".into()));
        }
        validate_password(password_plain)?;
        let phc = password::hash(password_plain)?;
        let key = Uuid::new_v4();
        let now: DateTime<Utc> = Utc::now();

        let res = sqlx::query(
            r#"INSERT INTO auth_users (key, username, display_name, email, password_hash,
                                        role, status, quota, used_quota, group_id,
                                        auth_version, created_at)
               VALUES ($1, $2, $3, $4, $5, 1, 1, 0, 0, 'default', 1, $6)"#,
        )
        .bind(key)
        .bind(username)
        .bind(username)
        .bind(email)
        .bind(&phc)
        .bind(now)
        .execute(&self.pool)
        .await;

        if let Err(e) = res {
            // 23505 unique_violation — 按约束名区分 (auth_users_username_key /
            // auth_users_email_key), 避免 email 有值时误报 EmailTaken。
            if let sqlx::Error::Database(db) = &e
                && db.code().as_deref() == Some("23505")
            {
                let constraint = db.constraint().unwrap_or_default();
                return Err(if constraint.contains("email") {
                    AuthError::EmailTaken
                } else {
                    AuthError::UsernameTaken
                });
            }
            return Err(AuthError::Db(e));
        }

        self.fetch_user_by_key(key).await
    }

    pub async fn refresh(
        &self,
        raw_refresh: &str,
        user_agent: &str,
        ip: &str,
    ) -> Result<RefreshResult, AuthError> {
        let (sid, secret) = self.split_refresh(raw_refresh)?;
        let presented = self.hash_refresh(&secret);

        let row: Option<(Uuid, String, i64)> = sqlx::query_as(
            r#"SELECT user_key, token_hash, auth_version
               FROM auth_refresh_tokens
               WHERE sid = $1 AND revoked_at IS NULL AND expires_at > now()"#,
        )
        .bind(sid)
        .fetch_optional(&self.pool)
        .await?;

        let (user_key, expected_hash, auth_version) = row.ok_or(AuthError::InvalidToken)?;
        if !constant_time_eq(presented.as_bytes(), expected_hash.as_bytes()) {
            return Err(AuthError::InvalidToken);
        }

        let revoked = sqlx::query(
            r#"UPDATE auth_refresh_tokens
               SET revoked_at = now() WHERE sid = $1 AND revoked_at IS NULL"#,
        )
        .bind(sid)
        .execute(&self.pool)
        .await?
        .rows_affected();

        // 并发 refresh 竞争: 另一个请求已经吊销了这个 sid → 本次拒绝。
        // 客户端拿到新 refresh 后旧的重放会走到这里 (new-api RotateUserSessionRefresh 语义)。
        if revoked == 0 {
            return Err(AuthError::InvalidToken);
        }

        // 新 refresh — 调 issue_session 需要 UserRow，从 DB 拉
        let user: UserRow = sqlx::query_as::<_, UserRow>(
            r#"SELECT key, username, display_name, email, password_hash, role, status,
                      quota, used_quota, group_id, auth_version, created_at
               FROM auth_users WHERE key = $1"#,
        )
        .bind(user_key)
        .fetch_one(&self.pool)
        .await?;

        if user.status != 1 {
            return Err(AuthError::UserDisabled);
        }

        // 会话创建后用户改过密码 (auth_version 变了) → 全 refresh 失效
        if user.auth_version != auth_version {
            return Err(AuthError::InvalidToken);
        }

        let login = self.issue_session(user, user_agent, ip).await?;
        Ok(RefreshResult {
            access_token: login.access_token,
            refresh_token: login.refresh_token,
            expires_in: login.expires_in,
        })
    }

    pub async fn logout(&self, raw_refresh: &str) -> Result<(), AuthError> {
        let (sid, secret) = self.split_refresh(raw_refresh)?;
        let presented = self.hash_refresh(&secret);

        let affected = sqlx::query(
            r#"UPDATE auth_refresh_tokens
               SET revoked_at = now()
               WHERE sid = $1 AND token_hash = $2 AND revoked_at IS NULL"#,
        )
        .bind(sid)
        .bind(presented)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if affected == 0 {
            return Err(AuthError::InvalidToken);
        }
        Ok(())
    }

    pub async fn self_by_access(&self, access_token: &str) -> Result<SelfView, AuthError> {
        let claims = jwt::parse(&self.jwt_secret, access_token)?;
        let key = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
        let user = self.fetch_user_by_key(key).await?;
        if user.auth_version != claims.auth_version {
            return Err(AuthError::InvalidToken);
        }
        if user.status != 1 {
            return Err(AuthError::UserDisabled);
        }
        Ok(user)
    }

    async fn fetch_user_by_key(&self, key: Uuid) -> Result<UserView, AuthError> {
        let row: UserRow = sqlx::query_as::<_, UserRow>(
            r#"SELECT key, username, display_name, email, password_hash, role, status,
                      quota, used_quota, group_id, auth_version, created_at
               FROM auth_users WHERE key = $1"#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthError::UserNotFound)?;
        Ok(row.into())
    }
    // ---------- self 管理 / admin 用户管理 ----------

    /// 改昵称 / 改密码。改密码需要原密码, 成功后 auth_version++
    /// (全部 access/refresh 立即失效) 并吊销该用户全部 refresh。
    pub async fn update_self(
        &self,
        user_key: Uuid,
        display_name: Option<&str>,
        original_password: Option<&str>,
        new_password: Option<&str>,
    ) -> Result<UserView, AuthError> {
        if let (Some(_), None) | (None, Some(_)) = (new_password, original_password) {
            return Err(AuthError::BadRequest(
                "original_password and new_password must be provided together".into(),
            ));
        }

        let row: UserRow = sqlx::query_as::<_, UserRow>(
            r#"SELECT key, username, display_name, email, password_hash, role, status,
                      quota, used_quota, group_id, auth_version, created_at
               FROM auth_users WHERE key = $1"#,
        )
        .bind(user_key)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthError::UserNotFound)?;

        let bump_version = if let Some(new_pwd) = new_password {
            let original = original_password.expect("checked above");
            validate_password(new_pwd)?;
            if !password::verify(original, &row.password_hash) {
                return Err(AuthError::InvalidCredentials);
            }
            let phc = password::hash(new_pwd)?;
            // 原子改密: WHERE 带旧 hash — 并发改密时第二个请求的 verify
            // 基于旧 hash 通过, 但 UPDATE 因 hash 已变 rows_affected=0 → 拒绝。
            let affected = sqlx::query(
                "UPDATE auth_users SET password_hash = $2, auth_version = auth_version + 1, updated_at = now() WHERE key = $1 AND password_hash = $3",
            )
            .bind(user_key)
            .bind(&phc)
            .bind(&row.password_hash)
            .execute(&self.pool)
            .await?
            .rows_affected();
            if affected == 0 {
                return Err(AuthError::Conflict("password changed concurrently".into()));
            }
            true
        } else {
            false
        };

        if let Some(name) = display_name {
            if name.chars().count() > 64 {
                return Err(AuthError::BadRequest("display_name <= 64 chars".into()));
            }
            sqlx::query(
                "UPDATE auth_users SET display_name = $2, updated_at = now() WHERE key = $1",
            )
            .bind(user_key)
            .bind(name)
            .execute(&self.pool)
            .await?;
        }

        if bump_version {
            // 改密 = 全端登出
            sqlx::query("DELETE FROM auth_refresh_tokens WHERE user_key = $1")
                .bind(user_key)
                .execute(&self.pool)
                .await?;
        }

        self.fetch_user_by_key(user_key).await
    }

    /// 注销 / admin 删号 — 平表阶段直接硬删 (含 refresh), 不过度设计软删。
    pub async fn delete_user(&self, user_key: Uuid) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM auth_refresh_tokens WHERE user_key = $1")
            .bind(user_key)
            .execute(&self.pool)
            .await?;
        let affected = sqlx::query("DELETE FROM auth_users WHERE key = $1")
            .bind(user_key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AuthError::UserNotFound);
        }
        Ok(())
    }

    /// admin 单查用户（按 key）。
    pub async fn get_user(&self, key: Uuid) -> Result<UserView, AuthError> {
        self.fetch_user_by_key(key).await
    }

    /// admin 搜索用户（ILIKE 过滤，前 20 条）。
    pub async fn search_users(&self, keyword: &str) -> Result<Vec<UserView>, AuthError> {
        let (items, _) = self.list_users(Some(keyword), 1, 20).await?;
        Ok(items)
    }

    /// admin 用户列表 — 分页 + 搜索 (username/email/display_name ILIKE)。
    pub async fn list_users(
        &self,
        search: Option<&str>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<UserView>, i64), AuthError> {
        let size = size.clamp(1, 100);
        let page = page.max(1);
        let offset = (page - 1) * size;
        let pattern = search
            .map(|s| format!("%{}%", s.trim()))
            .unwrap_or_else(|| "%".into());

        let total: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM auth_users WHERE username ILIKE $1 OR email ILIKE $1 OR display_name ILIKE $1",
        )
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await?;

        let rows: Vec<UserRow> = sqlx::query_as::<_, UserRow>(
            r#"SELECT key, username, display_name, email, password_hash, role, status,
                      quota, used_quota, group_id, auth_version, created_at
               FROM auth_users
               WHERE username ILIKE $1 OR email ILIKE $1 OR display_name ILIKE $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(&pattern)
        .bind(size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((rows.into_iter().map(Into::into).collect(), total))
    }

    /// admin 用户操作 — action ∈ {enable, disable, set_role, adjust_quota, reset_password}。
    pub async fn manage_user(
        &self,
        target_key: Uuid,
        action: &str,
        value: Option<&str>,
    ) -> Result<UserView, AuthError> {
        match action {
            "enable" | "disable" => {
                let status: i16 = if action == "enable" { 1 } else { 2 };
                let affected = sqlx::query(
                    "UPDATE auth_users SET status = $2, updated_at = now() WHERE key = $1",
                )
                .bind(target_key)
                .bind(status)
                .execute(&self.pool)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(AuthError::UserNotFound);
                }
                if status == 2 {
                    sqlx::query("DELETE FROM auth_refresh_tokens WHERE user_key = $1")
                        .bind(target_key)
                        .execute(&self.pool)
                        .await?;
                }
            }
            "set_role" => {
                let role: i16 = value
                    .and_then(|v| v.parse().ok())
                    .ok_or_else(|| AuthError::BadRequest("value must be 1 | 10 | 100".into()))?;
                if ![1, 10, 100].contains(&role) {
                    return Err(AuthError::BadRequest("role must be 1 | 10 | 100".into()));
                }
                let affected = sqlx::query(
                    "UPDATE auth_users SET role = $2, updated_at = now() WHERE key = $1",
                )
                .bind(target_key)
                .bind(role)
                .execute(&self.pool)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(AuthError::UserNotFound);
                }
            }
            "adjust_quota" => {
                let delta: i64 = value.and_then(|v| v.parse().ok()).ok_or_else(|| {
                    AuthError::BadRequest("value must be an integer delta".into())
                })?;
                let affected = sqlx::query(
                    "UPDATE auth_users SET quota = GREATEST(0, quota + $2), updated_at = now() WHERE key = $1",
                )
                .bind(target_key)
                .bind(delta)
                .execute(&self.pool)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(AuthError::UserNotFound);
                }
            }
            "reset_password" => {
                let new_pwd =
                    value.ok_or_else(|| AuthError::BadRequest("value = new password".into()))?;
                validate_password(new_pwd)?;
                let phc = password::hash(new_pwd)?;
                let affected = sqlx::query(
                    "UPDATE auth_users SET password_hash = $2, auth_version = auth_version + 1, updated_at = now() WHERE key = $1",
                )
                .bind(target_key)
                .bind(&phc)
                .execute(&self.pool)
                .await?
                .rows_affected();
                if affected == 0 {
                    return Err(AuthError::UserNotFound);
                }
                sqlx::query("DELETE FROM auth_refresh_tokens WHERE user_key = $1")
                    .bind(target_key)
                    .execute(&self.pool)
                    .await?;
            }
            _ => {
                return Err(AuthError::BadRequest(
                    "action must be enable|disable|set_role|adjust_quota|reset_password".into(),
                ));
            }
        }
        self.fetch_user_by_key(target_key).await
    }

    async fn issue_session(
        &self,
        user: UserRow,
        user_agent: &str,
        ip: &str,
    ) -> Result<LoginResult, AuthError> {
        let sid = Uuid::new_v4();
        let secret = self.random_refresh_secret();
        let token_hash = self.hash_refresh(&secret);
        let expires_at = Utc::now()
            + chrono::Duration::from_std(self.refresh_ttl)
                .map_err(|e| AuthError::Crypto(e.to_string()))?;

        sqlx::query(
            r#"INSERT INTO auth_refresh_tokens
               (sid, user_key, token_hash, auth_version, user_agent, ip, issued_at, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, now(), $7)"#,
        )
        .bind(sid)
        .bind(user.key)
        .bind(&token_hash)
        .bind(user.auth_version)
        .bind(user_agent)
        .bind(ip)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        let (access_token, _exp) = jwt::issue(
            &self.jwt_secret,
            &user.key.to_string(),
            user.role as u16,
            user.auth_version,
            &sid.to_string(),
        )?;

        Ok(LoginResult {
            user: user.into(),
            access_token,
            refresh_token: self.encode_refresh(sid, &secret),
            expires_in: self.access_ttl(),
        })
    }
}

/// 密码策略: 8..=128 字节 (上限防 argon2 DoS)。
fn validate_password(p: &str) -> Result<(), AuthError> {
    if p.len() < 8 || p.len() > 128 {
        return Err(AuthError::BadRequest(
            "password length must be 8..=128".into(),
        ));
    }
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
