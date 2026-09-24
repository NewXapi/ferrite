# metering

计费旁路：预扣额度、token 估算、定价与结算。

## 职责

请求进入时按预估用量预扣额度，流式过程中用 `scanner` 从 SSE 增量里估算 token，
结束后按实际用量 `settle` 结算并写 `ledger`（旁路，不阻塞响应）。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/estimate.rs` | 请求前预估用量 |
| `src/pricing.rs` | 模型定价（输入/输出/缓存分价） |
| `src/ledger.rs` | 计费流水账 |
| `src/scanner.rs` | 流式响应 token 增量扫描 |
| `src/settle.rs` | 结算：预估 vs 实际差额 |
| `src/sink.rs` | 计量结果下沉 |
| `src/lib.rs` | crate 导出面 |

## 关键语义

- 计费单位 `500000 = ¥1`（quota / used_quota 同量纲）
- 缺「预估偏高先占位、实际用量回来差额退还」的完整预留语义（参考实现 omnillm 的
  无锁预算 CAS 有这个能力）

## 依赖

只依赖 `contract`。

## 验收

```bash
cargo check -p metering
cargo test -p metering                     # CI
```
