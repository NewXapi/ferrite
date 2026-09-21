# dispatch

候选渠道管理、健康状态机、权重选择与失败重试。

## 职责

从候选渠道集合里挑一个上游。挑选前先用 `health` 的冷却状态过滤，再按 `selector` 的
priority 分层 + 层内加权随机选一个；失败后由 `retry(_policy)` 决定是否换渠道重试。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/candidate.rs` | 候选渠道集合与 `mark_tried`（同一请求内不重复选同一渠道） |
| `src/health.rs` | 健康状态机：EWMA 打分、`failure_streak`、冷却曲线、`cooldown_threshold` 弹射 |
| `src/selector.rs` | priority 分层 + 层内加权随机 + fallthrough |
| `src/retry.rs` | 重试循环：逐 attempt `report()` 给 health，`mark_tried` 排除已试渠道 |
| `src/retry_policy.rs` | `RetryPolicy`：max_attempts / total_budget / per_req_timeout |
| `src/ratelimit.rs` | 渠道级限流 |
| `src/failure_scope.rs` | 失败归类（哪些错误算渠道的错） |
| `src/stage.rs` | 作为 Stage 挂进 pipeline |
| `src/lib.rs` | crate 导出面 |

## 关键语义

- **`cooldown_threshold`（默认 5）** 是连续失败次数：一个渠道累计失败 5 次才进冷却。
  因为 `mark_tried` 排斥同请求重复选择，每请求每渠道最多失败一次，所以 streak 是
  跨请求累积的——坏渠道要连吃 5 个用户请求才被弹射。
- **`max_attempts`（默认 3）** 是单请求预算：在*不同*渠道间换，最多试 3 次。
  两个旋钮正交，不构成「重试完才冷却」的关系。
- `HealthSetting::max_ejection_percent` 字段已定义但**未实现**（`health.rs` 注释自证
  V1 未做 bypass），池枯竭时不会自动召回冷却中的渠道。

## 依赖

`{pipeline, gate, forward, protocol-bridge}` + `contract`。

## 验收

```bash
cargo check -p dispatch
cargo test -p dispatch                     # CI（含 health_anneal / health_streak / selector_recall）
```
