// Copyright (c) 2026 ferrite
// Licensed under the MIT License
// ported from shoes (MIT)
//
//! Shadowsocks 客户端协议栈（TCP CONNECT only）。
//! UDP / h2mux / UoT 分支未移植（ferrite 出口场景用不到）。

pub mod aead_util;
pub mod blake3_key;
pub mod default_key;
pub mod salt_checker;
pub mod shadowsocks_cipher;
pub mod shadowsocks_key;
pub mod shadowsocks_stream;
pub mod shadowsocks_stream_type;
pub mod shadowsocks_tcp_handler;
pub mod timed_salt_checker;

pub use blake3_key::Blake3Key;
pub use default_key::DefaultKey;
pub use shadowsocks_cipher::ShadowsocksCipher;
pub use shadowsocks_key::ShadowsocksKey;
pub use shadowsocks_stream::ShadowsocksStream;
pub use shadowsocks_stream_type::ShadowsocksStreamType;
pub use shadowsocks_tcp_handler::ShadowsocksTcpHandler;

/// SS 客户端连接器（对外类型名，实现即 [`ShadowsocksTcpHandler`]）。
pub type ShadowsocksProxyConnector = ShadowsocksTcpHandler;
