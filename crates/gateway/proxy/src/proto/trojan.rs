//! `trojan` —— Trojan 客户端握手（ported from shoes, MIT）。
//!
//! 协议：TLS 内 `hex(SHA224(password)) + CRLF + CMD_CONNECT + SOCKS地址帧 + CRLF`。
//! TLS 由 [`proto::tls`] 提供；本模块只负责明文帧。
//!
//! 不移植：server 端、UDP ASSOCIATE、h2mux、shadowsocks-over-trojan（超范围）。

use async_trait::async_trait;
use aws_lc_rs::digest::SHA224;
use tokio::io::AsyncWriteExt;

use super::proxy_connector::ProxyConnector;
use super::socks_addr::{CMD_CONNECT, write_location_to_vec};
use crate::proto::address::{NetLocation, ResolvedLocation};
use crate::proto::async_stream::AsyncStream;

const CRLF_BYTES: [u8; 2] = [0x0d, 0x0a];

/// Trojan 客户端连接器（TLS 流由调用方包好，见 [`proto::tls`]）。
#[derive(Debug)]
pub struct TrojanTcpClientHandler {
    password_hash: Box<[u8]>,
    location: NetLocation,
}

/// 对外类型名（实现即 [`TrojanTcpClientHandler`]）。
pub type TrojanProxyConnector = TrojanTcpClientHandler;

impl TrojanTcpClientHandler {
    /// 创建客户端连接器。
    ///
    /// # 参数
    /// - `location`：Trojan 服务器地址
    /// - `password`：明文密码（内部做 SHA224 + hex）
    pub fn new(location: NetLocation, password: &str) -> Self {
        Self {
            password_hash: create_password_hash(password),
            location,
        }
    }
}

#[async_trait]
impl ProxyConnector for TrojanTcpClientHandler {
    fn proxy_location(&self) -> &NetLocation {
        &self.location
    }

    /// 写入 Trojan 请求帧：hash + CRLF + CMD + 地址 + CRLF。
    /// 调用方保证 `stream` 已完成 TLS 握手（Trojan 协议要求 TLS 内传输）。
    async fn setup_tcp_stream(
        &self,
        mut stream: Box<dyn AsyncStream>,
        target: &ResolvedLocation,
    ) -> std::io::Result<Box<dyn AsyncStream>> {
        stream.write_all(&self.password_hash).await?;
        stream.write_all(&CRLF_BYTES).await?;
        stream.write_all(&[CMD_CONNECT]).await?;
        let location_bytes = write_location_to_vec(target.location());
        stream.write_all(&location_bytes).await?;
        stream.write_all(&CRLF_BYTES).await?;
        stream.flush().await?;
        Ok(stream)
    }
}

/// `hex(SHA224(password))`，固定 56 字节 ASCII（shoes 同款实现）。
fn create_password_hash(password: &str) -> Box<[u8]> {
    let digest = aws_lc_rs::digest::digest(&SHA224, password.as_bytes());
    let hash_bytes = digest.as_ref();
    let mut hex_bytes = Vec::with_capacity(hash_bytes.len() * 2);
    for b in hash_bytes {
        let hi = b >> 4;
        let lo = b & 0x0f;
        hex_bytes.push(if hi < 10 { b'0' + hi } else { b'a' + hi - 10 });
        hex_bytes.push(if lo < 10 { b'0' + lo } else { b'a' + lo - 10 });
    }
    debug_assert_eq!(
        hex_bytes.len(),
        56,
        "SHA224 hex must be 56 ASCII chars"
    );
    hex_bytes.into_boxed_slice()
}
