//! `sharelink` 单元测试骨架
//!
//! 场景登记：每个 `#[test] #[ignore = "TODO(#111): 骨架未实现"]` 函数体用 `todo!()`，
//! 注释写清预期与理由，供实现者在填充实现时参考。

use gateway_proxy::sharelink::{ShareLinkBatch, parse_share_link, parse_share_links};

/// 固定被测符号的引用，让骨架阶段的 import 不触发 unused（实现时删掉本函数）。
fn _skeleton_symbol_refs() {
    let _: fn(&str) -> _ = parse_share_link;
    let _: fn(&str) -> ShareLinkBatch = parse_share_links;
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn vmess_base64_json_roundtrip() {
    // 预期：vmess://base64(json) 能正确解析出 host、port、uuid(id)、aid、scy、net=ws、path、sni、fp 等，
    // 并映射到 ProxyNode：auth.user=uuid，auth.pass=security(scy)，vless.ws_path=path，vless.sni=sni，
    // vless.fingerprint=fp。WS 传输层字段留给 meow-config 装配。
    todo!("TODO(#111): vmess base64 json 往返解析")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn vmess_port_as_string() {
    // 预期：vmess 载荷里 port 可能是字符串 "443"，serde_json::Value 收后自行解析，
    // 两者都能得到 u16 端口。
    todo!("TODO(#111): vmess port 字符串形态")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn ss_legacy_base64() {
    // 预期：ss://base64(method:password)@host:port#备注 能解析出 cipher=method、pass=password、host、port。
    // auth.user=cipher，auth.pass=password。
    todo!("TODO(#111): ss legacy base64 形态")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn ss_sip002() {
    // 预期：ss://base64(method:password)@host:port 或 ss://cipher:pass@host:port (明文) 两种形态均支持。
    // base64 形态与 legacy 相同，明文形态直接走 userinfo 解码。
    todo!("TODO(#111): ss SIP002 双形态")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn base64_padding_missing() {
    // 预期：分享链接常见 base64 padding 缺失（末尾少 `=`），解码器应补全再解码，不应 panic。
    todo!("TODO(#111): base64 padding 缺失容错")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn bad_base64_error() {
    // 预期：非法 base64 载荷（如含非法字符、长度错误）返回 ParseError，不 panic。
    todo!("TODO(#111): 坏 base64 返回错误而非 panic")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn batch_mixed_bad_line() {
    // 预期：多行文本里混一行坏链接，其余行仍成功解析，ShareLinkBatch.nodes 包含成功的，
    // failures 包含掩码后的坏行与原因。
    todo!("TODO(#111): 批量解析混入坏行仍成功")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn ignore_comments_and_empty_lines() {
    // 预期：以 `#` 开头的注释行与空行被忽略，不计入 failures，不产生节点。
    todo!("TODO(#111): 注释行与空行被忽略")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn vmess_uuid_maps_to_auth_user() {
    // 预期：vmess 载荷的 `id` 字段（UUID）落在 ProxyNode.auth.user，
    // 对齐 adapter.rs 头部注释约定（VLESS/VMess 的 auth.user = UUID）。
    todo!("TODO(#111): vmess uuid -> auth.user")
}
