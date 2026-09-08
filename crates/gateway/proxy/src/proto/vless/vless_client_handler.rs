//! VLESS 客户端握手（ported from shoes, MIT）。
//!
//! 协议：VLESS 头部由用户 ID（16 字节 UUID）+ 附加数据 + 目标地址帧组成。
//!
//! 不移植：server 端、UDP ASSOCIATE、h2mux、shadowsocks-over-vless、Vision 流、Reality（超范围）。
//!
//! PR3：适配 `ProxyConnector` 协议，实现链式客户端接入。
//!
//! 对外类型名（实现即 `VlessProxyConnector`）：
//! - `VlessProxyConnector = VlessTcpClientHandler`
//!
//! 原计划中废弃的 UDP-over-TCP 见 `VlessTcpClientHandler::setup_client_udp_bidirectional`，
//! 可选启用用于 UDP 穿透。
//!
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use super::vless_util::{COMMAND_TCP, encode_flow_addon};
use crate::proto::address::{NetLocation, ResolvedLocation};
use crate::proto::async_stream::AsyncStream;
use crate::proto::proxy_connector::ProxyConnector;

/// VLESS 版本号
const VLESS_VERSION: u8 = 0;

/// VLESS 客户端连接器（实现 `ProxyConnector`）。
///
/// # 参数
/// - `user_id`：16 字节 UUID（hex 字符串解析后的二进制形式）
/// - `location`：代理服务器地址（用于 `proxy_location` 返回）
/// - `flow`：可选流控标识（如 "xtls-rprx-vision"），为空则不写入附加数据
#[derive(Debug)]
pub struct VlessTcpClientHandler {
    user_id: Box<[u8]>,
    location: NetLocation,
    flow: Option<String>,
}

/// 对外类型名（实现即 `VlessTcpClientHandler`）。
pub type VlessProxyConnector = VlessTcpClientHandler;

impl VlessTcpClientHandler {
    /// 创建客户端连接器。
    ///
    /// # 参数
    /// - `location`：VLESS 服务器地址
    /// - `user_id`：UUID 字符串（带或不带连字符均可），解析为 16 字节
    /// - `flow`：可选流控标识，如 "xtls-rprx-vision"；None 表示不使用流控
    pub fn new(location: NetLocation, user_id: &str, flow: Option<String>) -> Self {
        Self {
            user_id: parse_uuid(user_id)
                .expect("invalid UUID")
                .into_boxed_slice(),
            location,
            flow,
        }
    }
}

#[async_trait]
impl ProxyConnector for VlessTcpClientHandler {
    fn proxy_location(&self) -> &NetLocation {
        &self.location
    }

    /// 写入 VLESS 请求头：version + user_id + addon_length + addon_data + command + port + address。
    ///
    /// 调用方保证 `stream` 已完成 TLS 握手（VLESS 协议要求 TLS 内传输）。
    async fn setup_tcp_stream(
        &self,
        mut stream: Box<dyn AsyncStream>,
        target: &ResolvedLocation,
    ) -> std::io::Result<Box<dyn AsyncStream>> {
        // 准备附加数据（flow）
        let addon_data = if let Some(ref flow) = self.flow {
            encode_flow_addon(flow)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
        } else {
            Vec::new()
        };

        // 写入 VLESS 头部
        write_vless_header(&mut stream, &self.user_id, &addon_data, target.location()).await?;
        stream.flush().await?;
        Ok(stream)
    }
}

/// 将 VLESS 头部写入流。
///
/// 头部格式：
/// - version (1 byte)
/// - user_id (16 bytes)
/// - addon_length (1 byte)
/// - addon_data (addon_length bytes)
/// - command (1 byte)
/// - port (2 bytes, big-endian)
/// - address_type (1 byte)
/// - address (variable)
async fn write_vless_header<S: AsyncWriteExt + Unpin>(
    stream: &mut S,
    user_id: &[u8],
    addon_data: &[u8],
    remote_location: &NetLocation,
) -> std::io::Result<()> {
    use crate::proto::address::Address;

    // 计算基础头部大小：version + user_id + addon_length + addon_data + command + port + address_type
    let base_header_size = 1 + 16 + 1 + addon_data.len() + 1 + 2 + 1;
    let mut header_bytes = Vec::with_capacity(base_header_size);

    // version
    header_bytes.push(VLESS_VERSION);
    // user_id (16 bytes)
    header_bytes.extend_from_slice(user_id);
    // addon_length
    header_bytes.push(addon_data.len() as u8);
    // addon_data
    if !addon_data.is_empty() {
        header_bytes.extend_from_slice(addon_data);
    }

    let addon_end = 18 + addon_data.len();

    // command (1 = TCP)
    header_bytes.push(COMMAND_TCP);

    // port (2 bytes, big-endian)
    let remote_port = remote_location.port();
    header_bytes.push((remote_port >> 8) as u8);
    header_bytes.push((remote_port & 0xff) as u8);

    // address_type + address
    let _address_type_offset = addon_end + 3;

    match remote_location.address() {
        Address::Ipv4(v4addr) => {
            header_bytes.push(1);
            header_bytes.extend_from_slice(&v4addr.octets());
        }
        Address::Ipv6(v6addr) => {
            header_bytes.push(3);
            header_bytes.extend_from_slice(&v6addr.octets());
        }
        Address::Hostname(hostname) => {
            if hostname.len() > 255 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Hostname is too long: {hostname}"),
                ));
            }
            header_bytes.push(2);
            header_bytes.push(hostname.len() as u8);
            header_bytes.extend_from_slice(hostname.as_bytes());
        }
    }

    stream.write_all(&header_bytes).await?;
    Ok(())
}

/// Parse a UUID v4 string (with or without dashes) into 16 bytes.
/// Validates that the UUID has version 4 and RFC 4122 variant.
fn parse_uuid(uuid_str: &str) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(16);
    let mut first_nibble: Option<u8> = None;

    for &c in uuid_str.as_bytes() {
        if c == b'-' {
            continue;
        }
        let nibble = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid hex character in UUID: {c}"),
                ));
            }
        };
        if first_nibble.is_none() {
            first_nibble = Some(nibble);
        } else {
            bytes.push((first_nibble.unwrap() << 4) | nibble);
            first_nibble = None;
        }
    }

    if first_nibble.is_some() || bytes.len() != 16 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid UUID length",
        ));
    }

    // Validate version 4: upper nibble of byte 6 must be 4
    if (bytes[6] >> 4) != 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "UUID must be version 4",
        ));
    }

    // Validate variant (RFC 4122): upper 2 bits of byte 8 must be 10
    if (bytes[8] >> 6) != 2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "UUID must have RFC 4122 variant",
        ));
    }

    Ok(bytes)
}
