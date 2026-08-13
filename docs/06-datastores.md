# postgres / log-db / redis — 数据存储

三者放一份，因为它们是外部服务，没有自己的业务代码，但**谁能读写哪张表/哪个 key** 是拆分的核心约束。

---

# postgres — 主库

- 镜像：`postgres:17-alpine`
- 今天：`model/main.go` 的 `var DB`，支持 SQLite / MySQL / PostgreSQL 三方言
- **建议 Rust 侧只保留 PG**（理由见 §4）

## 1. 表清单（34 张，`model/main.go:261-296` AutoMigrate）

### 1.1 身份与访问（control 独占写）

- `User` — `model/user.go`（70 符号）
- `UserSession` — `model/user_session.go`（17 符号）
- `AuthFlow` — `model/auth_flow.go`（11 符号）
- `ExternalIdentityClaim` — `model/external_identity_claim.go`
- `PasskeyCredential` — `model/passkey.go`（10 符号）
- `TwoFA` / `TwoFABackupCode` — `model/twofa.go`（18 符号）
- `CustomOAuthProvider` — `model/custom_oauth_provider.go`（10 符号）
- `UserOAuthBinding` — `model/user_oauth_binding.go`（11 符号）
- `CasbinRule` / `AuthzRole` — `model/casbin_rule.go`、`model/authz_role.go`

### 1.2 路由目录（control 写，gateway 读）

- `Channel` — `model/channel.go`（64 符号）
- `Ability` — `model/ability.go`（14 符号），`(group, model, channel_id)` 路由元组
- `Model` / `Vendor` — `model/model_meta.go`（11）、`model/vendor_meta.go`（8）
- `PrefillGroup` — `model/prefill_group.go`（10 符号）
- `Option` — `model/option.go`（6 符号），键值配置，含 `proxy_config`
- `ProxyNode` — `model/proxy_node.go`（5 符号）
- `Setup` — `model/setup.go`

### 1.3 计费（**gateway 与 control 都写 — 争用点**）

- `Token` — `model/token.go`（29 符号）。gateway 写 `remain_quota`/`used_quota`，control 做 CRUD
- `User.quota` — gateway 写余额，control 写充值
- `TopUp` — `model/topup.go`（15 符号），control 独占
- `Redemption` — `model/redemption.go`（11 符号），control 独占
- `Checkin` — `model/checkin.go`（7 符号），control 独占
- `SubscriptionOrder` / `UserSubscription` / `SubscriptionPreConsumeRecord` — `model/subscription.go`（42 符号）。**gateway 写预扣记录，worker 写终态结算，control 写订单**
- `SubscriptionPlan` — 走方言分支建表（`model/main.go:306-311`）

### 1.4 任务（gateway 写提交，worker 写终态）

- `Task` — `model/task.go`（34 符号）
- `Midjourney` — `model/midjourney.go`（18 符号）

### 1.5 运维（worker 写，control 读）

- `SystemTask` / `SystemTaskLock` — `model/system_task.go`（24 符号）。`SystemTaskLock` 的 `type` 是主键
- `SystemInstance` — `model/system_instance.go`（8 符号）。**所有应用都写**（各写自己那行）
- `PerfMetric` — `model/perf_metric.go`（10 符号）
- `QuotaData` — `model/usedata.go`（9 符号），用量小时聚合
- `Log` — 主库也建了这张表（当 `LOG_SQL_DSN` 未设置时用主库）

## 2. 表 → 应用的读写矩阵

只写关键的争用点：

- `Token`、`User.quota` — **gateway 高频写 + control 低频写**。gateway 侧必须用单行原子
  `UPDATE q = q - ?`，不能读改写
- `Channel.channel_info` — **gateway 写多 key 轮询索引（`MultiKeyPollingIndex`，`model/channel.go:68`）
  + control 写渠道配置**。今天靠进程内 `channelPollingLocks`（`:614`）串行化，多副本失效 →
  索引移 Redis
- `Channel.status` — **gateway 自动禁用（`processChannelError` → `DisableChannel`）+ control 手动改**
- `SubscriptionPreConsumeRecord` — **gateway 预扣 + worker 结算**，靠记录 ID 对齐
- `SystemInstance` — 所有应用写，各写自己 `node_name` 那行，无争用
- `Task` — **gateway 提交 + worker 轮询更新**

## 3. 行锁

- `model/locking.go` `lockForUpdate(tx)` — MySQL/PG 发 `clause.Locking{Strength: "UPDATE"}`，
  SQLite 跳过（语法不支持）
- 使用处：用户余额升级路径、订阅 upsert、`model/user_auth_cache.go:180` `IncrementUserAuthVersionWithTx`
- 根 `AGENTS.md` 明确规定：`model/` 里的 `SELECT ... FOR UPDATE` 必须走这个 helper，
  不能用 GORM v1 的 `tx.Set("gorm:query_option", "FOR UPDATE")`（GORM v2 会静默忽略，锁根本没加上）

## 4. 三方言：建议只留 PG

今天为了兼容三方言存在的复杂度：

- `model/main.go` 的 `commonGroupCol` / `commonKeyCol` — `group` 和 `key` 是保留字，
  PG 用 `"col"`、MySQL/SQLite 用 `` `col` ``
- `commonTrueVal` / `commonFalseVal` — 布尔字面量差异
- `model/locking.go` `lockForUpdate` — SQLite 整个跳过
- `model/channel.go:278-318` `channelGroupFilterCondition` — MySQL `CONCAT` vs `||`
- `model/ability.go` `FixAbility` — SQLite `DELETE FROM` vs MySQL/PG `TRUNCATE TABLE`
- 迁移：SQLite 只能 `ADD COLUMN`，不能 `ALTER COLUMN`
- `model/main.go` 里 `migrateTokenModelLimitsToText`、`migrateSubscriptionPlanPriceAmount` 等方言修正

sqlx 的编译期查询校验也只对单方言生效。**这些是 Go 版的历史包袱，别背过来。**

---

# log-db — 日志库

- 镜像：`postgres:17-alpine`（或 ClickHouse）
- 今天：`model/main.go` 的 `var LOG_DB`，独立 DSN（`LOG_SQL_DSN`）
- 只有 1 张表：`Log`（`model/main.go:404` `LOG_DB.AutoMigrate(&Log{})`）

## 5. 读写方

- **写** — gateway（消费日志，`model/log.go:343` `RecordConsumeLog` → `:101` `createLog`）、
  worker（任务日志、错误日志）
- **读** — control（`controller/log.go` 的 7 个查询 handler）
- 清理 — worker 的 `log_cleanup` 任务

## 6. 今天的性能问题

`model/log.go:101` `createLog` 是**同步 `INSERT`，在用户响应路径上**。
而且 `:355` 还夹了一次 `GetUserSetting(userId, false)` 判断要不要记 IP。
日志库慢一下，用户请求就慢一下。

**Rust 侧必须改成**：`mpsc` channel + 批量 insert，gateway 侧 fire-and-forget。
这是 `07-inter-app-dataflow.md` F6 要解决的事。

## 7. ClickHouse 支持（暂不动）

`model/main.go` 已支持 `LOG_SQL_DSN` 指向 ClickHouse，但那条路径有：
- 手写 `CREATE TABLE ... ENGINE=MergeTree() PARTITION BY toYYYYMM(...)`
- TTL 维护（`syncClickHouseLogTTL`）
- 专用排序（`model/log.go:106` `clickHouseLogOrder`）

**建议**：先把日志写入改成异步批量，稳定之后再考虑换引擎。不要一次动两件事。

---

# redis

- 镜像：`redis:7-alpine`
- 今天：`common/redis.go` 的 `var RDB`，由 `REDIS_CONN_STRING` 初始化
- **gateway 多副本时必需**（否则限流退化为进程内计数，用户拿到 N 倍额度）

## 8. Key 空间清单（已核实）

### 8.1 鉴权缓存

- `token:<hmac>` — token 快照 HSET。`model/token_cache.go:14`。TTL = `common.RedisKeyCacheSeconds()`
- `user:<id>` — 用户快照 HSET。`model/user_cache.go:51`。字段含 `Id`/`Group`/`Email`/`Quota`/`Status`/`Role`/`Username`/`Setting`/`AuthVersion`/`CacheSchema`
- `auth:user:fence:<id>` — 待提交鉴权版本。`model/user_auth_cache.go:27`
- `auth:user:version:<id>` — 已提交鉴权版本下限。`model/user_auth_cache.go:31`
- 旧式单值 key（`constant/cache_key.go:5-8`）：`user_group:<id>`、`user_quota:<id>`、`user_enabled:<id>`、`user_name:<id>`

### 8.2 限流

- `rateLimit:v2:ip:<mark>:<ip>` — `middleware/rate-limit.go:44` `redisIPRateLimitKey`
- `rateLimit:v2:user:<mark>:<id>` — `middleware/rate-limit.go:48` `redisUserRateLimitKey`
- `mark` 取值：`GW`（全局 Web）、`GA`（全局 API）、`CT`（关键操作）、`DW`（下载）、`UP`（上传）
  — `middleware/rate-limit.go:162-197`
- 模型级限流 — `middleware/model-rate-limit.go:86,100` 的 `rateLimit:<...>`
- 脚本 — `middleware/rate-limit.go:22` `redisFixedWindowScript`（INCR + EXPIRE + 判定，原子）。
  注释 `:17-21` 明确说这是**固定窗口**，窗口边界可以突发到 2 倍限额，不要擅自改成滑动窗口 ZSET
- 令牌桶 — `common/limiter/lua/rate_limit.lua`（EVALSHA 缓存，用 Redis `TIME` 保证原子）

### 8.3 业务缓存

- `new-api:channel_affinity:v1:*` — 渠道亲和。`service/channel_affinity.go:29`
- `new-api:channel_affinity_usage_cache_stats:v1:*` — 亲和命中统计。`service/channel_affinity.go:30`
- `sub:<id>` — 订阅缓存。`model/subscription.go:1479`
- `SUB:<...>` — 订阅支付。`controller/subscription_payment_epay.go:104`
- `perf:<...>:<...>:<ts>` — 性能指标小时桶。`pkg/perf_metrics/metrics.go:427`，TTL 1 小时
- `notify_limit:<uid>:<...>:<...>` — 通知频率限制。`service/notify-limit.go:58`
- `file_cache_<hash>` / `b64_cache_<hash>` — 文件与 base64 缓存。`service/file_service.go:30,42`

### 8.4 新增：失效总线

拆分后必须加一个 pub/sub channel（详见 `07-inter-app-dataflow.md` F8）。
今天完全不存在 —— `grep -rn 'Subscribe|Publish'` 只命中 `model.PublishUserAuthCache`
（`model/user_auth_cache.go:229`），那是误导性命名，实际是 `GetUserById` + `updateUserCache`。

## 9. 鉴权版本栅栏（唯一非平凡的一致性机制）

`model/user_auth_cache.go:250-271` 的 Lua 脚本，三个 key 协同：

```
incoming  = 要写入的版本
pending   = GET auth:user:fence:<id>
committed = GET auth:user:version:<id>
current   = HGET user:<id> AuthVersion

若 pending > incoming 或 committed > incoming 或 current > incoming → 拒绝写入（返回 0）
若 committed < incoming → SET committed = incoming
若 0 < pending <= incoming → DEL fence
```

配合 `model/user_auth_cache.go:180` `IncrementUserAuthVersionWithTx`：
**在 DB 事务提交前先发 fence（fail-closed），提交成功后才发 committed 版本**。
这样密码修改、角色变更、禁用用户能立即生效，不会被并发的旧快照写回覆盖。

**Rust 侧必须原样保留这个脚本的语义。** 这是安全边界，不是性能优化。

---

## 10. 目录结构（公共 crate）

数据存储对应 3 个公共 crate，被多个二进制链接：

```
crates/store/                     # postgres
├── Cargo.toml                    # sqlx + PG only
├── migrations/                   # sqlx migrate（替掉 GORM AutoMigrate 34 表）
│   ├── 0001_init.sql
│   └── ...
└── src/
    ├── lib.rs                    # 连接池, tx helper
    ├── lock.rs                   # ← model/locking.go lockForUpdate（PG 版, 无方言分支）
    ├── identity/                 # §1.1 user session auth_flow passkey twofa oauth casbin
    ├── catalog/                  # §1.2 channel ability model vendor prefill option proxy_node
    ├── billing/                  # §1.3 token topup redemption checkin subscription
    ├── task/                     # §1.4 task midjourney
    └── ops/                      # §1.5 system_task system_instance perf_metric quota_data

crates/log-store/                 # log-db
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── writer.rs                 # mpsc + 批量 insert（替掉 model/log.go:101 同步 createLog）
    ├── query.rs                  # ← controller/log.go 的 7 个查询
    └── usage.rs                  # ← model/usedata*.go（3 文件）

crates/cache/                     # redis
├── Cargo.toml
└── src/
    ├── lib.rs                    # ← common/redis.go
    ├── token.rs                  # ← model/token_cache.go:14
    ├── user.rs                   # ← model/user_cache.go:51
    ├── auth_fence.rs             # ← model/user_auth_cache.go:250-271（§9, 原样保留）
    ├── ratelimit.rs              # ← middleware/rate-limit.go:22,44,48 + limiter/lua/
    ├── hybrid.rs                 # ← pkg/cachex/hybrid_cache.go（Redis + 本地 LRU）
    ├── affinity.rs               # ← service/channel_affinity.go 的缓存部分
    ├── metrics.rs                # ← pkg/perf_metrics/metrics.go:427
    └── invalidation.rs           # 新增: F8 失效总线
```

链接关系：
- `store` — control、worker、gateway（窄接口：只要计费写 + 渠道读）
- `log-store` — gateway（写）、worker（写+清理）、control（读）
- `cache` — 全部
