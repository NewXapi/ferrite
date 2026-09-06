# `gateway-proxy`

出口代理节点：配置解析 + 按 channel 选节点 + SSRF 校验。

## 文件

- `src/lib.rs` — 导出 `ProxyNode` / `ProxyPool` / `validate_url`。
- `src/node.rs` — 解析代理 URL（http / socks5，含 percent-encoded 认证），转 `reqwest::Proxy`。
- `src/pool.rs` — 按 channel 索引代理节点，priority 分层 + 层内随机选一个。
- `src/ssrf.rs` — IP 字面量与 DNS 解析结果双重校验，拦截保留地址段。

## 为什么没有 dialer

`reqwest`（workspace 已启用 `socks` feature）原生支持 HTTP CONNECT 与 SOCKS5 握手。
出口代理只需在构造 `forward::ReqwestEgress` 的 `reqwest::Client` 时把
`ProxyNode::to_reqwest_proxy()` 的结果喂给 `ClientBuilder::proxy()`。
自己实现 dialer 会绕过 reqwest 的连接池，得不偿失。

## SSRF 使用约定

`validate_url` 只校验 IP 字面量。**域名必须由调用方 DNS 解析后再调 `validate_resolved`**，
否则 SSRF 防护是空的（也防不住 DNS rebinding）。

## 验收

```bash
cargo test -p gateway-proxy
```
