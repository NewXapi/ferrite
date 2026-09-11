# `gateway-dispatch`

## 文件

- `src/lib.rs` — 公开渠道选择入口和选中路由。
- `src/candidate.rs` — 从内存快照（渠道 models × groups 展开的 RouteUnitRecord）生成候选。
- `src/health.rs` — 更新延迟、失败次数、冷却和慢启动状态。
- `src/selector.rs` — 按优先级、权重和健康分数选渠道。
- `src/retry.rs` — 排除已试渠道并控制最大重试次数。

