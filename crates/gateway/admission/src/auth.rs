//! 闸① 鉴权 — key 查找 + 状态/过期校验 + 模型白名单。
//!
//! 参考: new-api internal/identity + authtoken (会话与 relay token 两族凭证),
//! one-api middleware/auth.go (状态/过期/模型限制), wildtoken (SHA-256 即时查)。

use contract::records::TokenRecord;

use crate::error::Rejection;
use crate::snapshot::TokenSnapshot;

/// 闸①: 请求头明文 key → 快照查找 → 状态校验。
///
/// 零分配哈希: sha256(raw_key) 直接对 HashMap 查找; 找不到 = InvalidKey
/// (不区分"不存在"与"已吊销", 避免枚举探测)。
pub fn authenticate<'a>(snapshot: &'a TokenSnapshot, raw_key: &str) -> Result<&'a TokenRecord, Rejection> {
    let key_hash = sha256_hex(raw_key);
    let token = snapshot
        .find_token(&key_hash)
        .ok_or(Rejection::InvalidKey)?;

    if token.status != 1 {
        return Err(Rejection::TokenUnavailable("token disabled".into()));
    }
    if let Some(expires_at) = token.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err(Rejection::TokenUnavailable("token expired".into()));
        }
    }
    Ok(token)
}

/// 模型白名单校验 — ① 与 ② 之间。
///
/// 语义 (对齐 one-api token.Models): Some(空列表) = 显式无权;
/// allowed 为空 = 不限 (跟随组白名单, 组检查在 dispatch 候选阶段兜底)。
pub fn check_model_allowed(token: &TokenRecord, model: &str, _group_models: &[String]) -> Result<(), Rejection> {
    // TODO(#302): 三层白名单合并 — token 级 (本函数) + 组级 (GroupRecord.allowed_models,
    // 数据在 catalog 快照) + 用户级; 现在只做 token 级占位。
    let _ = (token, model);
    Ok(())
}

/// SHA-256 十六进制 (依赖注入而非直接引 sha2, 便于测试)。
/// TODO(#303): 引 sha2 crate; 这里签名先行。
fn sha256_hex(_raw: &str) -> String {
    todo!("TODO(#303): sha2::Sha256 实现")
}
