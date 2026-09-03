# `crates/admin-api`

## 功能 crate

- `store/` — 数据库连接、migration、事务和核心实体 CRUD。
- `sync/` — 管理中心与 gateway 配置同步。
- `catalog/` — 渠道、模型、分组、Token、路由单元和用户配置。
- `billing/` — 订单、支付、兑换码、订阅和配额交易。
- `observe/` — 请求用量、聚合、排行、性能和渠道监控。
- `ops/` — 后台任务、探活、通知、系统选项和实例运行。

## 单机 MVP 先开发

- `store/` — 建立 schema 和渠道、Token、用户组数据读写。
- `catalog/` — 提供渠道、Token、模型与路由单元管理 API。
- `observe/` — 记录每次生成的模型、token、成本和耗时。
- `ops/` — 提供渠道测试和基础任务。

