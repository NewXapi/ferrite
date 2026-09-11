//! `sharelink` —— 分享链接方言解析器
//!
//! [`ProxyNode::parse_url`] 是标准 URL 解析器，吃不下两种主流方言：
//!
//! | 方言 | 形态 | 为什么 `parse_url` 不行 |
//! |---|---|---|
//! | VMess v2rayN | `vmess://BASE64(JSON)` | 载荷整块是 base64，`Url::parse` 会把它当 host |
//! | SS legacy | `ss://BASE64(method:pass@host:port)#备注` | **整串**（含 host:port）都在 base64 里 |
//! | SS SIP002 | `ss://BASE64(method:pass)@host:port#备注` | 只有 userinfo 是 base64，host 明文 |
//! | SS 明文 | `ss://method:pass@host:port` | 已支持，直接委托 |
//!
//! 把方言混进 `parse_url` 会让它从 URL 解析器退化成方言分派器，故独立成模块：
//! 按 scheme 派给专用解析器，其余 scheme 原样委托 `parse_url`。
//!
//! VMess JSON 字段到 [`ProxyNode`] 的映射（与 `adapter.rs` 头部的 auth 语义一致，
//! 写错会导致装配后认证失败）：
//!
//! - `id` → `auth.user`（UUID）
//! - `scy` → `auth.pass`（VMess security）；`auto` / 空 留空让 meow 取缺省
//! - `add` → `host`，`port` → 端口（数字或字符串两种形态都吃）
//! - `net=ws`：`path` → [`NodeOpts::ws_path`]，`host` → `ws_host`
//! - `sni` → `NodeOpts::sni`；`tls=true` 而无 `sni` 时用 `add` 兜底（TLS 必须有 SNI）
//! - `fp` → `NodeOpts::fingerprint`
//! - `aid`（alterId）：VMess AEAD 之后已废弃，meow-config 不吃这个键，解析后丢弃
//! - `net=grpc`/`h2`/`httpupgrade`：[`NodeOpts`] 没有这些传输层字段，**报错而非静默
//!   降级**——降级成 tcp 会拨号成功但走错传输，排查成本远高于导入时报错

use base64::Engine;
use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::node::{BasicAuth, NodeOpts, ParseError, ProxyNode, ProxyScheme};

/// v2rayN `vmess://` base64 载荷的 JSON 结构。
///
/// 字段名是 v2rayN 的历史缩写。`port` / `aid` 在不同客户端里可能是数字或字符串
/// （v2rayN 写数字、部分安卓客户端写字符串），故用 [`JsonValue`] 收后自行解析。
#[derive(Debug, Clone, Deserialize)]
pub struct VmessShareLink {
    /// 协议版本，固定 `"2"`。仅作记录，不参与映射。
    #[serde(rename = "v", default)]
    pub v: String,
    /// 备注 / 标签。
    #[serde(rename = "ps", default)]
    pub ps: Option<String>,
    /// 服务器地址。
    #[serde(rename = "add")]
    pub add: String,
    /// 端口，数字或字符串。
    #[serde(rename = "port")]
    pub port: JsonValue,
    /// 用户 UUID。
    #[serde(rename = "id")]
    pub id: String,
    /// alterId，VMess AEAD 后废弃，解析后丢弃。
    #[serde(rename = "aid", default)]
    pub aid: JsonValue,
    /// 加密方式（security）。
    #[serde(rename = "scy", default)]
    pub scy: String,
    /// 传输层：`tcp` / `ws` / `grpc` / `h2` / `httpupgrade`。
    #[serde(rename = "net", default)]
    pub net: String,
    /// 伪装类型（`type`），当前不参与映射。
    #[serde(rename = "type", default)]
    pub type_: String,
    /// WebSocket Host 头。
    #[serde(rename = "host", default)]
    pub host: Option<String>,
    /// WebSocket path。
    #[serde(rename = "path", default)]
    pub path: Option<String>,
    /// 是否启用 TLS。v2rayN 写 `"tls"` / `""` 字符串，也有客户端写 bool。
    #[serde(rename = "tls", default)]
    pub tls: Option<JsonValue>,
    /// TLS SNI。
    #[serde(rename = "sni", default)]
    pub sni: Option<String>,
    /// uTLS 指纹。
    #[serde(rename = "fp", default)]
    pub fp: Option<String>,
    /// ALPN 列表，当前不参与映射（`NodeOpts` 无此字段）。
    #[serde(rename = "alpn", default)]
    pub alpn: Option<JsonValue>,
}

/// 一批分享链接的解析结果：成功与失败分开归集。
///
/// 为什么不是 `Vec<Result<..>>`：调用方（订阅 / 批量导入）要分别统计与展示，
/// 拆开省一次 partition。`failures` 的第一元素是**掩码后**的原始行——原始行含
/// 密码与 UUID，不能进日志或回前端。
pub struct ShareLinkBatch {
    /// 解析成功的节点（`id` = 0、`channel_keys` 为空、`priority` = 0，由调用方填充）。
    pub nodes: Vec<ProxyNode>,
    /// 解析失败的行：`(掩码后的原始行, 失败原因)`。
    pub failures: Vec<(String, String)>,
}

/// 解析单条分享链接 → [`ProxyNode`]。
///
/// `vmess://` 走 base64-JSON 方言，`ss://` 走 legacy / SIP002 / 明文三形态，
/// 其余 scheme（vless / trojan / hysteria2 / anytls / snell / http / socks5）
/// 原样委托 [`ProxyNode::parse_url`]。
///
/// scheme 判断用字符串前缀而非先 `Url::parse`：vmess 的 base64 载荷不是合法 URL，
/// 先 parse 会直接失败。
pub fn parse_share_link(link: &str) -> Result<ProxyNode, ParseError> {
    let lower = link.trim().to_ascii_lowercase();
    if lower.starts_with("vmess://") {
        parse_vmess_dialect(link.trim())
    } else if lower.starts_with("ss://") {
        parse_ss_dialect(link.trim())
    } else {
        ProxyNode::parse_url(link.trim())
    }
}

/// 多行批量解析：一行一条，跳过空行与 `#` 开头的注释行。
///
/// 单行失败只进 `failures`，不影响整批——一个机场订阅里混一条坏链接是常态。
pub fn parse_share_links(text: &str) -> ShareLinkBatch {
    let mut batch = ShareLinkBatch {
        nodes: Vec::new(),
        failures: Vec::new(),
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match parse_share_link(line) {
            Ok(node) => batch.nodes.push(node),
            Err(e) => batch.failures.push((mask_link(line), e.to_string())),
        }
    }
    batch
}

/// 掩码一行分享链接，用于日志与错误回显：只留 scheme 与 host 骨架。
///
/// **vmess 必须特判**：`vmess://BASE64` 的载荷全是字母数字，`Url::parse` 会把它
/// 当成合法 host 原样保留——掩码结果携带整段可解开的载荷，等于没掩。vmess 的
/// authority 就是载荷本身，不解析，直接 `vmess://***`。
///
/// 其余 scheme 按 URL 掩：userinfo 与 query 是凭据所在，剥掉；host 保留便于定位。
/// 必须能处理**非法** URL 的原始行（缺 host、超长、怪字符）——兜底只回
/// `<scheme>://***`，绝不回原文。
pub fn mask_link(link: &str) -> String {
    let link = link.trim();
    let scheme = link
        .split_once("://")
        .map(|(s, _)| s.to_ascii_lowercase())
        .unwrap_or_else(|| "?".to_string());
    if scheme == "vmess" {
        return "vmess://***".to_string();
    }
    match url::Url::parse(link) {
        Ok(u) => match u.host_str() {
            Some(host) => {
                let port = u.port().map(|p| format!(":{p}")).unwrap_or_default();
                format!("{scheme}://***@{host}{port}")
            }
            None => format!("{scheme}://***"),
        },
        Err(_) => format!("{scheme}://***"),
    }
}

/// `vmess://BASE64(JSON)` → [`ProxyNode`]（映射约定见模块头）。
fn parse_vmess_dialect(link: &str) -> Result<ProxyNode, ParseError> {
    let payload = link
        .get("vmess://".len()..)
        .ok_or_else(|| invalid("vmess 链接缺少载荷"))?;
    // 分享链接常把 `#备注` 附在 base64 后面。
    let payload = payload.split('#').next().unwrap_or(payload);
    let json = decode_base64(payload).map_err(|e| invalid(&format!("vmess 载荷 {e}")))?;
    let v: VmessShareLink = serde_json::from_str(&json)
        .map_err(|e| invalid(&format!("vmess 载荷不是合法 JSON: {e}")))?;

    if v.add.trim().is_empty() {
        return Err(invalid("vmess 载荷缺少 add（服务器地址）"));
    }
    if v.id.trim().is_empty() {
        return Err(invalid("vmess 载荷缺少 id（UUID）"));
    }
    let port = json_port(&v.port).ok_or_else(|| invalid("vmess 载荷的 port 非法"))?;

    // net 空串按 v2rayN 惯例视作 tcp。
    let net = v.net.trim().to_ascii_lowercase();
    let net = if net.is_empty() { "tcp" } else { net.as_str() };
    let tls = json_truthy(v.tls.as_ref());

    let mut opts = NodeOpts::default();
    match net {
        "tcp" => {}
        "ws" => {
            // path 缺省 `/`：clash 的 ws-opts 需要 path，分享链接常省略。
            opts.ws_path = Some(
                v.path
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("/")
                    .to_string(),
            );
            opts.ws_host = v
                .host
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
        }
        other => {
            return Err(invalid(&format!(
                "vmess 传输层 `{other}` 暂不支持（NodeOpts 无对应字段，静默降级会走错传输）"
            )));
        }
    }
    opts.sni = v
        .sni
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if tls && opts.sni.is_none() {
        // TLS 必须有 SNI，分享链接省略时用服务器地址兜底（与主流客户端一致）。
        opts.sni = Some(v.add.trim().to_string());
    }
    opts.fingerprint =
        v.fp.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

    // scy=auto / 空 时留空，让 meow-config 取它自己的缺省。
    let scy = v.scy.trim();
    let cipher = if scy.is_empty() || scy.eq_ignore_ascii_case("auto") {
        String::new()
    } else {
        scy.to_string()
    };

    Ok(ProxyNode {
        id: 0,
        scheme: ProxyScheme::Vmess,
        host: v.add.trim().to_string(),
        port,
        auth: Some(BasicAuth {
            user: v.id.trim().to_string(),
            pass: cipher,
        }),
        opts: Some(opts),
        channel_keys: vec![],
        priority: 0,
    })
}

/// `ss://` 三形态 → [`ProxyNode`]。
///
/// 判别顺序：先当 SIP002 / 明文（`@` 在 base64 之外，`Url::parse` 能拿到 host），
/// 拿不到 host 再按 legacy 解整串 base64 后重新解析。反过来做会把 SIP002 的
/// base64 userinfo 当成整串解，得到一段乱码。
fn parse_ss_dialect(link: &str) -> Result<ProxyNode, ParseError> {
    let body = link
        .get("ss://".len()..)
        .ok_or_else(|| invalid("ss 链接缺少载荷"))?;
    let body = body.split('#').next().unwrap_or(body);

    if let Some((userinfo, hostpart)) = body.rsplit_once('@') {
        // SIP002：userinfo 是 base64(method:password)；明文形态则直接是 method:password。
        let creds = match decode_base64(userinfo) {
            Ok(decoded) if decoded.contains(':') => decoded,
            // 解不出或解出来没有 `:`，说明本就是明文 userinfo。
            _ => percent_decode(userinfo),
        };
        let (cipher, password) = split_creds(&creds)?;
        // query（plugin= 等）跟在 host 后面，剥掉再拆 host:port。
        let hostpart = hostpart.split('?').next().unwrap_or(hostpart);
        let (host, port) = split_host_port(hostpart)?;
        return Ok(ss_node(cipher, password, host, port));
    }

    // legacy：整串 base64(method:password@host:port)
    let decoded = decode_base64(body).map_err(|e| invalid(&format!("ss 载荷 {e}")))?;
    let (creds, hostpart) = decoded
        .rsplit_once('@')
        .ok_or_else(|| invalid("ss legacy 载荷缺少 `@host:port`"))?;
    let (cipher, password) = split_creds(creds)?;
    let (host, port) = split_host_port(hostpart)?;
    Ok(ss_node(cipher, password, host, port))
}
/// 组装 ss 节点：`auth.user` = cipher、`auth.pass` = password（见 `adapter.rs`）。
fn ss_node(cipher: &str, password: &str, host: &str, port: u16) -> ProxyNode {
    ProxyNode {
        id: 0,
        scheme: ProxyScheme::Shadowsocks,
        host: host.to_string(),
        port,
        auth: Some(BasicAuth {
            user: cipher.to_string(),
            pass: password.to_string(),
        }),
        opts: None,
        channel_keys: vec![],
        priority: 0,
    }
}

/// 拆 `method:password`。password 可含 `:`，故只在第一个冒号处切。
fn split_creds(creds: &str) -> Result<(&str, &str), ParseError> {
    let (cipher, password) = creds
        .split_once(':')
        .ok_or_else(|| invalid("ss 凭据不是 `method:password` 形式"))?;
    if cipher.is_empty() {
        return Err(invalid("ss 缺少加密方法"));
    }
    Ok((cipher, password))
}

/// 拆 `host:port`，兼容 IPv6 字面量。ss 无标准默认端口，缺端口即报错。
fn split_host_port(hostpart: &str) -> Result<(&str, u16), ParseError> {
    let (host, port) = hostpart
        .rsplit_once(':')
        .ok_or_else(|| invalid("ss 缺少显式端口"))?;
    let host = host.strip_prefix('[').unwrap_or(host);
    let host = host.strip_suffix(']').unwrap_or(host);
    if host.is_empty() {
        return Err(invalid("ss 缺少服务器地址"));
    }
    let port: u16 = port
        .parse()
        .map_err(|_| invalid(&format!("ss 端口 `{port}` 非法")))?;
    Ok((host, port))
}

/// 解 base64：容忍缺 padding，标准与 URL-safe 两种字母表都试。
///
/// 分享链接里 padding 普遍被省略（`=` 在 URL 里要转义），且客户端在两种字母表之间
/// 不统一，所以两者都要试。解出的字节必须是合法 UTF-8（载荷是 JSON 或 ASCII 凭据）。
fn decode_base64(s: &str) -> Result<String, String> {
    let trimmed = s.trim().trim_end_matches('=');
    let bytes = STANDARD_NO_PAD
        .decode(trimmed)
        .or_else(|_| URL_SAFE_NO_PAD.decode(trimmed))
        .map_err(|e| format!("不是合法 base64: {e}"))?;
    String::from_utf8(bytes).map_err(|_| "base64 解出的内容不是合法 UTF-8".to_string())
}

/// v2rayN 的 `port` 字段可能是数字或字符串，两种都解成 `u16`。
fn json_port(v: &JsonValue) -> Option<u16> {
    match v {
        JsonValue::Number(n) => u16::try_from(n.as_u64()?).ok(),
        JsonValue::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// `tls` 字段的真值判定：v2rayN 写 `"tls"` / `""`，也有客户端写 `true` / `false`。
fn json_truthy(v: Option<&JsonValue>) -> bool {
    match v {
        Some(JsonValue::Bool(b)) => *b,
        Some(JsonValue::String(s)) => {
            let s = s.trim();
            !s.is_empty() && !s.eq_ignore_ascii_case("none")
        }
        _ => false,
    }
}

fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

fn invalid(msg: &str) -> ParseError {
    ParseError::Invalid(msg.to_string())
}
