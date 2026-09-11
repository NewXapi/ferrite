//! `subscription` 骨架的场景登记。
//!
//! 每个 `#[ignore]` 用例是实现 PR 必须覆盖的一个场景，注释写清测什么行为、
//! 为什么是这个预期。实现时把 `todo!()` 换成真断言并去掉 `#[ignore]`。
//!
//! clash map → URL 是纯函数，不需要 DB；入库路径的测试按
//! `tests/proxy_nodes.rs` 的惯例（`#[ignore = "requires DATABASE_URL"]`）。

use std::collections::HashMap;

use admin_proxy::subscription::clash_proxy_to_url;
use serde_yaml::Value as Yaml;

/// 造一份最小 clash proxy map。
fn clash_map(pairs: &[(&str, Yaml)]) -> HashMap<String, Yaml> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn s(v: &str) -> Yaml {
    Yaml::String(v.to_string())
}

/// vless + ws-opts + reality-opts 应映射成带 type/path/pbk/sid query 的 URL。
///
/// 预期 UUID 落在 userinfo（`vless://uuid@host`）——与 `gateway_proxy::adapter`
/// 的 auth 语义反向一致，写错会导致装配后认证失败。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn clash_vless_maps_uuid_into_userinfo() {
    let _proxy = clash_map(&[
        ("type", s("vless")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("uuid", s("550e8400-e29b-41d4-a716-446655440000")),
    ]);
    todo!("TODO(#111): 断言 URL 形如 vless://<uuid>@example.com:443 且 ws/reality query 齐全")
}

/// ss 的 cipher 与 password 是**两段** userinfo（`ss://cipher:password@`）。
///
/// 这是 ss 与其他协议的唯一差异点，单独立一个用例盯住。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn clash_ss_maps_cipher_and_password_as_two_segments() {
    let _proxy = clash_map(&[
        ("type", s("ss")),
        ("server", s("example.com")),
        ("port", Yaml::Number(8388.into())),
        ("cipher", s("aes-128-gcm")),
        ("password", s("pass")),
    ]);
    todo!("TODO(#111): 断言 URL 形如 ss://aes-128-gcm:pass@example.com:8388")
}

/// trojan 的密码是**单段** userinfo（`trojan://password@`），不是 `:password@`。
///
/// #98 真节点验证在这里踩过一次（当时按两段处理导致认证失败），#103 补了正向回归；
/// 这里盯住反向映射不要再犯。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn clash_trojan_maps_password_as_single_userinfo_segment() {
    let _proxy = clash_map(&[
        ("type", s("trojan")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("password", s("secret")),
    ]);
    todo!("TODO(#111): 断言 URL 形如 trojan://secret@example.com:443（无冒号分段）")
}

/// `grpc-opts` 在我们的 URL query 里没有对应键 → 必须报错，不能静默丢。
///
/// 丢掉传输层配置的节点会拨号成功但走错传输，排查成本远高于导入时报错。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn lossy_key_is_rejected_not_dropped() {
    let _proxy = clash_map(&[
        ("type", s("vless")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
        ("uuid", s("550e8400-e29b-41d4-a716-446655440000")),
        ("grpc-opts", Yaml::Mapping(Default::default())),
    ]);
    todo!("TODO(#111): 断言返回 Err 且原因里点名 grpc-opts")
}

/// 缺 `type` 无法定位协议 → Err，且不 panic（订阅内容不可信）。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn missing_type_key_errors_without_panic() {
    let _proxy = clash_map(&[("server", s("example.com"))]);
    let _ = clash_proxy_to_url;
    todo!("TODO(#111): 断言 Err")
}

/// 不在 `ProxyScheme` 白名单的协议（tuic / wireguard）→ Err 而非静默跳过。
///
/// 静默跳过会让「导入 40 个只进了 31 个」无法解释。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn unsupported_scheme_reports_failure() {
    let _proxy = clash_map(&[
        ("type", s("tuic")),
        ("server", s("example.com")),
        ("port", Yaml::Number(443.into())),
    ]);
    todo!("TODO(#111): 断言 Err 且原因说明协议不支持")
}

/// **安全断言**：`ImportReport::failures` 的 `source` 不得含明文凭据。
///
/// 这个报告会回到前端并进日志；漏了掩码等于把机场密码写进浏览器控制台。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn import_failure_source_is_masked() {
    todo!("TODO(#111): 构造一条含密码的失败项，断言 report.failures[0].source 不含该密码")
}

/// 订阅里的 `proxy-groups` / `rules` 应被忽略且不报错。
///
/// 我们的选点语义不需要 clash 的组与规则；带这两段的正常订阅必须能导入。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn subscription_groups_and_rules_are_ignored() {
    todo!("TODO(#111): 用带 proxy-groups/rules 的 YAML 断言 proxies 正常导入")
}

/// 请求歧义（url 与 text 同时给或同时缺）→ 400，不猜意图。
#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn ambiguous_request_is_rejected() {
    todo!("TODO(#111): 断言 url+text 同时给 → ServiceError::BadRequest；同时缺也是")
}

/// 入库往返：导入后 `list` 能看到节点，且 `channel_keys` / `priority` 按请求统一赋值。
#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn import_persists_nodes_with_shared_binding() {
    todo!("TODO(#111): 建表 → 导入两节点 → list 断言条数与绑定字段")
}
