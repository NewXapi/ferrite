# `sync`

## 目录

```text
src/
├── lib.rs
├── center.rs
├── edge.rs
└── snapshot.rs
```

## 要实现

- 版本摘要轮询。
- 配置快照和增量 mutation。
- 中心 delta 与 push 接口。
- revision 推进与 snapshot 恢复。
