//! token 估算 — 上游不回 usage 时的兜底。
//!
//! 参考: new-api usage/estimate_tokens.go (字符类权重: Latin/CJK/数字/emoji/
//! 数学/URL)。精度目标: ±10% (wildtoken 同量级), 用于 fallback 而非权威计量。

/// 按字符类加权估算 token 数。
///
/// 权重表 (TODO(#332) 校准): latin≈0.25 tok/char, cjk≈0.6, digit≈0.3...
pub fn estimate_tokens(_text: &str) -> u64 {
    0
}

/// 请求体 prompt 侧预扫 — forward::pipeline 在发送前调用。
/// JSON 结构感知: 只扫 messages 内容, 跳过 base64 图片数据。
/// TODO(#334): 内容抽取 (message content parts 遍历) + 图片 token 规则 (尺寸分档)。
pub fn estimate_prompt_tokens(_adapted_body: &bytes::Bytes) -> u64 {
    0
}
