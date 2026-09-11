# `gateway-proxy`

出口代理：URL 解析、按 `channel_key` 选节点、租约 Client、SSRF。

## 文件

- `src/node.rs` — 解析 `http://` / `socks5://` / `ss://` / `trojan://` / `vless://` / `vmess://` / `hysteria2://` / `anytls://` / `snell://`（含 percent-encoded 认证与 query），HTTP/SOCKS5 转 `reqwest::Proxy`。
- `src/adapter.rs` — 节点 → meow `ProxyAdapter`：协议握手在 `dial_tcp` 内完成，含 VLESS Vision flow 与 REALITY 参数解码，以及 Hysteria2/AnyTLS/Snell (fallback for now, Hy2 dial_tcp is TCP-capable via QUIC stream).
- `tests/extra_schemes.rs` — parse + adapter_for tests for new schemes (no network).
 - `src/manager.rs` — is_supported_scheme always true, to_reqwest_proxy error branch updated.

## 出口怎么走

`ProxyManager::acquire(channel_key)` 选出节点，用 `to_reqwest_proxy()` 构造 `reqwest::Client`。
层内并列最闲时软亲和：距上次使用 ≤10s 优先粘住上次选中的节点（省 ws/grpc/TLS 重复握手），
亲和节点冷却或负载拉开落出并列组后自动随机接替（rebalance）；行为测试见 `tests/affinity.rs`。
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

### 传输层 query 参数（`NodeOpts`，vless/vmess/hysteria2/anytls/snell 共用）

`vless://<uuid>@host:port?flow=&sni=&pbk=&sid=&fp=` —— 与常见分享链接同名：

|参数|含义|缺省|
|---|---|---|
|`flow`|`xtls-rprx-vision` 开 Vision；其他值 warn 后按非 Vision|无|
|`sni`|TLS / REALITY 的 SNI|回落节点 host|
|`pbk`|REALITY 服务端 X25519 公钥（64 hex）；**出现即走 REALITY**|无（明文 TCP）|
|`sid`|REALITY short id（0–16 hex，前对齐补零到 8 字节）|全 0|
|`type`|`ws` 启用 WebSocket 传输层（等价于 `path` 存在）|无|
|`path`|WebSocket 路径（如 `/ws`）；**存在即视为 WS 节点**|无|
|`host`|WebSocket Host header（可选，默认用 SNI 或节点 host）|无|
|`fp` / `fingerprint`|uTLS 指纹（`chrome`/`firefox`/`safari`/`ios`/`android`/`edge`/`360`/`qq`/`random`/`randomized`/`deprecated`），需开启 `utls` feature|无（默认 rustls，不仿冒）|

`sni` 或 `pbk` 任一存在才挂 TLS 层；参数非法（pbk 长度不对、sid 超 8 字节）
→ warn + 回落直连，不 panic。Hysteria2 / AnyTLS / Snell 尚未接节点配置。

### uTLS 指纹 feature

默认**不开启**。开启需要满足**构建前提**：

- 系统装有 `cmake`（BoringSSL 构建脚本硬依赖；缺失时 `boring-sys` 直接失败：`is cmake not installed?`）
- 编译耗时以十分钟计（BoringSSL 是 C/C++ 工程）

```bash
# Arch/CachyOS: sudo pacman -S cmake
cargo build -p gateway-proxy --features utls
```
或在依赖链中传递：`gateway-proxy = { ..., features = ["utls"] }`

- 开启后：`fp=chrome` → BoringSSL 真实 uTLS 仿冒
- 未开启：`fp=chrome` 落入 `TlsConfig.fingerprint`，**仅触发一次性 warn**（meow 内部 stub 实现），**依然使用 rustls**，不阻断节点构造
- Trojan 协议暂不支持 fingerprint（meow `TrojanAdapter` 无 fingerprint 参数），仅 VLESS TLS/REALITY 路径生效
- CI 覆盖：`.github/workflows/ci.yml` 的 `proxy-feature-matrix` job 同时编 `--features utls`（apt 装 cmake）与 `--no-default-features`（协议 feature 全关，验证 feature 门没漏）

## 依赖

`meow-config` 以 `default-features = false` + 按需协议 feature 引入，挡掉的是
`mux` / `vless-encryption` / `ech-tls-tunnel` 等默认 feature。注意
`maxminddb` / `meow-rules` 在 meow-config 0.21.x 是**无条件硬依赖**（不在任何
feature 之后），无法通过 feature 门移除；想减体积得等上游 gating，或换解析入口。

## SSRF

`validate_url` 只校验 IP 字面量。域名必须由调用方 DNS 解析后再调 `validate_resolved`。

## 验收

```bash
cargo check -p gateway-proxy
cargo test -p gateway-proxy --test vless_opts
# 真实节点（可选）：本机起 sing-box 后
FERRITE_PROXY_SOCKS5=127.0.0.1:7890 cargo test -p forward --test real_node
```
