//! HS256 JWT — access token 15 min。

use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AuthError;

/// 15 min
pub const ACCESS_TOKEN_TTL_SECS: u64 = 15 * 60;

/// 7 d
pub const REFRESH_TOKEN_TTL_SECS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// user key (UUID)
    pub sub: String,
    /// role: 1=user 10=admin 100=root
    pub role: u16,
    /// 当前 auth_version
    pub auth_version: i64,
    /// session id (UUID)
    pub sid: String,
    /// unix seconds
    pub exp: i64,
}

pub fn issue(
    secret: &[u8],
    user_key: &str,
    role: u16,
    auth_version: i64,
    sid: &str,
) -> Result<(String, i64), AuthError> {
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AuthError::Crypto(e.to_string()))?
        .as_secs() as i64
        + ACCESS_TOKEN_TTL_SECS as i64;
    let claims = Claims {
        sub: user_key.to_string(),
        role,
        auth_version,
        sid: sid.to_string(),
        exp,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )?;
    Ok((token, exp))
}

pub fn parse(secret: &[u8], token: &str) -> Result<Claims, AuthError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )?;
    Ok(data.claims)
}
