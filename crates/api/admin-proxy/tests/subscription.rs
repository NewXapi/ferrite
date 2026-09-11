//! `subscription` 的行为测试。
//!
//! `clash_proxy_to_url` 是纯函数，直接测；入库路径需要 PG，按
//! `tests/proxy_nodes.rs` 的惯例保留 `#[ignore = "requires DATABASE_URL"]`。
//!
//! 断言策略：**不只看字符串包含**，把生成的 URL 喂回
//! `gateway_proxy::node::ProxyNode::parse_url` 做 round-trip——URL 存进 DB 后就是
//! 被 `parse_url` 消费的，round-trip 相等才是真正要保证的契约；`contains` 断言
//! 会放过"字段在 query 里但位置错了"这类 bug。

use std::collections::HashMap;

use admin_proxy::subscription::clash_proxy_to_url;
use gateway_proxy::node::ProxyNode;
use serde_yaml::Value as Yaml;

fn s(v: &str) -> Yaml {
    Yaml::String(v.to_string())
}

fn clash_map(pairs: Vec<(&str, Yaml)>) -> HashMap<String, Yaml> {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn mapping(pairs: Vec<(&str, Yaml)>) -> Yaml {
    Yaml::Mapping(pairs.into_iter().map(|(k, v)| (s(k), v)).collect())
}

/// vless + ws + reality 的完整映射：round-trip 后每个字段都要落对位置。
///
/// UUID 落 `auth.user`（`clash_config` 对 vless 读 `auth.user` 当 uuid），
/// path 落 `ws_path`（parse_url 靠它识别 ws 节点），pbk 进 REALITY 选项。
#[test]
fn clash_vless_roundtrips_via_parse_url() {
    let proxy = clash_map(vec![
        ("type", s("vless")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("uuid", s("550e8400-e29b-41d4-a716-446655440000")),
        ("sni", s("sni.example.com")),
        ("flow", s("xtls-rprx-vision")),
        ("client-fingerprint", s("chrome")),
        (
            "reality-opts",
            mapping(vec![("public-key", s("pub")), ("short-id", s("ab"))]),
        ),
        ("network", s("ws")),
        ("ws-opts", mapping(vec![("path", s("/wspath"))])),
    ]);
    let url = clash_proxy_to_url(&proxy).expect("应构造成功");
    let node = ProxyNode::parse_url(&url).expect("URL 应能被 parse_url 消费");
    assert_eq!(node.scheme, gateway_proxy::node::ProxyScheme::Vless);
    assert_eq!(node.host, "example.com");
    assert_eq!(node.port, 443);
    let auth = node.auth.expect("应有 auth");
    assert_eq!(auth.user, "550e8400-e29b-41d4-a716-446655440000");
    let opts = node.vless.expect("应有 opts");
    assert_eq!(opts.ws_path.as_deref(), Some("/wspath"));
    assert_eq!(opts.sni.as_deref(), Some("sni.example.com"));
    assert_eq!(opts.flow.as_deref(), Some("xtls-rprx-vision"));
    assert_eq!(opts.fingerprint.as_deref(), Some("chrome"));
    assert_eq!(opts.pbk.as_deref(), Some("pub"));
    assert_eq!(opts.sid.as_deref(), Some("ab"));
}

/// ss 是唯一的双段 userinfo：cipher 在 user、password 在 pass。
///
/// 段位搞反是这类映射最典型的错误（#98 在 trojan 上踩过反向的），round-trip
/// 直接盯住两个字段的值。
#[test]
fn clash_ss_maps_cipher_and_password_as_two_segments() {
    let proxy = clash_map(vec![
        ("type", s("ss")),
        ("server", s("example.com")),
        ("port", Yaml::Number(8388.into())),
        ("cipher", s("aes-128-gcm")),
        ("password", s("ss-pass")),
    ]);
    let url = clash_proxy_to_url(&proxy).expect("应构造成功");
    let node = ProxyNode::parse_url(&url).expect("应能 parse");
    let auth = node.auth.expect("应有 auth");
    assert_eq!(auth.user, "aes-128-gcm", "cipher 落 auth.user");
    assert_eq!(auth.pass, "ss-pass", "password 落 auth.pass");
}

/// trojan 的密码是**单段** userinfo。
///
/// #98 的原始 bug 就是把 trojan 密码当两段处理，#103 补过正向回归；
/// 这里盯住反向映射不再犯。
#[test]
fn clash_trojan_maps_password_as_single_userinfo_segment() {
    let proxy = clash_map(vec![
        ("type", s("trojan")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("password", s("trojan-pass")),
    ]);
    let url = clash_proxy_to_url(&proxy).expect("应构造成功");
    let node = ProxyNode::parse_url(&url).expect("应能 parse");
    let auth = node.auth.expect("应有 auth");
    assert_eq!(auth.user, "trojan-pass", "密码落 auth.user（单段）");
    assert_eq!(auth.pass, "", "auth.pass 必须为空");
}

/// 凭据里的 URL 特殊字符必须活过 percent-encoding round-trip。
///
/// password 含 `@ : / ?` 时若不编码，拼出来的 URL 会被 parse_url 错误切段。
/// 这是引入 url crate（而不是手拼字符串）的全部理由。
#[test]
fn credentials_with_url_special_chars_survive_roundtrip() {
    let password = "p@ss:wo/rd?x";
    let proxy = clash_map(vec![
        ("type", s("trojan")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("password", s(password)),
    ]);
    let url = clash_proxy_to_url(&proxy).expect("应构造成功");
    let node = ProxyNode::parse_url(&url).expect("特殊字符密码应能 round-trip");
    assert_eq!(node.auth.expect("应有 auth").user, password);
}

/// vmess 的 cipher 落 `auth.pass`（与 parse_url 的 `uuid:cipher@` 对齐）；
/// `auto` 留空让 meow-config 取缺省。
#[test]
fn clash_vmess_cipher_lands_in_auth_pass() {
    let proxy = clash_map(vec![
        ("type", s("vmess")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("uuid", s("550e8400-e29b-41d4-a716-446655440000")),
        ("cipher", s("chacha20-poly1305")),
    ]);
    let url = clash_proxy_to_url(&proxy).expect("应构造成功");
    let node = ProxyNode::parse_url(&url).expect("应能 parse");
    let auth = node.auth.expect("应有 auth");
    assert_eq!(auth.user, "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(auth.pass, "chacha20-poly1305");

    let auto = clash_map(vec![
        ("type", s("vmess")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("uuid", s("550e8400-e29b-41d4-a716-446655440000")),
        ("cipher", s("auto")),
    ]);
    let url = clash_proxy_to_url(&auto).expect("应构造成功");
    let node = ProxyNode::parse_url(&url).expect("应能 parse");
    assert_eq!(node.auth.expect("应有 auth").pass, "");
}

/// `grpc-opts` 在 URL query 里没有对应键 → 拒绝，且原因点名该键。
///
/// 静默丢弃的后果是节点拨号成功但走错传输层——运行时表现为"连上了收不到响应"，
/// 比导入时报错难查得多。
#[test]
fn lossy_key_is_rejected_not_dropped() {
    let proxy = clash_map(vec![
        ("type", s("vless")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("uuid", s("550e8400-e29b-41d4-a716-446655440000")),
        ("grpc-opts", mapping(vec![("serviceName", s("x"))])),
    ]);
    let err = clash_proxy_to_url(&proxy).expect_err("grpc-opts 应被拒");
    assert!(err.contains("grpc-opts"), "原因应点名 grpc-opts: {err}");
}

/// 缺 `type` 无法定位协议 → Err，不 panic（订阅内容不可信）。
#[test]
fn missing_type_key_errors_without_panic() {
    let proxy = clash_map(vec![
        ("server", s("example.com")),
        ("port", Yaml::Number(1.into())),
    ]);
    assert!(clash_proxy_to_url(&proxy).is_err());
}

/// 不在 7 协议白名单的协议（tuic）→ Err：静默跳过会让
/// 「导入 40 个只进了 31 个」无法解释。
#[test]
fn unsupported_scheme_reports_failure() {
    let proxy = clash_map(vec![
        ("type", s("tuic")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("uuid", s("550e8400-e29b-41d4-a716-446655440000")),
    ]);
    let err = clash_proxy_to_url(&proxy).expect_err("tuic 应被拒");
    assert!(err.contains("tuic"), "原因应点名 tuic: {err}");
}

/// `network=ws` 而 ws-opts 无 path → 拒。
///
/// parse_url 靠 `path=` 识别 ws；不发 path 会被装配成 tcp，静默换传输层。
#[test]
fn ws_without_path_is_rejected() {
    let proxy = clash_map(vec![
        ("type", s("vless")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("uuid", s("550e8400-e29b-41d4-a716-446655440000")),
        ("network", s("ws")),
    ]);
    let err = clash_proxy_to_url(&proxy).expect_err("应拒");
    assert!(err.contains("path"), "原因应说明 path 问题: {err}");
}

/// 非空 `alpn` → 拒：URL query 没有 alpn 键，单值也带不上。
#[test]
fn non_empty_alpn_is_rejected() {
    let proxy = clash_map(vec![
        ("type", s("trojan")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("password", s("x")),
        ("alpn", Yaml::Sequence(vec![s("h2")])),
    ]);
    assert!(clash_proxy_to_url(&proxy).is_err());
}

/// 端口是字符串形态（部分订阅如此）也能解析。
#[test]
fn string_port_is_accepted() {
    let proxy = clash_map(vec![
        ("type", s("trojan")),
        ("server", s("example.com")),
        ("port", s("8443")),
        ("password", s("x")),
    ]);
    let url = clash_proxy_to_url(&proxy).expect("应构造成功");
    assert!(url.contains(":8443"), "端口应写入 URL: {url}");
}

/// **安全断言**：拒绝原因只含键名，不含凭据值。
///
/// `ImportFailure.reason` 会回前端并进日志；clash map 里的 password 是明文，
/// 错误信息里只能出现"哪个键有问题"，不能出现键的值。
#[test]
fn rejection_reason_never_contains_credential_values() {
    let proxy = clash_map(vec![
        ("type", s("trojan")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("password", s("secret-trojan-pass")),
        ("grpc-opts", mapping(vec![("serviceName", s("x"))])),
    ]);
    let err = clash_proxy_to_url(&proxy).expect_err("应拒");
    assert!(err.contains("grpc-opts"));
    assert!(!err.contains("secret-trojan-pass"), "原因不得含密码: {err}");
}

/// 入库往返：导入后 `list` 能看到节点，绑定与优先级按请求统一赋值。
/// 需要 PG，留给有 DATABASE_URL 的环境手动跑。
#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn import_persists_nodes_with_shared_binding() {
    use admin_proxy::ProxyNodeService;
    use admin_proxy::subscription::{ImportRequest, import_share_links};
    use sqlx::postgres::PgPoolOptions;

    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into());
    let pool = PgPoolOptions::new().connect(&url).await.expect("连接 PG");
    // ensure_table 已迁 sqlx migration（#112），测试里内联同样的 DDL。
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS proxy_nodes (
            key            UUID PRIMARY KEY,
            name           TEXT NOT NULL DEFAULT '',
            url            TEXT NOT NULL,
            channel_keys   JSONB NOT NULL DEFAULT '[]',
            priority       INT  NOT NULL DEFAULT 0,
            enabled        BOOL NOT NULL DEFAULT true,
            remark         TEXT NOT NULL DEFAULT '',
            created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
        );
        CREATE TABLE IF NOT EXISTS api_channels (
            key UUID PRIMARY KEY,
            name TEXT UNIQUE NOT NULL
        );",
    )
    .execute(&pool)
    .await
    .expect("建表");
    sqlx::query("INSERT INTO api_channels (key, name) VALUES (gen_random_uuid(), 'import-test-ch') ON CONFLICT (name) DO NOTHING")
        .execute(&pool)
        .await
        .expect("建渠道");

    let svc = ProxyNodeService::new(pool.clone());
    let text = "ss://YWVzLTEyOC1nY206aW1wb3J0LXBhc3NAZXhhbXBsZS5jb206ODM4OA==#导入测试\nvmess://!!!broken!!!";
    let req = ImportRequest {
        url: None,
        text: Some(text.into()),
        channel_keys: vec!["import-test-ch".into()],
        priority: 3,
    };
    let report = import_share_links(&svc, &req).await.expect("导入应成功");
    assert_eq!(report.created, 1, "一条好行应入库");
    assert_eq!(report.failures.len(), 1, "一条坏行应进报告");

    let items = svc.list(false).await.expect("list");
    let imported = items
        .iter()
        .find(|n| n.name == "导入测试")
        .expect("导入的节点应可按备注名找到");
    assert_eq!(imported.priority, 3);
    assert_eq!(imported.channel_keys, vec!["import-test-ch".to_string()]);
    // 掩码断言：列表接口永远不该吐明文凭据。
    assert!(!imported.url_masked.contains("import-pass"));
}
