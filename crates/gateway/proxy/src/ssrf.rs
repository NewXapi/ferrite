//! `ssrf` —— SSRF 防护
//!
//! 双重 IP 校验：解析时（URL 解析出的 IP 字面量）+ 拨号时（实际 DNS 解析结果）。
//! 域名由调用方 DNS 解析后再校验；`validate_url` 仅做 IP 字面量校验。
//!
//! IPv4-mapped IPv6（`::ffff:a.b.c.d`）绕过 loopback 检查，必须拦截。
//! CGNAT（100.64.0.0/10）为保留地址段，亦应拦截。

use std::net::IpAddr;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum SsrfError {
    #[error("loopback address not allowed")]
    Loopback,
    #[error("private ip not allowed")]
    PrivateIp,
    #[error("link-local address not allowed")]
    LinkLocal,
    #[error("multicast address not allowed")]
    Multicast,
    #[error("unspecified address not allowed")]
    Unspecified,
    #[error("dns resolution failed: {0}")]
    Dns(String),
}

/// URL 校验：仅检查 IP 字面量；域名需调用方 DNS 解析后通过 `validate_resolved`
pub fn validate_url(url: &Url) -> Result<(), SsrfError> {
    let host = url.host().ok_or(SsrfError::Dns("no host".into()))?;
    match host {
        url::Host::Ipv4(ip) => check_ip(IpAddr::V4(ip))?,
        url::Host::Ipv6(ip) => check_ip(IpAddr::V6(ip))?,
        url::Host::Domain(d) => {
            // 域名不做 DNS 解析，由调用方在拨号时解析并校验
            let _ = d;
        }
    }
    Ok(())
}

/// 单个 IP 校验：保留段一律拒绝。
///
/// IPv4-mapped IPv6（`::ffff:a.b.c.d`）先 unmap 再按 IPv4 规则查——否则
/// `::ffff:127.0.0.1` 的 `is_loopback()` 为 false，能绕过整套检查。
///
/// # Errors
/// loopback / unspecified / multicast / private（含 CGNAT）/ link-local 各返回对应变体。
pub fn check_ip(ip: IpAddr) -> Result<(), SsrfError> {
    // 折叠到规范形式：IPv4-mapped 一律按 IPv4 规则判定。
    let ip = match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        v4 => v4,
    };
    if ip.is_loopback() {
        return Err(SsrfError::Loopback);
    }
    if ip.is_unspecified() {
        return Err(SsrfError::Unspecified);
    }
    if ip.is_multicast() {
        return Err(SsrfError::Multicast);
    }
    if is_private(&ip) {
        return Err(SsrfError::PrivateIp);
    }
    if is_link_local(&ip) {
        return Err(SsrfError::LinkLocal);
    }
    Ok(())
}

/// 拨号前再次校验（防 DNS rebinding）——同步版本
pub fn validate_resolved(addrs: &[IpAddr]) -> Result<IpAddr, SsrfError> {
    for ip in addrs {
        check_ip(*ip)?;
    }
    addrs
        .first()
        .copied()
        .ok_or(SsrfError::Dns("no addresses".into()))
}

/// 判断是否为私有地址（含 CGNAT 100.64.0.0/10）
fn is_private(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            // CGNAT 100.64.0.0/10
            let bytes = v4.octets();
            (bytes[0] == 100 && (bytes[1] & 0xc0) == 64) || v4.is_private()
        }
        IpAddr::V6(v6) => {
            let b = v6.octets();
            // fc00::/7
            (b[0] & 0xfe) == 0xfc
        }
    }
}

/// 判断是否为链路本地地址
fn is_link_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_link_local(),
        IpAddr::V6(v6) => v6.segments()[0] == 0xfe80,
    }
}
