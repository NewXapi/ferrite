//! PNG tEXt chunk 读写
//!
//! 角色卡 JSON 以 base64 存在 `chara` 关键字的 tEXt chunk 中，与 SillyTavern
//! `src/character-card-parser.js` 同构。只处理这一个 chunk，不做图像解码。

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
const KEYWORD: &[u8] = b"chara";

#[derive(Debug, thiserror::Error)]
pub enum PngError {
    #[error("not a png")]
    NotPng,
    #[error("truncated chunk")]
    Truncated,
    #[error("no `chara` tEXt chunk")]
    NoCharaChunk,
    #[error("base64: {0}")]
    Base64(String),
}

struct Chunk<'a> {
    kind: &'a [u8],
    data: &'a [u8],
    /// chunk 在原始字节里的完整范围（含长度、类型、CRC）
    span: std::ops::Range<usize>,
}

fn chunks(bytes: &[u8]) -> Result<Vec<Chunk<'_>>, PngError> {
    if bytes.len() < 8 || bytes[..8] != SIGNATURE {
        return Err(PngError::NotPng);
    }
    let mut out = Vec::new();
    let mut i = 8;
    while i + 12 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
        let end = i + 12 + len;
        if end > bytes.len() {
            return Err(PngError::Truncated);
        }
        out.push(Chunk {
            kind: &bytes[i + 4..i + 8],
            data: &bytes[i + 8..i + 8 + len],
            span: i..end,
        });
        i = end;
    }
    Ok(out)
}

/// 取出 `chara` chunk 里的角色 JSON。
pub fn read_chara(bytes: &[u8]) -> Result<Vec<u8>, PngError> {
    for c in chunks(bytes)? {
        if c.kind != b"tEXt" {
            continue;
        }
        let Some(nul) = c.data.iter().position(|b| *b == 0) else {
            continue;
        };
        if &c.data[..nul] != KEYWORD {
            continue;
        }
        return base64_decode(&c.data[nul + 1..]);
    }
    Err(PngError::NoCharaChunk)
}

/// 把角色 JSON 写进 `chara` chunk，替换已有的那个。
pub fn write_chara(base_png: &[u8], json: &[u8]) -> Result<Vec<u8>, PngError> {
    let existing = chunks(base_png)?;
    let mut drop_span: Option<std::ops::Range<usize>> = None;
    let mut iend_start = base_png.len();
    for c in &existing {
        if c.kind == b"IEND" {
            iend_start = c.span.start;
        }
        if c.kind == b"tEXt" {
            if let Some(nul) = c.data.iter().position(|b| *b == 0) {
                if &c.data[..nul] == KEYWORD {
                    drop_span = Some(c.span.clone());
                }
            }
        }
    }

    let mut out = Vec::with_capacity(base_png.len() + json.len());
    let insert_at = drop_span.as_ref().map(|s| s.start).unwrap_or(iend_start);
    out.extend_from_slice(&base_png[..insert_at]);
    out.extend_from_slice(&text_chunk(KEYWORD, base64_encode(json).as_bytes()));
    let tail_from = match drop_span {
        Some(s) => s.end,
        None => insert_at,
    };
    out.extend_from_slice(&base_png[tail_from..]);
    Ok(out)
}

fn text_chunk(keyword: &[u8], value: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(keyword.len() + 1 + value.len());
    data.extend_from_slice(keyword);
    data.push(0);
    data.extend_from_slice(value);

    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(b"tEXt");
    out.extend_from_slice(&data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(b"tEXt");
    crc_input.extend_from_slice(&data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for b in bytes {
        crc ^= *b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18 & 63) as usize] as char);
        out.push(B64[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn base64_decode(input: &[u8]) -> Result<Vec<u8>, PngError> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let clean: Vec<u8> = input
        .iter()
        .copied()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
        .collect();
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for group in clean.chunks(4) {
        let mut n = 0u32;
        for (i, c) in group.iter().enumerate() {
            let v = val(*c).ok_or_else(|| PngError::Base64(format!("bad char {c}")))?;
            n |= v << (18 - 6 * i);
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..group.len() - 1]);
    }
    Ok(out)
}

/// 最小合法 PNG（1x1），测试与默认头像用。
pub fn minimal_png() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&SIGNATURE);
    let ihdr: [u8; 13] = [0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0];
    out.extend_from_slice(&(ihdr.len() as u32).to_be_bytes());
    out.extend_from_slice(b"IHDR");
    out.extend_from_slice(&ihdr);
    let mut crc_in = b"IHDR".to_vec();
    crc_in.extend_from_slice(&ihdr);
    out.extend_from_slice(&crc32(&crc_in).to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(b"IEND");
    out.extend_from_slice(&crc32(b"IEND").to_be_bytes());
    out
}
