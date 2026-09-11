# `crates/api` admin 域

## 功能 crate

- `auth/` — 平台账号中心：注册/登录/refresh/logout/self/users/sessions/settings。
- `catalog/` — 渠道、模型、分组、Token、路由单元（平表 CRUD + 校验）。
- `billing/` — 兑换码生成/核销 + 配额事务入账。
- `observe/` — 请求用量日志、今日统计、渠道探活监控。
- `ops/` — 系统选项、系统信息诊断。
- `admin-proxy/` — 代理节点池 CRUD（供网关出站代理热更消费）。
- `admin-router/` — 各域路由聚合 + 管理员鉴权守卫。
- `db-bootstrap/` — 启动期建库/迁移引导。

## 架构约束（单机平表）

- 不引入 store trait 抽象层：各 crate 直接以 `sqlx::PgPool` 读写平表，
  DDL 内嵌在功能源码（`CREATE TABLE IF NOT EXISTS`），启动时执行。
- 不做 sync/分布式：无 center↔edge 同步、无 outbox/租约，单机直连 PG。
- 网关逻辑不进 admin-api：转发/计量在 `crates/gateway/*`，结算结果由
  网关侧写 `usage_logs` 平表，admin-observe 查表聚合。
- 用户管理只在 `auth/`（`/api/user/manage` 覆盖启停/角色/配额调整），
  其他域不重复实现用户写操作。

## API 现状与路线图

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

### 已完成 — `ops/` options (系统选项，3 端点)
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/option` | admin 全部选项（库值回退默认） |
| GET | `/api/option/{key}` | admin 单查 |
| PUT | `/api/option` | admin 写入（注册表校验，未知 key 拒绝） |

表 `options`（key TEXT PK + JSONB value）；首批 5 项：site.registration_enabled / site.quota_new_user / gateway.retry.max_attempts / gateway.timeout.first_byte_ms / observe.retention.usage_days。
### 已完成 — `ops/` system_info (系统信息诊断，1 端点)
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/system-info` | admin 诊断综合指标：运行时环境（OS/arch/hostname）、Uptime、内存（RSS/virt/meminfo）、CPU（核数/loadavg）、数据库连接池状态与核心业务计数 |

### 已完成 — `billing/` redeem (兑换码，4 端点)
| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/redemption` | admin 批量生成（quota>0，count 1..100，明文 `fx-` 只返一次） |
| GET | `/api/redemption` | admin 列表（status 过滤 + 分页） |
| DELETE | `/api/redemption/{key}` | admin 禁用未核销码 |
| POST | `/api/user/topup` | 用户兑换：CAS 核销 + `auth_users.quota` 事务入账 |

表 `billing_redemptions`（code_hash 唯一）；并发核销同一码只有一个成功。

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

### 已完成 — 新增聚合与诊断端点（2026-09）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/log/top` | 消耗 Top 榜（by=user\|model，时间窗聚合） |
| GET | `/api/log/trend` | 用量趋势（date_trunc hour/day/month 桶 × 模型） |
| GET | `/api/monitor`、`/api/monitor/{key}` | 渠道探活可用率（7-90 天窗口） |
| GET | `/api/system-info` | 运行时/内存/CPU/数据库/实体计数诊断 |
| GET/PUT | `/api/option`、`/api/option/{key}` | 系统选项存储 |
| GET/POST | `/api/route_unit`、`/api/route_unit/{key}` | 路由单元 CRUD（拓扑/调度配置） |
| GET/POST | `/api/models`、`/api/models/{key}` 等 | 模型 CRUD/搜索/缺失检测 |
| POST | `/api/gateway/reload` | 网关快照热更（admin 守卫） |

### 之后 — 未做

- ops::jobs 后台 runner（探活定时调度，当前探活为手动触发）
- 小时聚合 / 排行 (usage_hourly / model_rankings) — `/api/log/top`、
  `/api/log/trend` 已按查询时聚合实现；物化预聚合表等数据量起来再做
- 订阅套餐（admin-web 订阅页暂无后端端点）
