//! token 估算 — 上游不回 usage 时的兜底。
//!
//! 参考: new-api usage/estimate_tokens.go (字符类权重: Latin/CJK/数字/emoji/
//! 数学/URL)。精度目标: ±10% (wildtoken 同量级), 用于 fallback 而非权威计量。

/// 按字符类加权估算 token 数。
///
/// 权重表 (TODO(#332) 校准): latin≈0.25 tok/char, cjk≈0.6, digit≈0.3...
pub fn estimate_tokens(text: &str) -> u64 {
    let mut cjk = 0u64;
    let mut latin = 0u64;
    let mut digit = 0u64;
    let mut other = 0u64;

    for ch in text.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&ch)
            || ('\u{3400}'..='\u{4dbf}').contains(&ch)
            || ('\u{20000}'..='\u{2a6df}').contains(&ch)
        {
            cjk += 1;
        } else if ch.is_ascii_alphabetic() {
            latin += 1;
        } else if ch.is_ascii_digit() {
            digit += 1;
        } else if !ch.is_ascii_whitespace() {
            other += 1;
        }
    }

    // CJK ≈ 0.6 tok/char, Latin ≈ 0.25 tok/char, digit ≈ 0.3 tok/char, other ≈ 1.0
    let tokens =
        (cjk as f64 * 0.6) + (latin as f64 * 0.25) + (digit as f64 * 0.3) + (other as f64 * 1.0);
    tokens.ceil() as u64
}

/// 请求体 prompt 侧预扫 — forward::pipeline 在发送前调用。
/// JSON 结构感知: 只扫 messages 内容, 跳过 base64 图片数据。
/// TODO(#334): 内容抽取 (message content parts 遍历) + 图片 token 规则 (尺寸分档)。
pub fn estimate_prompt_tokens(adapted_body: &bytes::Bytes) -> u64 {
    let s = match std::str::from_utf8(adapted_body) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    // 必须是合法 JSON 才做内容抽取
    if serde_json::from_str::<serde_json::Value>(s).is_err() {
        return 0;
    }

    // 简单启发式: 提取所有 "content":"..." 字段的文本长度
    let mut total_chars = 0u64;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            // 读取字段名
            let mut field = String::new();
            while let Some(c) = chars.next() {
                if c == '"' {
                    break;
                }
                field.push(c);
            }
            // 跳过 : 和空白
            while let Some(&c) = chars.peek() {
                if c == ':' || c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
            if field == "content" && chars.peek() == Some(&'"') {
                // 读取字符串值
                chars.next(); // skip opening "
                let mut value = String::new();
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        if let Some(escaped) = chars.next() {
                            value.push(escaped);
                        }
                    } else if c == '"' {
                        break;
                    } else {
                        value.push(c);
                    }
                }
                total_chars += value.len() as u64;
            }
        }
    }

    // 粗略: 混合文本约 4 chars/token
    if total_chars > 0 {
        total_chars / 4
    } else {
        s.chars().count() as u64 / 4
    }
}
