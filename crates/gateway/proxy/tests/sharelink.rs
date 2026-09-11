//! 分享链接方言解析的行为测试。
//!
//! base64 载荷一律现场编码生成，不写死长串——写死了看不出测的是什么内容，
//! 改一个字段还得手工重编码。凭据用明显的假值（`test-pass`、全 0 UUID 段）。

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use gateway_proxy::node::ProxyScheme;
use gateway_proxy::sharelink::{parse_share_link, parse_share_links};

const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
const PASS: &str = "test-pass";

/// 造 `vmess://BASE64(JSON)`，JSON 由字段片段拼出。
fn vmess_link(extra: &str) -> String {
    let json = format!(
        r#"{{"v":"2","ps":"节点","add":"example.com","port":443,"id":"{UUID}","aid":0,"scy":"auto"{extra}}}"#
    );
    format!("vmess://{}", STANDARD.encode(json))
}

/// VMess 载荷映射：UUID 落 `auth.user`，host/port 就位。
///
/// UUID 位置是最易错的点——`adapter.rs::clash_config` 对 vmess 读的是 `auth.user`
/// 当 `uuid`，写到 `auth.pass` 会让装配出的适配器认证失败（#98 在 trojan 上踩过同类）。
#[test]
fn vmess_maps_uuid_into_auth_user() {
    let node = parse_share_link(&vmess_link(r#","net":"tcp""#)).expect("应解析成功");
    assert_eq!(node.scheme, ProxyScheme::Vmess);
    assert_eq!(node.host, "example.com");
    assert_eq!(node.port, 443);
    let auth = node.auth.expect("vmess 必须有 auth");
    assert_eq!(auth.user, UUID, "UUID 应落在 auth.user");
    assert_eq!(auth.pass, "", "scy=auto 应留空让 meow 取缺省 cipher");
}

/// `scy` 是具体算法时写入 `auth.pass`（clash 的 `cipher` 键）。
///
/// 自建不含默认 `scy` 的载荷：helper 里的 `"scy":"auto"` 会造成 JSON 重复键，
/// serde derive 对重复字段是**报错**而不是取后者（与 Map 的覆盖语义不同）。
#[test]
fn vmess_explicit_cipher_lands_in_auth_pass() {
    let json = format!(
        r#"{{"v":"2","add":"example.com","port":443,"id":"{UUID}","scy":"chacha20-poly1305","net":"tcp"}}"#
    );
    let node =
        parse_share_link(&format!("vmess://{}", STANDARD.encode(&json))).expect("应解析成功");
    assert_eq!(node.auth.unwrap().pass, "chacha20-poly1305");
}

/// `port` 是字符串形态（部分安卓客户端如此）也要能解。
#[test]
fn vmess_accepts_string_port() {
    let json = format!(
        r#"{{"v":"2","add":"example.com","port":"8443","id":"{UUID}","net":"tcp","scy":"auto"}}"#
    );
    let node = parse_share_link(&format!("vmess://{}", STANDARD.encode(json))).expect("应解析成功");
    assert_eq!(node.port, 8443);
}

/// `net=ws` 映射到 `ws_path` / `ws_host`；`sni` 与 `fp` 各就各位。
#[test]
fn vmess_ws_transport_maps_path_host_sni_fp() {
    let link = vmess_link(
        r#","net":"ws","path":"/vm","host":"cdn.example.com","tls":"tls","sni":"sni.example.com","fp":"chrome""#,
    );
    let opts = parse_share_link(&link)
        .expect("应解析成功")
        .vless
        .expect("ws 节点应有 VlessOpts");
    assert_eq!(opts.ws_path.as_deref(), Some("/vm"));
    assert_eq!(opts.ws_host.as_deref(), Some("cdn.example.com"));
    assert_eq!(opts.sni.as_deref(), Some("sni.example.com"));
    assert_eq!(opts.fingerprint.as_deref(), Some("chrome"));
}

/// `net=ws` 省略 path 时缺省 `/`——clash 的 ws-opts 需要 path，分享链接常省。
#[test]
fn vmess_ws_without_path_defaults_to_slash() {
    let opts = parse_share_link(&vmess_link(r#","net":"ws""#))
        .expect("应解析成功")
        .vless
        .expect("应有 VlessOpts");
    assert_eq!(opts.ws_path.as_deref(), Some("/"));
}

/// `tls=true` 但没给 `sni` 时用服务器地址兜底：TLS 握手必须有 SNI。
#[test]
fn vmess_tls_without_sni_falls_back_to_server_address() {
    let opts = parse_share_link(&vmess_link(r#","net":"tcp","tls":"tls""#))
        .expect("应解析成功")
        .vless
        .expect("应有 VlessOpts");
    assert_eq!(opts.sni.as_deref(), Some("example.com"));
}

/// `net=grpc` 报错而非静默降级。
///
/// `VlessOpts` 没有 grpc 字段，降级成 tcp 会拨号成功但走错传输层——那种失败
/// 在运行时表现为"连上了但收不到响应"，比导入时报错难查得多。
#[test]
fn vmess_unsupported_transport_errors() {
    let err = parse_share_link(&vmess_link(r#","net":"grpc""#))
        .expect_err("grpc 传输层应报错")
        .to_string();
    assert!(err.contains("grpc"), "错误应点名 grpc，实际: {err}");
}

/// 缺 UUID 的载荷报错（没有 id 就无法认证）。
#[test]
fn vmess_missing_uuid_errors() {
    let json = r#"{"v":"2","add":"example.com","port":443,"id":"","net":"tcp"}"#;
    assert!(parse_share_link(&format!("vmess://{}", STANDARD.encode(json))).is_err());
}

/// 坏 base64 返回错误而不 panic：订阅内容不可信。
#[test]
fn vmess_bad_base64_errors_without_panic() {
    assert!(parse_share_link("vmess://!!!not-base64!!!").is_err());
}

/// base64 解得开但不是 JSON 时报错。
#[test]
fn vmess_non_json_payload_errors() {
    let link = format!("vmess://{}", STANDARD.encode("这不是 JSON"));
    assert!(parse_share_link(&link).is_err());
}

/// SS legacy：**整串** base64（含 host:port）。
///
/// 这是与 SIP002 的关键区别——legacy 把 `@host:port` 一起编码进 base64，
/// 判别顺序搞反会把 SIP002 的 userinfo 当整串解出乱码。
#[test]
fn ss_legacy_whole_string_base64() {
    let payload = STANDARD.encode(format!("aes-128-gcm:{PASS}@example.com:8388"));
    let node = parse_share_link(&format!("ss://{payload}#备注")).expect("应解析成功");
    assert_eq!(node.scheme, ProxyScheme::Shadowsocks);
    assert_eq!(node.host, "example.com");
    assert_eq!(node.port, 8388);
    let auth = node.auth.expect("ss 必须有 auth");
    assert_eq!(auth.user, "aes-128-gcm", "cipher 落 auth.user");
    assert_eq!(auth.pass, PASS, "password 落 auth.pass");
}

/// SS SIP002：只有 userinfo 是 base64，host:port 明文。
#[test]
fn ss_sip002_base64_userinfo_with_plain_host() {
    let userinfo = STANDARD.encode(format!("chacha20-ietf-poly1305:{PASS}"));
    let node = parse_share_link(&format!("ss://{userinfo}@example.com:8389")).expect("应解析成功");
    assert_eq!(node.host, "example.com");
    assert_eq!(node.port, 8389);
    let auth = node.auth.unwrap();
    assert_eq!(auth.user, "chacha20-ietf-poly1305");
    assert_eq!(auth.pass, PASS);
}

/// SIP002 的 `?plugin=` query 不能被当成 host 的一部分。
#[test]
fn ss_sip002_strips_plugin_query_from_host() {
    let userinfo = STANDARD_NO_PAD.encode(format!("aes-256-gcm:{PASS}"));
    let node = parse_share_link(&format!(
        "ss://{userinfo}@example.com:8390?plugin=obfs-local"
    ))
    .expect("应解析成功");
    assert_eq!(node.host, "example.com");
    assert_eq!(node.port, 8390);
}

/// 明文 userinfo 形态（`ss://method:pass@host:port`）。
#[test]
fn ss_plain_userinfo() {
    let node =
        parse_share_link(&format!("ss://aes-128-gcm:{PASS}@example.com:8391")).expect("应解析成功");
    let auth = node.auth.unwrap();
    assert_eq!(auth.user, "aes-128-gcm");
    assert_eq!(auth.pass, PASS);
}

/// 密码含 `:` 时只在第一个冒号处切（密码里的冒号是合法的）。
#[test]
fn ss_password_containing_colon_is_preserved() {
    let userinfo = STANDARD.encode("aes-128-gcm:pa:ss:word");
    let node = parse_share_link(&format!("ss://{userinfo}@example.com:8392")).expect("应解析成功");
    assert_eq!(node.auth.unwrap().pass, "pa:ss:word");
}

/// 缺 padding 的 base64 要能解：`=` 在 URL 里要转义，分享链接普遍省略。
#[test]
fn ss_base64_without_padding_decodes() {
    let payload = STANDARD_NO_PAD.encode(format!("aes-128-gcm:{PASS}@example.com:8393"));
    assert!(!payload.ends_with('='), "本用例要测无 padding 输入");
    let node = parse_share_link(&format!("ss://{payload}")).expect("无 padding 应能解");
    assert_eq!(node.port, 8393);
}

/// URL-safe 字母表的 base64 也要能解（客户端间不统一）。
#[test]
fn ss_url_safe_alphabet_decodes() {
    // `?` 与 `~` 编码后会产出 `-`/`_`，正好落在两套字母表的差异位上。
    let creds = "aes-128-gcm:p?a~ss@example.com:8394";
    let payload = URL_SAFE_NO_PAD.encode(creds);
    let node = parse_share_link(&format!("ss://{payload}")).expect("URL-safe 应能解");
    assert_eq!(node.host, "example.com");
    assert_eq!(node.port, 8394);
}

/// ss 无标准默认端口，缺端口必须报错而不是猜一个。
#[test]
fn ss_missing_port_errors() {
    let payload = STANDARD.encode(format!("aes-128-gcm:{PASS}@example.com"));
    assert!(parse_share_link(&format!("ss://{payload}")).is_err());
}

/// 非方言 scheme 委托 `parse_url`：vless 的 query 选项应被解出。
#[test]
fn non_dialect_scheme_delegates_to_parse_url() {
    let node = parse_share_link(&format!(
        "vless://{UUID}@example.com:443?sni=a.example.com&type=ws&path=/x"
    ))
    .expect("vless 应由 parse_url 处理");
    assert_eq!(node.scheme, ProxyScheme::Vless);
    let opts = node.vless.expect("vless 应有 opts");
    assert_eq!(opts.sni.as_deref(), Some("a.example.com"));
    assert_eq!(opts.ws_path.as_deref(), Some("/x"));
}

/// 批量解析：坏行进 failures，好行照常入 nodes。
#[test]
fn batch_isolates_bad_lines() {
    let good = format!(
        "ss://{}",
        STANDARD.encode(format!("aes-128-gcm:{PASS}@a.example.com:1"))
    );
    let text = format!("{good}\nvmess://!!!broken!!!\n{good}");
    let batch = parse_share_links(&text);
    assert_eq!(batch.nodes.len(), 2, "两条好行都应入库");
    assert_eq!(batch.failures.len(), 1, "一条坏行应被隔离");
}

/// 空行与 `#` 注释行被忽略，既不产生节点也不记为失败。
#[test]
fn batch_ignores_blank_and_comment_lines() {
    let good = format!(
        "ss://{}",
        STANDARD.encode(format!("aes-128-gcm:{PASS}@a.example.com:1"))
    );
    let batch = parse_share_links(&format!("\n# 这是注释\n  \n{good}\n"));
    assert_eq!(batch.nodes.len(), 1);
    assert!(batch.failures.is_empty(), "注释与空行不该记为失败");
}

/// **安全断言**：失败项回显的原始行必须掩码，不能泄露密码。
///
/// `failures` 会进日志并回前端。vmess 的坏载荷解不出 host，这种情况只能回
/// `vmess://***`——绝不能把含密码的原文塞回去。
#[test]
fn batch_failure_source_never_leaks_credentials() {
    // 载荷 base64 解得开但 JSON 缺 id，失败时原文含密码明文。
    let json = format!(r#"{{"add":"example.com","port":443,"id":"","net":"tcp","scy":"{PASS}"}}"#);
    let link = format!("vmess://{}", STANDARD.encode(&json));
    let batch = parse_share_links(&link);
    assert_eq!(batch.failures.len(), 1);
    let (masked, _reason) = &batch.failures[0];
    assert!(!masked.contains(PASS), "掩码后不得含密码，实际: {masked}");
    assert!(
        !masked.contains(&STANDARD.encode(&json)),
        "掩码后不得含原始 base64 载荷（它可被解开），实际: {masked}"
    );
    assert!(masked.starts_with("vmess://"), "应保留 scheme 便于定位");
}

/// 掩码保留 host 便于定位，但去掉 userinfo。
#[test]
fn masked_link_keeps_host_drops_userinfo() {
    let batch = parse_share_links(&format!("ss://aes-128-gcm:{PASS}@example.com"));
    assert_eq!(batch.failures.len(), 1, "缺端口应失败");
    let (masked, _) = &batch.failures[0];
    assert!(!masked.contains(PASS), "不得含密码: {masked}");
    assert!(
        masked.contains("example.com"),
        "应保留 host 便于定位: {masked}"
    );
}
