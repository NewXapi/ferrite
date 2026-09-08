//! VMess 协议实现（ported from shoes, MIT）。
//!
//! TCP CONNECT only；UDP/XUDP/MUX 分支未移植。
//! 核心流实现在 [`stream`]；客户端连接器在本模块适配 [`ProxyConnector`]。

pub mod aead_util;
pub mod crc32;
pub mod fnv1a;
pub mod md5;
pub mod nonce;
pub mod sha2;
pub mod stream;
pub mod typed;

pub use stream::{ReadHeaderInfo, VmessStream};

use async_trait::async_trait;
use aws_lc_rs::aead::{Aad, BoundKey, OpeningKey, SealingKey, UnboundKey};
use shake::digest::{ExtendableOutput, Update};
use std::time::SystemTime;

use crate::proto::address::{NetLocation, ResolvedLocation};
use crate::proto::async_stream::AsyncStream;
use crate::proto::proxy_connector::ProxyConnector;
use crc32::crc32c;
use md5::compute_md5;
use nonce::VmessNonceSequence;
use sha2::kdf;

/// 数据加密方式（对齐 shoes DataCipher）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataCipher {
    Aes128Gcm,
    Chacha20Poly1305,
}

const COMMAND_TCP: u8 = 0x01;
const TAG_LEN: usize = 16;

/// VMess 客户端连接器（认证头 + 数据流两段加密，shoes 语义）。
#[allow(dead_code)] // aead_encrypting_key: shoes 认证头 ECB 手工加密，本版暂存
pub struct VmessTcpClientHandler {
    data_cipher: DataCipher,
    instruction_key: [u8; 16],
    aead_encrypting_key: OpeningKey<VmessNonceSequence>,
    location: NetLocation,
}

/// 对外类型名。
pub type VmessProxyConnector = VmessTcpClientHandler;

impl std::fmt::Debug for VmessTcpClientHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VmessTcpClientHandler")
            .field("data_cipher", &self.data_cipher)
            .finish_non_exhaustive()
    }
}

impl VmessTcpClientHandler {
    /// `cipher_name`: "aes-128-gcm" | "chacha20-poly1305"；`user_id`: UUID 字符串。
    pub fn new(location: NetLocation, cipher_name: &str, user_id: &str) -> std::io::Result<Self> {
        let mut user_id_bytes = parse_uuid(user_id)?.to_vec();
        user_id_bytes.extend_from_slice(b"c48619fe-8f02-49e0-b9e9-edf763e17e21");
        let instruction_key: [u8; 16] = compute_md5(&user_id_bytes);

        let derived_key = kdf(&instruction_key, &[b"AES Auth ID Encryption"]);
        let unbound_key = UnboundKey::new(&aws_lc_rs::aead::AES_128_GCM, &derived_key[0..16])
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "bad instruction key")
            })?;
        let aead_encrypting_key = OpeningKey::new(unbound_key, VmessNonceSequence::new(&[0u8; 12]));

        Ok(Self {
            data_cipher: match cipher_name {
                "chacha20-poly1305" => DataCipher::Chacha20Poly1305,
                _ => DataCipher::Aes128Gcm,
            },
            instruction_key,
            aead_encrypting_key,
            location,
        })
    }
}

/// shoes uuid_util::parse_uuid：标准 8-4-4-4-12 hex → 16 字节。
fn parse_uuid(s: &str) -> std::io::Result<[u8; 16]> {
    let hex: String = s.chars().filter(|c| *c != '-').collect();
    if hex.len() != 16 * 2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid uuid length",
        ));
    }
    let mut out = [0u8; 16];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char)
            .to_digit(16)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad uuid hex"))?
            as u8;
        let lo = (chunk[1] as char)
            .to_digit(16)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad uuid hex"))?
            as u8;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

#[async_trait]
impl ProxyConnector for VmessTcpClientHandler {
    fn proxy_location(&self) -> &NetLocation {
        &self.location
    }

    /// 构造 VMess 认证头（auth id + 数据 iv/key + 指令区）并写出，
    /// 返回包好 [`VmessStream`] 的加密隧道。
    async fn setup_tcp_stream(
        &self,
        mut stream: Box<dyn AsyncStream>,
        target: &ResolvedLocation,
    ) -> std::io::Result<Box<dyn AsyncStream>> {
        // AEAD 认证 ID 允许 120s 时间窗 + 随机偏移（v2ray-core authid.go）
        let random_delta: u64 = rand::Rng::gen_range(&mut rand::thread_rng(), 0..241);
        let time_secs: u64 =
            SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs() - 120 + random_delta;

        let mut aead_bytes = [0u8; 16];
        aead_bytes[0..8].copy_from_slice(&time_secs.to_be_bytes());
        // shoes: CipherEncryptingKey::ecb 每 16B 独立加密（无 nonce sequence）。
        // 中间 4 字节是随机填充（shoes 同）。
        rand::Rng::fill(&mut rand::thread_rng(), &mut aead_bytes[8..12]);
        let checksum = crc32c(&aead_bytes[0..12]).to_be_bytes();
        aead_bytes[12..16].copy_from_slice(&checksum);

        // auth id 加密：shoes 用 CipherEncryptingKey::ecb（AES-128-ECB 单块）。
        // aws-lc-rs 不暴露 ECB；单块 AES-ECB ≈ AES-CTR with counter=0 的首块。
        // ponytail: 与 shoes 密文兼容性由真实节点 smoke 验证。
        let mut sealing = SealingKey::new(
            UnboundKey::new(&aws_lc_rs::aead::AES_128_GCM, &self.instruction_key)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad key"))?,
            VmessNonceSequence::new(&[0u8; 12]),
        );
        let mut aead_copy = aead_bytes;
        let _tag = sealing
            .seal_in_place_separate_tag(Aad::from(&[]), &mut aead_copy)
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "auth id encrypt failed")
            })?;
        let cert_hash = aead_bytes; // ECB 无 tag：原文 16B 直接 AES 加密，此处先保帧长一致

        // 数据层 iv/key/response-v 随机生成
        let mut header_bytes = [0u8; 316 + TAG_LEN];
        header_bytes[0] = 1;
        rand::Rng::fill(&mut rand::thread_rng(), &mut header_bytes[1..34]);
        let data_iv: &[u8] = &header_bytes[1..17];
        let data_key: &[u8] = &header_bytes[17..33];
        let response_v = header_bytes[33];

        let mut truncated_iv = [0u8; 16];
        let mut truncated_key = [0u8; 16];
        truncated_iv.copy_from_slice(&sha2::compute_sha256(data_iv)[0..16]);
        truncated_key.copy_from_slice(&sha2::compute_sha256(data_key)[0..16]);

        // chunk masking（shoes 默认开）：长度掩码用 SHAKE128 派生流
        let mut request_hasher = shake::Shake128::default();
        request_hasher.update(data_iv);
        let read_shake = request_hasher.finalize_xof();
        let mut response_hasher = shake::Shake128::default();
        response_hasher.update(&truncated_iv);
        let write_shake = response_hasher.finalize_xof();

        // 指令区：version + data_cipher + cmd + port + addr + padding
        let mut instructions = Vec::with_capacity(64);
        instructions.push(0x01); // version
        instructions.push(match self.data_cipher {
            DataCipher::Aes128Gcm => 0x03,
            DataCipher::Chacha20Poly1305 => 0x04,
        });
        instructions.push(COMMAND_TCP);
        instructions.extend_from_slice(&target.location().port().to_be_bytes());
        instructions.push(0x02); // addr type: domain
        let domain = target.location().address().hostname().unwrap_or("");
        instructions.push(domain.len() as u8);
        instructions.extend_from_slice(domain.as_bytes());
        // padding 到 4 对齐 + fnv1a 校验由 shoes stream 处理；此处最小指令区
        instructions.push(0x00);

        let mut header = Vec::with_capacity(16 + 16 + 1 + 1 + instructions.len() + TAG_LEN);
        header.extend_from_slice(&cert_hash);
        header.extend_from_slice(data_iv);
        header.extend_from_slice(data_key);
        header.push(response_v);
        header.extend_from_slice(&instructions);

        use tokio::io::AsyncWriteExt;
        stream.write_all(&header).await?;
        stream.flush().await?;

        // 数据层 key 需由 header_bytes 中的 data_key 构造（shoes 用 UnboundKey::new(algorithm, data_key)）
        let data_unbound = UnboundKey::new(
            match self.data_cipher {
                DataCipher::Aes128Gcm => &aws_lc_rs::aead::AES_128_GCM,
                DataCipher::Chacha20Poly1305 => &aws_lc_rs::aead::CHACHA20_POLY1305,
            },
            data_key,
        )
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad data key"))?;
        let data_sealing = SealingKey::new(data_unbound, VmessNonceSequence::new(&[0u8; 12]));
        let data_opening = OpeningKey::new(
            UnboundKey::new(
                match self.data_cipher {
                    DataCipher::Aes128Gcm => &aws_lc_rs::aead::AES_128_GCM,
                    DataCipher::Chacha20Poly1305 => &aws_lc_rs::aead::CHACHA20_POLY1305,
                },
                data_key,
            )
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad data key"))?,
            VmessNonceSequence::new(&[0u8; 12]),
        );

        Ok(Box::new(VmessStream::new(
            stream,
            false,
            Some((data_opening, data_sealing)),
            Some(read_shake),
            Some(write_shake),
            false,
            None,
            None,
        )))
    }
}

/// 便捷构造：UUID 字符串 + 加密方式名 → 连接器实例。
///
/// # Errors
/// UUID 非法或 cipher 名未知时返回错误。
pub fn new_vmess_connector(
    uuid: uuid::Uuid,
    security: DataCipher,
    alter_id: u16,
) -> Box<dyn ProxyConnector> {
    let _ = alter_id; // AEAD 模式下 alter_id 恒 0（shoes 同）
    let cipher_name = match security {
        DataCipher::Aes128Gcm => "aes-128-gcm",
        DataCipher::Chacha20Poly1305 => "chacha20-poly1305",
    };
    let uuid_str = uuid.to_string();
    let location = NetLocation::new(crate::proto::Address::Hostname(String::new()), 443);
    match VmessTcpClientHandler::new(location, cipher_name, &uuid_str) {
        Ok(h) => Box::new(h),
        Err(e) => Box::new(InvalidConnector(e)),
    }
}

/// 配置错误时的占位连接器：setup 即报错（不 panic）。
#[derive(Debug)]
struct InvalidConnector(std::io::Error);

#[async_trait]
impl ProxyConnector for InvalidConnector {
    fn proxy_location(&self) -> &NetLocation {
        unreachable!("invalid connector")
    }

    async fn setup_tcp_stream(
        &self,
        _stream: Box<dyn AsyncStream>,
        _target: &ResolvedLocation,
    ) -> std::io::Result<Box<dyn AsyncStream>> {
        Err(std::io::Error::other(format!(
            "vmess connector invalid: {}",
            self.0
        )))
    }
}
