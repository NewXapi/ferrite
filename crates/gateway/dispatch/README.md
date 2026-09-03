# `gateway-dispatch`

调度状态机 — 从 catalog 快照中选出本次请求的执行目标（热路径第 2 步）。

## 文件

- `src/lib.rs` — `Dispatcher` 组装（快照 + 健康表 + 选择器）与 `Dispatch` trait；
  `Snapshot` 快照类型（路由单元 + 渠道凭据索引）、`candidates_from_snapshot` 过滤。
- `src/candidate.rs` — 候选过滤与凭据解析：`resolve_candidate(unit, channel)`
  把 (group, model) 匹配的路由单元解析成可转发目标（secret / base_url）。
- `src/health.rs` — `MemoryHealthTable`：EWMA 延迟质量分、失败连击熔断、
  冷却窗口（冷却中再失败顺延）、慢启动渐进恢复。
- `src/selector.rs` — `WeightedSelector`：priority 分层（DESC，逐层 fallthrough）
  + 层内按 `(weight+1) × slow_start × latency_quality` 加权随机。
- `src/retry.rs` — `Failover` 状态机：已试排除集 + 尝试预算；`RetryPolicy` /
  `AttemptOutcome` / `RetryLoop` 编排接口。

## 实现对照

| 机制 | 参考实现 | 本 crate 落地 |
|---|---|---|
| 候选索引 | new-api `route_unit_selector.go` `group2alias2routes` | `Snapshot.units` 按 (group, public_model) 精确过滤；别名已显式化，不做模糊匹配 |
| 权重打分 | new-api `routingBaseWeight` (weight+1)、wildtoken `selectWeightedByPriority` | priority 降序分层，顶层正权重优先；weight=0 仍以最低份额参与 |
| 健康状态 | new-api `channel_model_health.go` 隔离阶梯、wildtoken `health.go` 整数分+定时恢复 | EWMA 延迟质量分（≥5 样本生效）+ 5 连击熔断 + 30s 冷却 + 慢启动 0.5→1.0 |
| 重试编排 | sub2api `failover_loop.go` `FailoverState` | `Failover` 排除集 + `max_attempts` 预算；时序退避由上层循环控制 |

## 明确不做（边界）

- 限流（滑动窗口 RPM）→ `gateway-gate` 的 `gate/ratelimit.rs`。
- 会话亲和（affinity）→ V2，等 `session_hash` 提取规则定型。
- 共享份额修正（new-api `routestats` share-deficit）→ 需要观测窗口，热路径成本高，暂不引入。
- 别名模糊归一（new-api `FormatMatchingModelName`）→ `RouteUnitRecord` 已显式分离 public/upstream 模型名。

## 验收

```sh
cargo test -p dispatch
cargo clippy -p dispatch --all-targets
```