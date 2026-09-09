//! `node` —— `ProxyNode`：解析后的代理节点
//!
//! ponytail: 解析与转 `reqwest::Proxy` 仅做配置映射，不做拨号。
//! 真实拨号由 `forward::ReqwestEgress` 的 `reqwest::Client` 完成
//! （reqwest 已启用 `socks` feature，原生支持 HTTP CONNECT 与 SOCKS5 握手）。

use url::Url;

/// 代理协议
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProxyScheme {
    Direct,
    Http,
    Socks5,
    Vless,
    Vmess,
    Shadowsocks,
    Trojan,
}

/// HTTP/SOCKS 代理基础认证
#[derive(Debug, Clone)]
pub struct BasicAuth {
    pub user: String,
    pub pass: String,
}

/// VLESS 传输层选项（从 `vless://` URL 的 query 解析；其他协议恒为 `None`）
///
/// query 键与常见 vless 分享链接约定一致：
/// - `flow=xtls-rprx-vision` — XTLS-Vision 内层流模式
/// - `sni=<域名>` — TLS / REALITY 握手的 SNI
/// - `pbk=<64hex>` — REALITY 服务端 X25519 公钥（出现即视为 REALITY 节点）
/// - `sid=<0-16hex>` — REALITY short id（缺省全 0）
#[derive(Debug, Clone, Default)]
pub struct VlessOpts {
    /// XTLS flow，目前只识别 `xtls-rprx-vision`
    pub flow: Option<String>,
    /// TLS / REALITY SNI
    pub sni: Option<String>,
    /// REALITY 公钥（hex 64 字符），与 `short_id` 成对
    pub pbk: Option<String>,
    /// REALITY short id（hex，0-16 字符）
    pub sid: Option<String>,
}

/// 代理节点
#[derive(Debug, Clone)]
pub struct ProxyNode {
    pub id: i64,
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub auth: Option<BasicAuth>,
    /// VLESS 传输层选项；非 VLESS 节点为 `None`
    pub vless: Option<VlessOpts>,
    pub channel_keys: Vec<String>,
    pub priority: i32,
}

impl ProxyNode {
    /// 解析原始代理 URL 字符串（接受 `http://user:pass@host:port` / `socks5://...` / `socks5h://...` / `vless://...` / `vmess://...` / `ss://...` / `trojan://...`）
    ///
    /// - scheme 映射：http/https -> Http，socks5/socks5h -> Socks5，vless:// -> Vless，vmess:// -> Vmess，ss:// -> Shadowsocks，trojan:// -> Trojan
    /// - host 必填，缺失报错
    /// - port：vless/vmess/trojan 默认 443，ss 无标准默认（要求显式端口，缺失报错）
    /// - 认证：username()/password() 非空时填入 auth；只有 user 无 pass 时 pass 用空串
    /// - 返回的 id=0、channel_keys=[]、priority=0，调用方后续填充
    pub fn parse_url(url: &str) -> Result<Self, ParseError> {
        let url_obj =
            Url::parse(url).map_err(|e| ParseError::Invalid(format!("url parse failed: {}", e)))?;
        let scheme_str = url_obj.scheme();
        let scheme = match scheme_str {
            "http" | "https" => ProxyScheme::Http,
            "socks5" | "socks5h" => ProxyScheme::Socks5,
            "vless" => ProxyScheme::Vless,
            "vmess" => ProxyScheme::Vmess,
            "ss" => ProxyScheme::Shadowsocks,
            "trojan" => ProxyScheme::Trojan,
            _ => {
                return Err(ParseError::Invalid(format!(
                    "unsupported proxy scheme: {}",
                    scheme_str
                )));
            }
        };
        let host = url_obj
            .host_str()
            .ok_or_else(|| ParseError::Invalid("missing host".to_string()))?;
        // port 逻辑：
        // - vless/vmess/trojan 默认 443
        // - socks5 默认 1080
        // - http 默认 8080 (兼容)
        // - ss 必须显式端口，无默认
        let port = match scheme {
            ProxyScheme::Shadowsocks => url_obj.port().ok_or_else(|| {
                ParseError::Invalid("ss scheme requires explicit port".to_string())
            })?,
            ProxyScheme::Socks5 => url_obj.port().unwrap_or(1080),
            ProxyScheme::Vless | ProxyScheme::Vmess | ProxyScheme::Trojan => {
                url_obj.port().unwrap_or(443)
            }
            _ => url_obj.port().unwrap_or(8080), // Http or Direct (though Direct never reaches here)
        };

        // `Url::username()` / `password()` 返回 percent-encoded 原文，必须解码：
        // 用户名含 `@` 时会是 `user%40domain`，直接用会让代理认证失败。
        let auth = match url_obj.username() {
            "" => None,
            user => Some(BasicAuth {
                user: percent_decode(user),
                pass: percent_decode(url_obj.password().unwrap_or("")),
            }),
        };
        let vless = (scheme == ProxyScheme::Vless).then(|| {
            let mut opts = VlessOpts::default();
            for (k, v) in url_obj.query_pairs() {
                match k.to_string().as_str() {
                    "flow" => opts.flow = Some(v.into_owned()),
                    "sni" => opts.sni = Some(v.into_owned()),
                    "pbk" => opts.pbk = Some(v.into_owned()),
                    "sid" => opts.sid = Some(v.into_owned()),
                    _ => {}
                }
            }
            opts
        });
        Ok(Self {
            id: 0,
            scheme,
            host: host.to_string(),
            port,
            auth,
            vless,
            channel_keys: vec![],
            priority: 0,
        })
    }

    /// 转成 reqwest::Proxy（用于 ClientBuilder::proxy() 注入）
    ///
    /// - Direct -> None
    /// - Http/Socks5 -> reqwest::Proxy::all(...) + basic_auth(若有)
    /// - Vless/Vmess -> `Err(ProxyConvertError)`（走 [`super::adapter`] 的 meow 适配器）
    /// - Shadowsocks/Trojan -> `Err(ProxyConvertError)`（同上，meow `ProxyAdapter`）
    /// - 认证走 .basic_auth() 而非拼在 URL 里，避免特殊字符转义问题
    pub fn to_reqwest_proxy(&self) -> Result<Option<reqwest::Proxy>, ProxyConvertError> {
        match self.scheme {
            ProxyScheme::Direct => Ok(None),
            ProxyScheme::Http => {
                let url = format!("http://{}:{}", self.host, self.port);
                let mut proxy =
                    reqwest::Proxy::all(url).map_err(|e| ProxyConvertError(e.to_string()))?;
                if let Some(auth) = &self.auth {
                    proxy = proxy.basic_auth(&auth.user, &auth.pass);
                }
                Ok(Some(proxy))
            }
            ProxyScheme::Socks5 => {
                let url = format!("socks5://{}:{}", self.host, self.port);
                let mut proxy =
                    reqwest::Proxy::all(url).map_err(|e| ProxyConvertError(e.to_string()))?;
                if let Some(auth) = &self.auth {
                    proxy = proxy.basic_auth(&auth.user, &auth.pass);
                }
                Ok(Some(proxy))
            }
            ProxyScheme::Vless
            | ProxyScheme::Vmess
            | ProxyScheme::Shadowsocks
            | ProxyScheme::Trojan => Err(ProxyConvertError(format!(
                "scheme {:?} 不走 reqwest（用 adapter 的 meow ProxyAdapter 拨号）",
                self.scheme
            ))),
        }
    }
}

/// reqwest::Proxy 转换失败（reqwest::Error 无公开构造器，故自定义轻量错误）。
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ProxyConvertError(pub String);

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
