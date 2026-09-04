# `store`

## 文件

- `src/lib.rs` — 公开 Store、PgStore、migration 和错误。
- `src/traits.rs` — 定义渠道、Token、用户、路由、用量与会话存储接口。
- `src/error.rs` — 定义存储错误。
- `src/pg/mod.rs` — 实现 PostgreSQL 连接池、事务和 SQL 查询。
- `src/migrations/V1__core.sql` — 创建用户、渠道、Token、路由单元、用量、会话、outbox 和审计表。
- `src/migrations/mod.rs` — 执行嵌入 SQL migration。
- `src/embedded/mod.rs` — 实现单机本地存储。


## 实现状态

**设计态骨架** — traits/pg/embedded 均为 `todo!()` 占位（TODO(#4xx)）。
单机版阶段，auth/catalog/observe 各子域以平表直连 sqlx 自行落库（见各自 README），
不走本 crate 的 store trait；sync/outbox/分区设计待数据稳定后再启用。
