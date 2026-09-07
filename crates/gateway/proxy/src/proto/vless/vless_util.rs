//! VLESS 工具函数（ported from shoes, MIT）。
//!
//! 提供 VLESS 协议常量、流控附加数据编码等。
//! 仅保留客户端所需部分（COMMAND_TCP、encode_flow_addon），其余 server 端解析函数已砍。
//!
//! 忠实抄自 shoes/src/vless/vless_util.rs (214 行)，仅保留客户端路径。
//!
use std::io;

pub const COMMAND_TCP: u8 = 1;
pub const COMMAND_UDP: u8 = 2; // 保留常量但不实现 UDP

/// 将流控字符串编码为 protobuf addon data。
///
/// Format: field_tag(0x0a) + length + data
/// Field 1 = flow (string), wire type 2 (length-delimited)
pub fn encode_flow_addon(flow: &str) -> io::Result<Vec<u8>> {
    let flow_bytes = flow.as_bytes();
    let flow_len = flow_bytes.len();

    if flow_len > 127 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Flow string too long for simple varint encoding",
        ));
    }

    let mut result = Vec::new();

    // Field 1, wire type 2 (0x0a = (1 << 3) | 2)
    result.push(0x0a);

    // Length as varint (simple case: < 128)
    result.push(flow_len as u8);

    // Flow string data
    result.extend_from_slice(flow_bytes);

    Ok(result)
}
