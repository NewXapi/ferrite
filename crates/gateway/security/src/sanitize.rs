//! `sanitize` —— 输入静默脱敏
//!
//! 命中位置替换为 `***`（silent replace），不返回 403。
//! 脱敏后重建 token 计数（避免计费计算偏差）。

use crate::aho_corasick::MatchHit;

/// 在 input 上原地替换命中位置为 `***`
pub fn sanitize(input: &mut Vec<u8>, hits: &[MatchHit]) {
    // TODO: 按 end 倒序替换（避免 start 偏移失效）
    let _ = (input, hits);
    unimplemented!("sanitize")
}

/// 计算脱敏后实际有效字节数（用于重建 token 统计）
pub fn effective_len(hits: &[MatchHit]) -> usize {
    // TODO: sum(end - start) + len(hits) * 3  // "***"
    let _ = hits;
    unimplemented!("effective_len")
}
