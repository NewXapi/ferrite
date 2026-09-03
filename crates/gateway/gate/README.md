# `gateway-gate`

## 目录

```text
src/
├── lib.rs
├── auth.rs
├── chain.rs
├── concurrency.rs
├── graylist.rs
├── model.rs
├── quota.rs
├── ratelimit.rs
├── snapshot.rs
└── state.rs
```

## 要实现

- API Key 提取和认证。
- Token 状态、IP 和模型白名单。
- 配额、限流、灰名单和并发槽。
- GateChain。
- 内存快照更新。
