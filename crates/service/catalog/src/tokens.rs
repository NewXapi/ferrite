//! 令牌生命周期 — 创建 (一次性明文) / 吊销。

use store::StoreError;

/// 生成新令牌: 明文 = "sk-" + 48 位随机 base62。
///
/// 返回 (record_with_hash, plaintext_once)。
/// 参考: wildtoken api_tokens (token_hash + preview + 一次性明文)。
/// TODO(#427): 随机源 (rand::thread_rng) + 哈希 (sha256) + preview 生成。
pub fn generate_token(_user_key: &str, _name: &str) -> (contract::records::TokenRecord, String) {
    todo!("TODO(#427): 生成明文/哈希/预览")
}

/// 吊销: status → 2 (保留记录, 审计需要)。
pub async fn revoke(_key: &str) -> Result<(), StoreError> {
    todo!("TODO(#427): UPDATE status + outbox mutation")
}
