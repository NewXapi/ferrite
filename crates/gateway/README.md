# gateway — 网关数据面（热路径）

`apps/api` 与 `apps/gateway` 共同组装的热路径 crate 群。请求按下面的顺序流过：

```
HTTP → pipeline（上下文 + Stage 链）
     → gate（准入：auth / state / quota / ratelimit / model / graylist / concurrency）
     → dispatch（候选渠道 → 健康打分 → 权重选择 → 失败重试）
     → forward（上游请求 / 响应 / SSE 流转发 + 协议适配）
     → metering（计量旁路：预扣 / 估算 / 定价 / 结算）
```

`protocol-bridge` 做 pipeline 上下文 ↔ 各上游协议 codec 的输入输出适配；`proxy` 管出口
节点；`security` 做内容扫描。

## crate 清单

| crate | 职责 | 关键模块 |
|---|---|---|
| `pipeline` | 编排核心：请求上下文、Stage 接口、链执行器、HTTP 路由 | `ctx` / `stage` / `pipeline` / `router` / `error` |
| `gate` | 准入过滤链 | `auth` / `state` / `quota` / `ratelimit` / `model` / `graylist` / `concurrency` / `rewrite` / `snapshot` |
| `dispatch` | 候选渠道、健康状态、权重选择与失败重试 | `candidate` / `health` / `selector` / `retry(_policy)` / `ratelimit` / `stage` / `failure_scope` |
| `forward` | 上游 URL/头/体/SSE 流转发与出口适配 | `stage` / `stream` / `pipeline` / `egress` / `adapter(_egress)` / `stream_resilience` |
| `protocol-bridge` | openai / claude / gemini 协议编解码 + provider-neutral IR | `ir` / `openai` / `claude` / `gemini` / `responses` / `sse` / `format_codec` / `error_mapping` / `adaptor` / `stage` |
| `proxy` | 出口节点解析、Client 租借、SSRF 防护 | `manager` / `node` / `pool` / `probe` / `ssrf` / `sharelink` / `adapter` |
| `metering` | 预扣额度、token 估算、定价与结算 | `estimate` / `pricing` / `ledger` / `scanner` / `settle` / `sink` |
| `security` | 词库、输入替换、跨 chunk 扫描 | `scan` / `wordlist` |

## 依赖顺序

实测自各 `Cargo.toml` path 声明：

- `dispatch` → `{pipeline, gate, forward, protocol-bridge}`
- `forward` → `{pipeline, dispatch, protocol-bridge, proxy, metering}`
- `gate` / `pipeline` / `protocol-bridge` / `metering` 只依赖 `contract`（+ 域内必要件）
- `security` **零消费方**：`apps/api` 与 `apps/gateway` 均未依赖，改它不会被任何链路感知

## 硬约束

- 全部 crate 带 `tests/` 目录；**不得**在 `src/` 里写 `#[cfg(test)]`
- 路由与健康计算的默认值由 `apps/gateway` 的 `[dispatch]` / `[retry]` 配置段消费；
  `apps/api` 走硬编码默认（改默认值前确认两个入口都已接）

## 验收

```bash
cargo check -p gateway-pipeline -p gateway-gate -p dispatch -p forward \
  -p gateway-protocol-bridge -p gateway-proxy -p metering -p gateway-security
```
