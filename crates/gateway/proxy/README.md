# `gateway-proxy`

出口代理：URL 解析、按 `channel_key` 选节点、租约 Client、SSRF。

## 文件

- `src/lib.rs` — 导出 `ProxyNode` / `ProxyPool` / `ProxyManager` / `Lease` / `validate_url`。
- `src/node.rs` — 解析 `http://` / `socks5://`（含 percent-encoded 认证），转 `reqwest::Proxy`。
- `src/pool.rs` — 按 `channel_key`（`ChannelRecord.meta.key`）索引，priority 分层。
- `src/manager.rs` — `acquire` / `feedback` / per-node Client 缓存 / 冷却。
- `src/ssrf.rs` — IP 字面量与 DNS 解析结果双重校验。

## 出口怎么走

`ProxyManager::acquire(channel_key)` 选出节点，用 `to_reqwest_proxy()` 构造 `reqwest::Client`。
HTTP CONNECT / SOCKS5 握手由 reqwest（workspace `socks` feature）完成，不自写 dialer。
无节点或全部冷却 → `node_id = 0` 直连。
`ForwardStage::with_proxies` 在每次模型请求上租约、转发、按状态反馈。

vless / vmess / ss / trojan 本 crate 不握手；非 http/socks5 URL `parse_url` 失败。

## SSRF

`validate_url` 只校验 IP 字面量。域名必须由调用方 DNS 解析后再调 `validate_resolved`。

## 验收

```bash
cargo check -p gateway-proxy
```
