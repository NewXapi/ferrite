# `gateway-dispatch`

## 目录

```text
src/
├── lib.rs
├── candidate.rs
├── health.rs
├── retry.rs
└── selector.rs
```

## 要实现

- 候选路由匹配。
- 渠道健康度。
- 优先级、权重和 EWMA 选路。
- 失败重试与已试路由排除。
