# `crates/admin-api`

## 目录

```text
admin-api/
├── store/
├── sync/
├── catalog/
├── billing/
├── observe/
└── ops/
```

## 要实现

- `store` 提供 PostgreSQL schema、migration、事务、CRUD、审计和 outbox。
- `sync` 同步中心与边缘配置快照和增量。
- `catalog` 管理渠道、分组、Token、路由单元和模型元数据。
- `billing` 管理订单、支付、兑换码和配额交易。
- `observe` 记录用量、聚合统计、排行和渠道监控。
- `ops` 运行探活、任务、通知、系统选项和备份。
