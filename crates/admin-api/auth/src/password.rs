//! argon2id 密码哈希 / 校验。
//!
//! PHC 字符串存库 (`$argon2id$v=19$m=19456,t=2,p=1$...$...`)，参数和盐都含在里面。

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

use crate::error::AuthError;

pub fn hash(plain: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let phc = argon
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| AuthError::Crypto(e.to_string()))?
        .to_string();
    Ok(phc)
}

pub fn verify(plain: &str, phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok()
}
