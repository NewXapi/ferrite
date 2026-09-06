//! `node` —— `ProxyNode`：解析后的代理节点
//!
//! ponytail: 解析与转 `reqwest::Proxy` 仅做配置映射，不做拨号。
//! 真实拨号由 `forward::ReqwestEgress` 的 `reqwest::Client` 完成
//! （reqwest 已启用 `socks` feature，原生支持 HTTP CONNECT 与 SOCKS5 握手）。

use url::Url;

/// 代理协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyScheme {
    Direct,
    Http,
    Socks5,
}

/// HTTP/SOCKS 代理基础认证
#[derive(Debug, Clone)]
pub struct BasicAuth {
    pub user: String,
    pub pass: String,
}

/// 代理节点
#[derive(Debug, Clone)]
pub struct ProxyNode {
    pub id: i64,
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub auth: Option<BasicAuth>,
    pub channel_ids: Vec<i64>,
    pub priority: i32,
}

impl ProxyNode {
    /// 解析原始代理 URL 字符串（接受 `http://user:pass@host:port` / `socks5://...` / `socks5h://...`）
    ///
    /// - scheme 映射：http/https -> Http，socks5/socks5h -> Socks5
    /// - host 必填，缺失报错
    /// - port：URL 显式给出用显式值；否则 Http 默认 8080，Socks5 默认 1080
    /// - 认证：username()/password() 非空时填入 auth；只有 user 无 pass 时 pass 用空串
    /// - 返回的 id=0、channel_ids=[]、priority=0，调用方后续填充
    pub fn parse_url(url: &str) -> Result<Self, ParseError> {
        let url_obj = Url::parse(url)
            .map_err(|e| ParseError::Invalid(format!("url parse failed: {}", e)))?;
        let scheme_str = url_obj.scheme();
        let scheme = match scheme_str {
            "http" | "https" => ProxyScheme::Http,
            "socks5" | "socks5h" => ProxyScheme::Socks5,
            _ => return Err(ParseError::Invalid(format!("unsupported proxy scheme: {}", scheme_str))),
        };
        let host = url_obj.host_str()
            .ok_or_else(|| ParseError::Invalid("missing host".to_string()))?;
        let port = url_obj.port().unwrap_or_else(|| {
            match scheme {
                ProxyScheme::Http => 8080,
                ProxyScheme::Socks5 => 1080,
                ProxyScheme::Direct => 0,
            }
        });
        // `Url::username()` / `password()` 返回 percent-encoded 原文，必须解码：
        // 用户名含 `@` 时会是 `user%40domain`，直接用会让代理认证失败。
        let auth = match url_obj.username() {
            "" => None,
            user => Some(BasicAuth {
                user: percent_decode(user),
                pass: percent_decode(url_obj.password().unwrap_or("")),
            }),
        };
        Ok(Self {
            id: 0,
            scheme,
            host: host.to_string(),
            port,
            auth,
            channel_ids: vec![],
            priority: 0,
        })
    }

    /// 转成 reqwest::Proxy（用于 ClientBuilder::proxy() 注入）
    ///
    /// - Direct -> None
    /// - Http/Socks5 -> reqwest::Proxy::all(...) + basic_auth(若有)
    /// - 注意：认证走 .basic_auth() 而非拼在 URL 里，避免特殊字符转义问题
    pub fn to_reqwest_proxy(&self) -> Result<Option<reqwest::Proxy>, reqwest::Error> {
        match self.scheme {
            ProxyScheme::Direct => Ok(None),
            ProxyScheme::Http => {
                let url = format!("http://{}:{}", self.host, self.port);
                let mut proxy = reqwest::Proxy::all(url)?;
                if let Some(auth) = &self.auth {
                    proxy = proxy.basic_auth(&auth.user, &auth.pass);
                }
                Ok(Some(proxy))
            },
            ProxyScheme::Socks5 => {
                let url = format!("socks5://{}:{}", self.host, self.port);
                let mut proxy = reqwest::Proxy::all(url)?;
                if let Some(auth) = &self.auth {
                    proxy = proxy.basic_auth(&auth.user, &auth.pass);
                }
                Ok(Some(proxy))
            },
        }
    }
}

/// percent-decode 一段 URL 组件，非法 UTF-8 序列按 lossy 处理。
fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid proxy url: {0}")]
    Invalid(String),
}
