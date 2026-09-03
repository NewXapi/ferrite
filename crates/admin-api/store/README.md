# `store`

## 目录

```text
src/
├── lib.rs
├── traits.rs
├── error.rs
├── migrations/
├── pg/
└── embedded/
```

## 要实现

- PostgreSQL 连接池和事务。
- SQL migration。
- 渠道、用户、Token、路由单元和用量 CRUD。
- 审计日志。
- outbox 事件。
- 会话和 auth_version。
