//! 单次尝试转发管道 — forward 的核心编排。
//!
//! 管道（一去一回）：
//! ```text
//! ForwardTask
//!   → adapter::prepare (URL 拼接 / 鉴权头 / 渠道参数覆盖)
//!   → 请求转换：入站格式 → IR → 上游格式
//!   → egress::execute (带超时发送)
//!   → [response]
//!       ├─ 非流式：上游格式 → IR → 入站格式，整体回传
//!       └─ 流式：  逐 SSE 帧解码成 IR 事件 → 入站格式的 StreamEncoder 再编码
//! ```
//!
//! ## 为什么转换在这里闭环（WP6 定稿）
//!
//! 请求方向与响应方向必须用同一份 codec 协作，而入站格式由请求路径决定、
//! 上游格式由渠道 `provider_type` 决定——两者都只有在本函数里才同时可见。
//! 旧实现把响应转换放在 `protocol-bridge` 的 stage 里，且硬编码「响应源协议 =
//! OpenAI」，流式路径更是完全不转换（`ctx.upstream` 只在非流式分支写入），
//! 导致流式的 Claude/Gemini 客户端拿到的永远是 OpenAI 事件形状（G2）。
//!
//! 失败出口统一为 contract::error::NormalizedError → dispatch::FailureClass。

use crate::ForwardTask;
use crate::adapter::{self, PreparedRequest};
use crate::egress::{Egress, ForwardedResponse};
use bytes::Bytes;
use contract::error::NormalizedError;
use gateway_protocol_bridge::adaptor::Protocol;
use gateway_protocol_bridge::format_codec::FormatRegistry;
use gateway_protocol_bridge::sse::SseScanner;
use std::sync::Arc;

/// 合并后的请求头 (adapter 鉴权 + 渠道覆盖 + 客户端已过滤头)。
pub fn merge_headers(
    prepared: &PreparedRequest,
    client_headers: &[(String, String)],
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> =
        Vec::with_capacity(2 + client_headers.len() + prepared.extra_headers.len());
    out.push(prepared.auth_header.clone());
    out.extend(prepared.extra_headers.iter().cloned());
    out.extend(client_headers.iter().cloned());
    out
}

/// 读完整响应 body — 仅非流式路径使用。
async fn read_all_body(resp: ForwardedResponse) -> Result<Bytes, std::io::Error> {
    use futures_util::TryStreamExt;
    let mut body = resp.into_body_stream();
    let mut buf = Vec::new();
    while let Some(chunk) = body.try_next().await? {
        buf.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(buf))
}

/// 单次转发的核心编排 — prepare + 双向协议转换 + execute。
///
/// 流式 / 非流式统一返回 `Forwarded`：非流式响应也按字节流形状给出（单 chunk）。
///
/// `body_stream` 决定响应形态：
/// - 流式 (`stream=true`) → 上游 SSE 逐帧转换后流过
/// - 非流式 (`stream=false`) → `read_all_body` 一次性收集并整体转换
///
/// TODO(#334): 非流式响应也需走 metering 估算 prompt token。
pub async fn forward_once(
    task: &ForwardTask,
    egress: &dyn Egress,
    formats: &FormatRegistry,
    timeouts: &crate::egress::Timeouts,
) -> Result<crate::Forwarded, NormalizedError> {
    let prepared = adapter::prepare(
        &task.candidate,
        &task.path,
        &task.provider_type,
        task.extra_headers.clone(),
    );
    let merged = merge_headers(&prepared, &task.headers);

    // 入站格式（客户端说的协议）→ 上游格式（渠道 provider_type 决定的厂商协议）。
    let inbound = Protocol::from_kind(task.inbound_format);
    let upstream_format = Protocol::from_provider_type(&task.provider_type);

    // 公开别名 → 上游真名：必须在协议转换之前改，否则各家 codec 已把 model
    // 搬进自己的字段位置，再改就得按协议分别处理。
    let body = rewrite_upstream_model(&task.body, &task.candidate.upstream_model);
    let body = formats
        .translate_request(inbound, upstream_format, body)
        .map_err(|e| protocol_bridge_error(e, 400, false))?;
    let body = capture_prompt_body(&body);

    let resp = egress
        .execute(&prepared.url, &merged, body, timeouts)
        .await?;

    let status = resp.status();
    let content_type = resp.content_type().to_string();

    // 响应方向：上游格式 → 入站格式。流式逐帧转换，非流式整体转换。
    if task.stream {
        // 每条流一个 encoder（`&mut self` 持有 block 边界与 tool 碎片状态），
        // 必须与本次请求同生共死——复用会串状态。
        let encoder = formats.resolve(inbound).map(|c| c.stream_encoder());
        let decoder = formats.resolve(upstream_format);
        let same_format = inbound == upstream_format;

        let mapped = futures_util::stream::unfold(
            (resp.into_body_stream(), SseScanner::default(), encoder, decoder),
            move |(mut upstream, mut scanner, mut encoder, decoder)| async move {
                use futures_util::StreamExt;
                loop {
                    let chunk = match upstream.next().await {
                        Some(Ok(c)) => c,
                        Some(Err(e)) => return Some((Err(e), (upstream, scanner, encoder, decoder))),
                        None => {
                            // 上游流结束：让 encoder 补收尾帧（block 关闭 / [DONE]）。
                            if let Some(enc) = encoder.as_mut() {
                                match enc.finish() {
                                    Ok(frames) if !frames.is_empty() => {
                                        let out = frames.concat();
                                        // finish 已吐完，置空避免重复收尾。
                                        return Some((
                                            Ok::<Bytes, std::io::Error>(out),
                                            (upstream, scanner, None, decoder),
                                        ));
                                    }
                                    Ok(_) => {}
                                    Err(e) => {
                                        return Some((
                                            Err(std::io::Error::other(e.to_string())),
                                            (upstream, scanner, None, decoder),
                                        ));
                                    }
                                }
                            }
                            return None;
                        }
                    };

                    // 同格式 = 零转换透传（客户端与渠道说同一种协议时不该被重写字节）。
                    if same_format {
                        return Some((Ok(chunk), (upstream, scanner, encoder, decoder)));
                    }

                    // 跨格式：扫描器重组成完整行，逐帧 `data:` 负载解码成 IR 事件，
                    // 再交给 encoder 编成入站格式的帧。
                    let (_passthrough, _events) = scanner.push(&chunk);
                    let frames = scanner.take_data_frames();
                    if frames.is_empty() {
                        // 半帧（跨 chunk 的行还没凑齐）：本块无可产出，继续读。
                        continue;
                    }

                    let mut out = Vec::new();
                    for frame in frames {
                        let events = match decoder.as_ref() {
                            Some(d) => match d.decode_event(&frame) {
                                Ok(evs) => evs,
                                // 单帧解码失败不该断整条流（SSE 里存在非 JSON 控制帧），
                                // 跳过该帧继续。
                                Err(_) => continue,
                            },
                            None => continue,
                        };
                        for ev in events {
                            if let Some(enc) = encoder.as_mut() {
                                match enc.encode_event(&ev) {
                                    Ok(bytes) => out.extend(bytes),
                                    Err(e) => {
                                        return Some((
                                            Err(std::io::Error::other(e.to_string())),
                                            (upstream, scanner, encoder, decoder),
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    if out.is_empty() {
                        // 该 chunk 只含无内容事件（如 message_start），继续读下一块。
                        continue;
                    }
                    return Some((Ok(out.concat()), (upstream, scanner, encoder, decoder)));
                }
            },
        );

        return Ok(crate::Forwarded {
            status,
            body: Box::pin(mapped),
            content_type,
        });
    }

    // 非流式：收全上游 body 后整体两跳转换。
    let bytes = match read_all_body(resp).await {
        Ok(b) => b,
        Err(e) => {
            return Err(NormalizedError {
                code: contract::error::code::UPSTREAM_ERROR,
                status: 502,
                retryable: true,
                channel_scoped: false,
                message: format!("read upstream body: {e}"),
            });
        }
    };
    let bytes = formats
        .translate_response(upstream_format, inbound, bytes)
        .map_err(|e| protocol_bridge_error(e, 502, true))?;

    Ok(crate::Forwarded {
        status,
        body: Box::pin(futures_util::stream::iter(std::iter::once(Ok(bytes)))),
        content_type,
    })
}

/// protocol-bridge 错误 → contract::error::NormalizedError 统一出口。
fn protocol_bridge_error(
    e: gateway_protocol_bridge::adaptor::AdaptorError,
    status: u16,
    retryable: bool,
) -> NormalizedError {
    NormalizedError {
        code: contract::error::code::UPSTREAM_ERROR,
        status,
        retryable,
        channel_scoped: false,
        message: e.to_string(),
    }
}
