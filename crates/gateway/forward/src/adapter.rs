//! 上游请求准备 — URL / 鉴权头 / 渠道策略。
//!
//! 对应 new-api adaptor 五方法契约 (relay/adaptor/interface.go) 中的
//! GetRequestURL + SetupRequestHeader + ConvertRequest 的非协议部分。
//! 协议体转换在 protocol crate, 这里只做**寻址与鉴权**。

use dispatch::Candidate;

/// 准备产物 — egress::execute 的直接输入。
#[derive(Debug, Clone)]
pub struct PreparedRequest {
    /// 完整上游 URL (base_url + path, 含 /v1 前缀规则)。
    pub url: String,
    /// 鉴权头 (如 Authorization: Bearer sk-...)。
    pub auth_header: (String, String),
    /// 渠道级覆盖头 (ChannelRecord.settings.headers)。
    pub extra_headers: Vec<(String, String)>,
}

/// 鉴权头名 (provider → 头名)。path 规则见 [`build_url`]。
///
/// openai → Bearer (`Authorization`)
/// claude → `x-api-key`
/// gemini → `x-goog-api-key`
/// passthrough → `Authorization: Bearer` (无 secrets 兜底, 由 settings 覆盖)
fn auth_header_for(provider: &str, secret: &str) -> (String, String) {
    let normalized = provider.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "claude" => ("x-api-key".into(), secret.into()),
        "gemini" => ("x-goog-api-key".into(), secret.into()),
        // openai / passthrough / 未知 — Bearer 兜底 (settings 可覆盖)。
        _ => ("Authorization".into(), format!("Bearer {secret}")),
    }
}

/// 拼接上游 URL — 按 provider 分发路径模板。
///
/// - `openai` → `{base}/v1{path}` (兼容 client 路径含/不含 /v1 前缀)
/// - `claude` → `{base}/v1/messages` (强制覆盖, 客户端 path 不影响)
/// - `gemini` → `{base}/v1beta/{path}` (Gemini 路径含 `models/...`)
/// - `passthrough` → `{base}{path}` (字节透传, 路径由客户端决定)
///
/// base 末尾的 `/` 与 path 开头的 `/` 自动归一化, 避免双斜杠或漏斜杠。
fn build_url(provider: &str, base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let normalized = provider.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "claude" => format!("{base}/v1/messages"),
        "gemini" => {
            // Gemini 路径形如 `/v1beta/models/foo:streamGenerateContent`, path 通常已含 /v1beta 前缀。
            // 若 path 已含 /v1beta/ 则直接拼接 base+path; 否则强制补 /v1beta。
            let trimmed = path.trim_start_matches('/');
            if trimmed.starts_with("v1beta/") {
                format!("{base}/{trimmed}")
            } else {
                format!("{base}/v1beta/{trimmed}")
            }
        }
        "openai" => {
            // path 通常已含 /v1/..., 但客户端也可能直接打 /chat/completions (无 /v1)。
            // 仅补一次 /v1 前缀, 避免 /v1/v1 双前缀。
            let trimmed = path.trim_start_matches('/');
            if trimmed.starts_with("v1/") {
                format!("{base}/{trimmed}")
            } else {
                format!("{base}/v1/{trimmed}")
            }
        }
        // passthrough / 未知 — 透传, 客户端路径完全决定目标。
        _ => {
            let trimmed = path.trim_start_matches('/');
            if trimmed.is_empty() {
                base.to_string()
            } else {
                format!("{base}/{trimmed}")
            }
        }
    }
}

/// 从 ChannelRecord.settings.headers (JSON object) 解析覆盖头。
///
/// 非法结构 → 空列表。键名小写化以匹配后续 sanitize 的 case-insensitive 比对。
pub fn extra_headers_from_settings(settings: &serde_json::Value) -> Vec<(String, String)> {
    let Some(obj) = settings.get("headers").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(obj.len());
    for (k, v) in obj {
        let Some(s) = v.as_str() else { continue };
        out.push((k.to_ascii_lowercase(), s.to_string()));
    }
    out
}
/// 按 provider_type 准备请求 — adapter 工厂函数。
///
/// 路径规则 (new-api 一致): openai → base_url + /v1{path};
/// claude → base_url + /v1/messages; gemini → base_url + /v1beta/...
/// 鉴权头按 provider 协议族选择; 渠道 settings.headers 覆盖可改写鉴权头键名。
pub fn prepare(
    candidate: &Candidate,
    path: &str,
    provider_type: &str,
    extra_headers: Vec<(String, String)>,
) -> PreparedRequest {
    let url = build_url(provider_type, &candidate.base_url, path);
    let auth = auth_header_for(provider_type, &candidate.secret);
    PreparedRequest {
        url,
        auth_header: auth,
        extra_headers,
    }
}
/// 头过滤 — 剥离 hop-by-hop/凭据/冲突头 (new-api api_request.go 规则)。
/// - 永远剥离 (HTTP/1.1 RFC 7230 §6.1 hop-by-hop): `connection`, `keep-alive`,
///   `transfer-encoding`, `upgrade`, `proxy-*`
/// - 永远剥离凭据: `authorization`, `cookie`, `x-api-key`, `x-goog-api-key`
/// - 永远剥离冲突: `host`, `content-length` (reqwest 重新计算)
/// - 大小写不敏感; 透传其余 (含 Accept, User-Agent 等)。
pub fn sanitize_client_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    const STRIPPED: &[&str] = &[
        "connection",
        "keep-alive",
        "transfer-encoding",
        "upgrade",
        "proxy-authorization",
        "proxy-authenticate",
        "authorization",
        "cookie",
        "x-api-key",
        "x-goog-api-key",
        "host",
        "content-length",
    ];

    headers
        .iter()
        .filter(|(name, _)| {
            let lower = name.trim().to_ascii_lowercase();
            !STRIPPED.iter().any(|s| lower == *s) && !lower.starts_with("proxy-")
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}
