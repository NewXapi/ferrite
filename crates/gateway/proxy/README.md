# `gateway-proxy`

出口代理：URL 解析、按 `channel_key` 选节点、租约 Client、SSRF。

## 文件

- `src/node.rs` — 解析 `http://` / `socks5://` / `ss://` / `trojan://` / `vless://` / `vmess://` / `hysteria2://` / `anytls://` / `snell://`（含 percent-encoded 认证与 query），HTTP/SOCKS5 转 `reqwest::Proxy`。
- `src/adapter.rs` — 节点 → meow `ProxyAdapter`：协议握手在 `dial_tcp` 内完成，含 VLESS Vision flow 与 REALITY 参数解码，以及 Hysteria2/AnyTLS/Snell (fallback for now, Hy2 dial_tcp is TCP-capable via QUIC stream).
- `tests/extra_schemes.rs` — parse + adapter_for tests for new schemes (no network).
 - `src/manager.rs` — is_supported_scheme always true, to_reqwest_proxy error branch updated.

## 出口怎么走

`ProxyManager::acquire(channel_key)` 选出节点，用 `to_reqwest_proxy()` 构造 `reqwest::Client`。
HTTP CONNECT / SOCKS5 握手由 reqwest（workspace `socks` feature）完成，不自写 dialer。
无节点或全部冷却 → `node_id = 0` 直连。
`ForwardStage::with_proxies` 在每次模型请求上租约、转发、按状态反馈。

节点由 `apps/gateway` 的 `[[proxy_nodes]]` 经 `build_proxy_snapshot` 注入。

## vless / vmess / ss / trojan 走哪

**meow 适配器（[`adapter`]），在拨号时完成协议握手。**
`adapter_for(&ProxyNode)` 把节点映射成 `Arc<dyn meow_common::ProxyAdapter>`；
`ProxyManager` 缓存它（`ProxyClient::Adapter`），`forward::adapter_egress`
把 `dial_tcp` 产出的流桥成 hyper connector——HTTPS 由 rustls 在流上叠加。

旧 `src/proto/`（shoes 手抄客户端链，PR1-3）已删除：那条 `ProxyConnector`
路径从未接进 forward（恒 502），由 meow 实现整体替代。

### VLESS 的 query 参数

`vless://<uuid>@host:port?flow=&sni=&pbk=&sid=` —— 与常见分享链接同名：

|参数|含义|缺省|
|---|---|---|
|`flow`|`xtls-rprx-vision` 开 Vision；其他值 warn 后按非 Vision|无|
|`sni`|TLS / REALITY 的 SNI|回落节点 host|
|`pbk`|REALITY 服务端 X25519 公钥（64 hex）；**出现即走 REALITY**|无（明文 TCP）|
|`sid`|REALITY short id（0–16 hex，前对齐补零到 8 字节）|全 0|
|`type`|`ws` 启用 WebSocket 传输层（等价于 `path` 存在）|无|
|`path`|WebSocket 路径（如 `/ws`）；**存在即视为 WS 节点**|无|
|`host`|WebSocket Host header（可选，默认用 SNI 或节点 host）|无|

`sni` 或 `pbk` 任一存在才挂 TLS 层；参数非法（pbk 长度不对、sid 超 8 字节）
→ warn + 回落直连，不 panic。Hysteria2 / AnyTLS / Snell 尚未接节点配置。

## SSRF

`validate_url` 只校验 IP 字面量。域名必须由调用方 DNS 解析后再调 `validate_resolved`。

## 验收

```bash
cargo check -p gateway-proxy
cargo test -p gateway-proxy --test vless_opts
# 真实节点（可选）：本机起 sing-box 后
FERRITE_PROXY_SOCKS5=127.0.0.1:7890 cargo test -p forward --test real_node
```
