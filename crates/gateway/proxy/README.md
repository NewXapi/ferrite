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

节点由 `apps/gateway` 的 `[[proxy_nodes]]` 经 `build_proxy_snapshot` 注入；
只接受 `http://` / `socks5://`。

## vless / vmess / ss / trojan / reality 走哪

**走 shoes sidecar，不在本 crate 握手。** `apps/gateway` 按 `[egress]` 拉起独立
shoes 进程（MIT，https://github.com/cfal/shoes），它开 mixed HTTP+SOCKS5 入站，
复杂协议写在它自己的 `config/shoes.yaml` 的 `client_chain` 里。ferrite 只把
`socks5://127.0.0.1:7890` 当普通节点用——出口协议对网关透明。

`src/proto/` 下有 PR1-3 从 shoes 移植的 SS/Trojan/VMess/VLESS/WS 客户端链，
产出 `ProxyClient::Connector`。**该路径尚未接进 forward**（`forward::stage`
遇到 Connector 直接 502 让 retry 换候选），所以生产出口一律走 sidecar。
把 vless 之类的 URL 写进 `[[proxy_nodes]]` 会被 `build_proxy_snapshot` 跳过并 warn。

## SSRF

`validate_url` 只校验 IP 字面量。域名必须由调用方 DNS 解析后再调 `validate_resolved`。

## 验收

```bash
cargo check -p gateway-proxy
```
