# `crates/gateway`

## 目录

```text
gateway/
├── pipeline/
├── gate/
├── dispatch/
├── forward/
├── proxy/
├── protocol/
├── protocol-bridge/
├── metering/
└── security/
```

## 要实现

- `pipeline` 提供请求上下文、Stage trait、Pipeline 和 Axum 路由。
- `gate` 执行鉴权、状态、配额、频率、模型和并发检查。
- `dispatch` 匹配能力、评估健康度、按权重选路和重试。
- `forward` 组装上游请求并处理响应流。
- `proxy` 提供出口代理池、拨号和 SSRF 检查。
- `protocol` 提供 OpenAI、Claude、Gemini 编解码和 SSE 解析。
- `protocol-bridge` 连接 gateway 请求上下文与 protocol 编解码。
- `metering` 预扣配额、提取或估算 token、结算成本。
- `security` 扫描和处理输入输出内容。
