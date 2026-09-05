//! `ssrf` —— SSRF 防护
//!
//! 双重 IP 校验：解析时（URL 解析出的 host）+ 拨号时（实际 DNS 解析结果）。
//! 防止 DNS rebinding 攻击：攻击者控制 DNS 第一次返回公网 IP、第二次返回内网 IP。

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

/// URL 校验：解析时检查 + 拨号时检查（异步）
pub fn validate_url(url: &Url) -> Result<(), SsrfError> {
    let host = url.host().ok_or(SsrfError::Dns("no host".into()))?;
    match host {
        url::Host::Ipv4(ip) => check_ip(IpAddr::V4(ip))?,
        url::Host::Ipv6(ip) => check_ip(IpAddr::V6(ip))?,
        url::Host::Domain(d) => {
            // TODO: 解析时不做 DNS（防 DoS），由调用方在拨号时解析
            let _ = d;
        }
    }
    Ok(())
}

/// 单个 IP 校验
pub fn check_ip(ip: IpAddr) -> Result<(), SsrfError> {
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

/// 拨号前再次校验（防 DNS rebinding）
pub async fn validate_resolved(addrs: &[IpAddr]) -> Result<IpAddr, SsrfError> {
    for ip in addrs {
        check_ip(*ip)?;
    }
    addrs
        .first()
        .copied()
        .ok_or(SsrfError::Dns("no addresses".into()))
}

fn is_private(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private(),
        IpAddr::V6(v6) => {
            // fc00::/7
            let b = v6.octets();
            (b[0] & 0xfe) == 0xfc
        }
    }
}

fn is_link_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_link_local(),
        IpAddr::V6(v6) => v6.segments()[0] == 0xfe80,
    }
}
