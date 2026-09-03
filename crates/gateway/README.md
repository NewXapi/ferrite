# `crates/gateway`

## 功能 crate

- `pipeline/` — 请求上下文、Stage 接口、链执行器和 HTTP 路由。
- `gate/` — API Key、状态、配额、频率、模型和并发准入。
- `dispatch/` — 候选渠道、健康状态、权重选择和失败重试。
- `forward/` — 上游 URL、头、请求体、响应体和 SSE 流。
- `proxy/` — 直连、HTTP CONNECT、SOCKS5 和 SSRF 校验。
- `protocol/` — OpenAI、Claude、Gemini 请求、响应、错误和 SSE 编解码。
- `protocol-bridge/` — 将 pipeline 上下文转换为 protocol codec 输入输出。
- `metering/` — 预扣额度、token 获取与估算、价格和结算。
- `security/` — 词库、输入替换、跨 chunk 扫描和审核结果。

## 单机 MVP 先开发

- `pipeline/` — 定义可运行请求链。
- `gate/` — 校验 API Key 和启用状态。
- `dispatch/` — 按模型选择一个可用渠道。
- `forward/` — 转发 OpenAI 请求和 SSE。
- `protocol/` — 先支持 OpenAI 协议。
- `catalog/` — 由 admin-api 提供渠道和 Token 配置。

