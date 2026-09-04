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

### 已完成 — `auth/` (用户 9 端点)

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/user/login` | 登录 → access JWT (15min) + refresh (7d) |
| POST | `/api/user/register` | 自注册，argon2id |
| POST | `/api/user/refresh` | refresh 旋转，并发重放拒 |
| POST | `/api/user/logout` | 吊销 sid |
| GET/PUT/DELETE | `/api/user/self` | 自查 / 改昵称改密 (auth_version++) / 注销 |
| GET | `/api/user` | admin 用户列表 (search/page/size) |
| GET | `/api/user/search` | admin 搜索 (ILIKE，前 20 条) |
| GET | `/api/user/{key}` | admin 单查 |
| POST | `/api/user/manage` | admin: enable/disable/set_role/adjust_quota/reset_password |

### 已完成 — `catalog/` tokens (6 端点)

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/token` | 创建，明文 sk- 只返一次，库存 sha256 |
| GET | `/api/token` | 列表 (all=true 需 admin) |
| GET | `/api/token/search` | 搜索 |
| PUT/DELETE | `/api/token/{key}` | 编辑 / 删除 (owner 或 admin) |
| POST | `/api/token/{key}/key` | 重取明文 = 重新生成（旧 key 即刻失效，不做可逆存储） |

### 已完成 — `catalog/` channels (9 端点)

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/channel` | 创建 (name 唯一, keys 非空, base_url 必填 http(s), models=[{alias,upstream}]) |
| GET | `/api/channel` | 列表 (keys 掩码) + search |
| GET | `/api/channel/{key}` | 单查 (含完整 keys) |
| PUT | `/api/channel/{key}` | 更新 (合并后整体校验) |
| POST | `/api/channel/{key}/status` | 启停 |
| POST | `/api/channel/{key}/test` | 探活：reqwest 真调 chat/completions (max_tokens=1, 10s 超时)，结果落 monitor_history |
| POST | `/api/channel/test` | 全量探活 (启用渠道串行，配置缺失记 config 错误不中断) |
| DELETE | `/api/channel/{key}` | 删除 |

表 `api_channels` 字段覆盖 gateway `dispatch::ChannelConfig` 所需，
apps/api 迁移读这张表后 kv_store JSON blob 可废弃。

### 已完成 — `catalog/` groups (4 端点)

| 方法 | 路径 | 说明 |
|------|------|------|
| GET/POST | `/api/group` | 列表 / 创建 (default 组保留名, ratio>0) |
| PUT/DELETE | `/api/group/{key}` | 编辑倍率白名单 / 删除 (有引用拒删) |

表 `api_groups` 启动时 seed default 组；auth_users.group_id /
api_tokens.group_id / api_channels.group_name 按名字引用 (loose)。

### 已完成 — `observe/` logs + dashboard (5 端点)

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/log` | admin 全量 (log_type/username/token_name/model_name/时间范围) |
| GET | `/api/log/stat` | 今日 quota/requests + rpm/tpm (60s 窗口) |
| GET | `/api/log/self` + `/self/stat` | 用户自查 |
| GET | `/api/dashboard` | 汇总 (users/tokens/channels/groups/今日用量/rpm/tpm) |

表 `usage_logs` (BIGSERIAL, log_type: 1=topup 2=consume 3=manage 4=system)。
网关侧调 `observe::logs::LogService::record(&UsageEvent)` 写入。

### 已完成 — `observe/` monitor (2 端点)

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/monitor/{key}?days=7&limit=50` | 渠道探活历史 + 可用率 (total/ok_count/availability/avg_latency_ms) |
| GET | `/api/monitor?days=7` | 全渠道可用率一览 |

表 `monitor_history` (BIGSERIAL) 由探活执行方 (`catalog::channels::test_channel`) 写入；
`MonitorDeps` 是落库/查询的封装，ops::probe 后续复用。

### 之后 — 未做

- ops::jobs 后台 runner（探活定时调度，当前探活为手动触发）
- 小时聚合 / 排行 (usage_hourly / model_rankings) — 数据量起来再做
- 兑换码 / 支付 (billing 域)
