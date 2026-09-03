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

/// 按 provider_type 准备请求 — adapter 工厂函数。
///
/// 路径规则 (new-api 一致): openai → base_url + /v1{path};
/// claude → base_url + /v1/messages; gemini → base_url + /v1beta/...
/// TODO(#521): 各 provider 路径模板表 + header 模板 (含 client 头透传白名单)。
pub fn prepare(candidate: &Candidate, path: &str) -> PreparedRequest {
    let _ = (candidate, path);
    PreparedRequest {
        url: String::new(),
        auth_header: (String::new(), String::new()),
        extra_headers: Vec::new(),
    }
}

/// 头过滤 — 剥离 hop-by-hop/凭据/冲突头 (new-api api_request.go 规则)。
pub fn sanitize_client_headers(_headers: &[(String, String)]) -> Vec<(String, String)> {
    Vec::new() // TODO(#521): hop-by-hop / authorization / cookie / host / content-length 剥离
}
