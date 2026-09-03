# `gateway-proxy`

## 文件

- `src/lib.rs` — 公开代理池和 ProxyStage。
- `src/node.rs` — 解析直连、HTTP 和 SOCKS5 代理节点。
- `src/pool.rs` — 按渠道保存并选择代理节点。
- `src/dialer.rs` — 实现直连、HTTP CONNECT 和 SOCKS5 拨号。
- `src/ssrf.rs` — 校验 URL、DNS 解析结果和拨号地址。
- `src/stage.rs` — 把代理选择接到 pipeline。

