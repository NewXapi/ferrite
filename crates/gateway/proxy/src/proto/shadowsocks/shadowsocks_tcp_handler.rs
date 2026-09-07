// Copyright (c) 2026 ferrite
// Licensed under the MIT License
// ported from shoes (MIT)

use parking_lot::Mutex;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use super::salt_checker::SaltChecker;
use super::shadowsocks_stream_type::ShadowsocksStreamType;
use super::timed_salt_checker::TimedSaltChecker;
use super::{Blake3Key, DefaultKey, ShadowsocksCipher, ShadowsocksKey, ShadowsocksStream};
use crate::proto::address::{NetLocation, ResolvedLocation};
use crate::proto::async_stream::AsyncStream;
use crate::proto::proxy_connector::ProxyConnector;
use crate::proto::socks_addr::write_location_to_vec;

/// Shadowsocks TCP client handler implementing ProxyConnector trait.
///
/// This enables Shadowsocks connections through the gateway proxy chain.
/// Only TCP CONNECT mode is supported; UDP and h2mux are omitted per specification.
#[derive(Debug)]
pub struct ShadowsocksTcpHandler {
    location: NetLocation,
    cipher: ShadowsocksCipher,
    key: Arc<Box<dyn ShadowsocksKey>>,
    aead2022: bool,
    salt_checker: Option<Arc<Mutex<dyn SaltChecker>>>,
}

impl ShadowsocksTcpHandler {
    /// Create a new Shadowsocks TCP handler for client use.
    ///
    /// # Arguments
    /// * `location` - Proxy server address
    /// * `cipher` - Encryption cipher to use
    /// * `password` - Password for key derivation (or raw key for AEAD2022)
    /// * `aead2022` - Whether to use AEAD2022 mode
    pub fn new_client(
        location: NetLocation,
        cipher: ShadowsocksCipher,
        password: &str,
        aead2022: bool,
    ) -> Self {
        let key: Arc<Box<dyn ShadowsocksKey>> = if aead2022 {
            Arc::new(Box::new(Blake3Key::new(
                password.as_bytes().to_vec().into_boxed_slice(),
                cipher.key_len(),
            )))
        } else {
            Arc::new(Box::new(DefaultKey::new(password, cipher.key_len())))
        };

        let salt_checker: Option<Arc<Mutex<dyn SaltChecker>>> = if aead2022 {
            Some(Arc::new(Mutex::new(TimedSaltChecker::new(60))))
        } else {
            None
        };

        Self {
            location,
            cipher,
            key,
            aead2022,
            salt_checker,
        }
    }
}

#[async_trait]
impl ProxyConnector for ShadowsocksTcpHandler {
    fn proxy_location(&self) -> &NetLocation {
        &self.location
    }

    async fn setup_tcp_stream(
        &self,
        stream: Box<dyn AsyncStream>,
        target: &ResolvedLocation,
    ) -> std::io::Result<Box<dyn AsyncStream>> {
        // 包上加密流：首写时 SS 流内部会把 SOCKS 地址帧加密进第一包。
        // AEAD2022 用 TimedSaltChecker 做盐重复检测。
        let stream_type = if self.aead2022 {
            ShadowsocksStreamType::AEAD2022Client
        } else {
            ShadowsocksStreamType::Aead
        };
        let mut ss = ShadowsocksStream::new(
            stream,
            stream_type,
            self.cipher.algorithm(),
            self.cipher.salt_len(),
            self.key.clone(),
            self.salt_checker.clone(),
        );

        // 目标地址由流加密写出（process_write_header）
        let addr_bytes = write_location_to_vec(target.location());
        ss.write_all(&addr_bytes).await?;
        ss.flush().await?;

        Ok(Box::new(ss))
    }
}
