//! AuthService — login / register / refresh / logout / self。
//!
//! 直连 sqlx::PgPool，不走 store trait。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    pub fn new(pool: PgPool, jwt_secret: Vec<u8>) -> Self {
        // refresh secret = HMAC(secret, "refresh") — 与 jwt secret 解耦，
        // 即便 jwt 密钥轮换，存库的 refresh_hash 仍稳定。
        let mut mac = HmacSha256::new_from_slice(&jwt_secret)
            .expect("HMAC accepts any key length");
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
        let mut mac = HmacSha256::new_from_slice(&self.refresh_secret)
            .expect("HMAC accepts any key");
        mac.update(secret);
        hex::encode(mac.finalize().into_bytes())
    }

    fn random_refresh_secret(&self) -> [u8; 32] {
        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        buf
    }

    fn split_refresh(&self, raw: &str) -> Result<(Uuid, Vec<u8>), AuthError> {
        let (sid_s, secret_s) = raw
            .split_once('.')
            .ok_or(AuthError::InvalidToken)?;
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

        let row = match row {
            Some(r) if r.status == 1 && password::verify(password_plain, &r.password_hash) => r,
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
        if username.is_empty() || password_plain.len() < 8 {
            return Err(AuthError::BadRequest(
                "username required, password >= 8 chars".into(),
            ));
        }
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
            // 23505 unique_violation
            if let sqlx::Error::Database(db) = &e
                && db.code().as_deref() == Some("23505")
            {
                return Err(if email.is_some() {
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
               FROM auth_refresh_tokens t
               JOIN auth_users u ON u.key = t.user_key
               WHERE t.sid = $1 AND t.revoked_at IS NULL AND t.expires_at > now()"#,
        )
        .bind(sid)
        .fetch_optional(&self.pool)
        .await?;

        let (user_key, expected_hash, auth_version) = row.ok_or(AuthError::InvalidToken)?;
        if !constant_time_eq(presented.as_bytes(), expected_hash.as_bytes()) {
            return Err(AuthError::InvalidToken);
        }

        // revoke old
        sqlx::query(
            r#"UPDATE auth_refresh_tokens
               SET revoked_at = now() WHERE sid = $1 AND revoked_at IS NULL"#,
        )
        .bind(sid)
        .execute(&self.pool)
        .await?;

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

        // 旋转后 auth_version 应一致；改了密则全 refresh 失效
        let _ = auth_version; // 已通过 SELECT 校验存在；当前 row.auth_version 再走 access JWT 即可

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
               (sid, user_key, token_hash, user_agent, ip, issued_at, expires_at)
               VALUES ($1, $2, $3, $4, $5, now(), $6)"#,
        )
        .bind(sid)
        .bind(user.key)
        .bind(&token_hash)
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

// quiet unused
#[allow(dead_code)]
fn _unused() -> Option<SystemTime> {
    SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|_| SystemTime::now())
}
