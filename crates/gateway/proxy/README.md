# `gateway-proxy`

## 目录

```text
src/
├── lib.rs
├── dialer.rs
├── node.rs
├── pool.rs
├── ssrf.rs
└── stage.rs
```

## 要实现

- 直连、HTTP CONNECT 和 SOCKS5 拨号。
- 出口代理节点和代理池。
- URL、解析和拨号阶段 SSRF 检查。
- Pipeline ProxyStage。
