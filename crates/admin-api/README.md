# `crates/admin-api`

## 功能 crate

- `store/` — 数据库连接、migration、事务和核心实体 CRUD。
- `sync/` — 管理中心与 gateway 配置同步。
- `catalog/` — 渠道、模型、分组、Token、路由单元和用户配置。
- `billing/` — 订单、支付、兑换码、订阅和配额交易。
- `observe/` — 请求用量、聚合、排行、性能和渠道监控。
- `ops/` — 后台任务、探活、通知、系统选项和实例运行。

## MVP：store

`store/` 是管理端和 gateway 的配置权威。

### `store/src/migrations/V1__core.sql`

- `users`：用户名、密码哈希、角色、状态、配额、已用配额、auth_version。
- `channels`：名字、类型、base_url、凭据、状态、priority、weight。
- `tokens`：user_id、key_hash、名字、状态、过期、配额、渠道绑定、模型白名单。
- `route_units`：group、model、channel_id、状态。
- `groups`：名字、倍率、模型白名单。
- `usage_logs`：用户、Token、渠道、模型、输入输出 token、成本、延迟、时间。

### `store/src/traits.rs`

- Channel CRUD。
- Token CRUD 和按 key_hash 查找。
- User 和 Group 读写。
- RouteUnit 读写。
- UsageLog 插入、按用户和模型查询。

### `store/src/pg/mod.rs`

- `PgStore` 连接池，最大 8 连接。
- Channel、Token、Group、RouteUnit、UsageLog 的 SQL 实现。
- 写操作事务。

### `store/src/migrations/mod.rs`

- 启动时执行 migration。

### 验收

```sh
cargo check -p store
cargo test -p store
```

测试覆盖 Token 哈希查找和 UsageLog 插入。

## MVP：catalog

依赖 store。

### `catalog/src/channels.rs`

- 渠道创建、读取、更新、删除。
- 渠道凭据加密保存，查询响应掩码。
- 渠道测试请求。

### `catalog/src/tokens.rs`

- 生成 API Key，只返回一次明文。
- 保存 sha256 哈希。
- 编辑启用状态、过期、配额、渠道绑定和模型白名单。

### `catalog/src/routes.rs`

- 建立 model + group → channel 的 RouteUnit。
- 更新路由单元后输出 snapshot 刷新事件。

### `catalog/src/groups.rs`

- 创建、编辑和列举用户组。
- default 组、倍率和模型白名单。

### `catalog/src/users.rs`

- 用户列表、状态、角色和配额管理。

### 验收

```sh
cargo test -p catalog
```

覆盖凭据加解密、API Key 哈希和 RouteUnit 快照变更。

## MVP：observe + ops

依赖 store。

### `observe/src/lib.rs`

- `record_usage` 写一条请求：模型、渠道、用户、Token、输入输出 token、成本、TTFT、总延迟、状态。

### `observe/src/hourly.rs`

- 按 `(hour, user, model)` upsert 累加。

### `observe/src/perf.rs`

- 记录 TTFT、总延迟和成功率。

### `observe/src/rankings.rs`

- 按用户、模型、渠道计算日用量排行。

### `ops/src/probe.rs`

- 调 catalog 渠道测试请求。
- 写 Observe 渠道状态。

### `ops/src/jobs.rs`

- 认领和执行 channel_probe、hourly_rollup、usage_cleanup。

### 验收

```sh
cargo test -p observe -p ops
```

测试覆盖小时聚合重放不重复累加、渠道探活结果写入。

## MVP 后续 crate

- `sync/`：多实例版本摘要、delta 和 snapshot 同步。
- `billing/`：支付、订单、兑换码和订阅。

## API 路线图（单机版）

约束：不做 sync/分布式，全部平表（内存建全字段、无关联表/FK），
等数据聚合点明确后再分析读写路径优化表结构。gateway 逻辑不进 admin-api。

### 已完成 — `auth/`

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/user/login` | 用户名+密码 → access JWT (15min) + refresh (7d) |
| POST | `/api/user/register` | 自注册，argon2id |
| POST | `/api/user/refresh` | refresh 旋转，旧 sid 即刻吊销 |
| POST | `/api/user/logout` | 吊销 sid |
| GET | `/api/user/self` | Bearer access → 当前用户 |

### 接下来 — `auth/` 扩展（用户自管理 + admin 用户管理）

| 方法 | 路径 | 说明 |
|------|------|------|
| PUT | `/api/user/self` | 改昵称/密码（改密 auth_version++ 全端失效） |
| DELETE | `/api/user/self` | 注销 |
| GET | `/api/user` | admin 用户列表（分页/搜索） |
| GET | `/api/user/{key}` | admin 查单个用户 |
| POST | `/api/user/manage` | admin 启停/改角色/改额度/改密 |
| DELETE | `/api/user/{key}` | admin 软删 |

### 接下来 — `catalog/` token 管理（API Key）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/token` | 令牌列表 |
| POST | `/api/token` | 创建（明文只返一次，库存哈希） |
| PUT | `/api/token` | 编辑启停/过期/配额/模型白名单 |
| DELETE | `/api/token/{key}` | 删除 |
| GET | `/api/token/search` | 搜索 |

### 之后 — 渠道 / 日志 / 计费

- `/api/channel` CRUD + 测试请求（catalog/channels）
- `/api/log` + `/api/log/self`（observe）
- `/api/redemption` 兑换码、`/api/group` 分组倍率（billing）
