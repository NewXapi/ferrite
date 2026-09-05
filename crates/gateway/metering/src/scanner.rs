//! 流式 token 扫描器 — 挂在 forward::stream 管道里。

use bytes::Bytes;

/// 流结束时的最终计数。
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenCounts {
    pub prompt: u64,
    pub completion: u64,
    pub cached: u64,
}

#[derive(Default)]
pub struct StreamScanner {
    upstream_usage: Option<TokenCounts>,
    char_count: u64,
    line_buf: Vec<u8>,
}

impl StreamScanner {
    pub fn new() -> Self {
        Self {
            upstream_usage: None,
            char_count: 0,
            line_buf: Vec::new(),
        }
    }

    pub fn push(&mut self, chunk: &Bytes) {
        let mut full = self.line_buf.clone();
        full.extend_from_slice(chunk);

        let mut start = 0;
        for (i, &b) in full.iter().enumerate() {
            if b == b'\n' {
                let line = &full[start..i];
                let line = if line.ends_with(b"\r") {
                    &line[..line.len() - 1]
                } else {
                    line
                };
                if let Some(rest) = line.strip_prefix(b"data: ")
                    && !rest.starts_with(b"[DONE]")
                {
                    self.char_count += rest.len() as u64;
                    if let Some(usage) = try_extract_usage(rest) {
                        self.upstream_usage = Some(usage);
                    }
                }
                start = i + 1;
            }
        }
        if start < full.len() {
            self.line_buf = full[start..].to_vec();
        } else {
            self.line_buf.clear();
        }
    }

    pub fn finish(self, prompt: u64) -> TokenCounts {
        if let Some(usage) = self.upstream_usage {
            usage
        } else {
            TokenCounts {
                prompt,
                completion: self.char_count / 4,
                cached: 0,
            }
        }
    }
}

fn try_extract_usage(data: &[u8]) -> Option<TokenCounts> {
    let s = std::str::from_utf8(data).ok()?;
    let v: serde_json::Value = serde_json::from_str(s).ok()?;

    if let Some(usage) = v.get("usage") {
        return Some(TokenCounts {
            prompt: usage
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            completion: usage
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cached: 0,
        });
    }

    if let Some(usage) = v.get("usage") {
        return Some(TokenCounts {
            prompt: usage
                .get("input_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            completion: usage
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            cached: 0,
        });
    }

    None
}
