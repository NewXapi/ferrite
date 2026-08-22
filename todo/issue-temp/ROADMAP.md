 生成日期: 2026-08-22
 
 ## 进展
 
 ### 已完成
 - **F1** — Stream SSE 转发
 - **F2** — OpenAI 格式错误响应
 - **F3** — /v1/models 端点
 - **F4** — 渠道热重载
 - **F1.1** — Stream 状态前置检查 (ensure_stream_ok)
 - **G1** — 管理面认证 (require_admin + is_admin)
- **F5** — 请求日志（tracing 统一，滚动 JSON 文件 + 查询 API）
 
 ### 待开发
- F6.x, F7.x, F8.x, F10.x — 见下文
 
 ---

| 能力 | 状态 | 模块 |
|---|---|---|
| TOML 配置 + PG 连接池 | 完成 | config.rs |
| Bearer token 认证 (PG tokens 表) | 完成 | identity.rs |
| 固定窗口限流 (PG rate_buckets, UNLOGGED) | 完成 | ratelimit.rs |
| model → channel 路由索引 (ArcSwap) | 完成 | dispatch.rs |
| OpenAI 透传 (非流式 + SSE 流式) | 完成 | adapter.rs, gateway.rs |
| OpenAI 标准错误响应 | 完成 | gateway.rs |
| /v1/models | 完成 | gateway.rs |
| /admin/reload 热重载 | 完成 | gateway.rs |
| Stream 状态前置检查 (F1.1) | 完成 | adapter.rs, gateway.rs |
| 管理面认证 (G1) | 完成 | identity.rs, gateway.rs |
| 请求日志（F5.1+F5.2，tracing → 滚动 JSON 文件） | 完成 | gateway.rs span + main.rs telemetry |
| 日志查询 API (F5.3，读文件过滤) | 完成 | gateway.rs |
| Token 管理 API (F6.1 创建) | 完成 | gateway.rs |
| 渠道管理 API (F7.1 创建) | 完成 | gateway.rs |
| 多协议适配 (F8.x) | 待开发 | — |
| 计费系统 (F10.x) | 待开发 | — |
|---|---|---|
#### 测试策略

每个 feature 实现后补测试代码，放在 `apps/api/tests/` 目录下，按源文件分文件（如 `tests/gateway.rs`、`tests/identity.rs`）。
每个文件一个 happy path + 一个错误路径。不预建集中 test infrastructure。


---

## 1. F5 — 请求日志

拆为 3 个子任务。

### F5.1 — 日志存储 ✅

**实现**: 滚动 JSON 文件（`tracing-appender`，按天滚动到 `logs/ferrite.log.YYYY-MM-DD`，非阻塞写入）。零自研代码、PG 零负担。原 kv_store 方案已废弃。

**验收**: 已完成（冒烟验证：/health 请求落盘，span 上下文完整）。

### F5.2 — 日志写入 ✅

**实现**: 纯现成 crate 组合。tower-http `TraceLayer` 自定义 `make_span_with` 声明业务字段（request_id/user/model/channel/upstream_model/stream/tokens），handler 内 `Span::record` 填充；TraceLayer 的 on_response 事件自动携带全部字段写 JSON 文件。stdout 人类可读层 + JSON 文件层同 registry，各自挂 per-layer EnvFilter。非流式响应解析 usage 记 token 数；流式为空。

**验收**: 已完成。

### F5.3 — 日志查询 API ✅

**范围**: `GET /admin/logs`。

**实现**: 读取 `logs/ferrite.log.*`（文件名降序 = 新→旧），逐行解析 JSON，展平 `fields`/`span`/`spans[0]` 三层后内存过滤。支持 user/model/channel/path/status/since/until 过滤 + limit/offset 分页。仅 admin 可访问（require_admin）。目录不存在返回空列表；损坏行静默跳过。核心逻辑为纯函数 `filter_log_lines`，可独立单测。

**验收**: 已完成。E2E 实测：无 token 401、非 admin 403、admin 200 返回数据；分页 total/页长正确；status/path/since 过滤生效。

**依赖**: F5.2 (已完成), G1 (已完成)。
## 2. F6 — Token 管理 API

拆为 3 个子任务。全部需 G1 (admin 认证)。

### F6.1 — POST /admin/tokens (创建) ✅

**范围**: 创建新 token。

**输入**: `user_id` (可选, 不传则自增), `username`, `quota` (默认值), `group` (默认 `default`), `is_admin` (默认 false)。

**实现**: 生成随机 key (`sk-` + 32 字符 hex)。INSERT 到 tokens 表。返回完整 token（含 key，仅创建时返回明文）。

**验收**: 已完成。E2E 实测：admin 创建 token → 新 token 调 /v1/models 200；非 admin 创建 → 403；空 username → 400。

**依赖**: G1.

### F6.2 — GET /admin/tokens (列表)

**范围**: 列出 token。

**实现**: 直接 `SELECT * FROM tokens`，分页。返回所有字段，key 做掩码（前 8 位 + `...`）。可按 `user_id`、`enabled` 过滤。

**验收**: curl 列表返回已创建的 token，key 掩码，分页生效。

**依赖**: G1.

### F6.3 — DELETE /admin/tokens/:key (禁用)

**范围**: 软删除。

**实现**: `UPDATE tokens SET enabled = false WHERE key = $1`。返回 200 或 404。identity.rs 已有 `enabled` 检查，无需改。

**验收**: curl 创建 token → 请求成功 → 禁用 → 同一 token 请求返回 403。

**依赖**: F6.1, G1.

---

## 3. F7 — 渠道管理 API

拆为 3 个子任务。全部需 G1。渠道存 PG `kv_store`，key 格式 `channel:{id}`。

### F7.1 — POST /admin/channels (创建) ✅

**范围**: 创建渠道配置。

**输入**: `name`, `base_url`, `channel_type` (openai/openai-compat/claude/gemini), `keys` (数组), `models` (数组, 每项 `{alias, upstream}`)。

**实现**: 校验 name 唯一（查现有 channels）、base_url 可解析、models 非空。id 用毫秒时间戳。序列化 ChannelConfig JSON 存入 kv_store `channel:{id}`。返回创建结果。

**验收**: 已完成。E2E 实测：创建渠道 → reload → /v1/models 出现新 alias；重名 → 409；非法 channel_type → 400；非 admin → 403。

**依赖**: G1.

### F7.2 — GET /admin/channels (列表)

**范围**: 列出所有渠道。

**实现**: 读 `kv_store WHERE key LIKE 'channel:%'`，反序列化返回数组。可按 `channel_type` 过滤。

**验收**: curl 列表返回已配置的渠道。

**依赖**: G1.

### F7.3 — PUT/DELETE /admin/channels/:id (更新 + 删除)

**范围**: 更新和删除渠道。

**实现**:
- PUT: 读现有配置 → 合并更新字段 → 写回 kv_store。
- DELETE: 从 kv_store 删除 `channel:{id}`。
- 两者操作后返回时附带提示：调用 `POST /admin/reload` 让新配置生效。

**验收**: curl 更新渠道 → reload → 新配置生效；删除 → reload → 该 model 不可路由。

**依赖**: F7.1, G1.

---

## 4. F8 — 多协议适配

拆为 5 个子任务。结构性最大的重构。

### F8.1 — 协议适配 trait

**范围**: 把 adapter.rs 从 OpenAI 硬编码改为 trait 抽象。

**实现**: 定义 `ProtocolAdapter` trait，方法: `build_url(&self, base_url, path) -> String`, `build_headers(&self, api_key) -> HeaderMap`, `build_body(&self, body, upstream_model) -> Value`, `classify_error(&self, status, body) -> ErrorKind`。OpenAI 实现迁移到 trait。channel 的 `channel_type` 决定用哪个 adapter。dispatch 解析时记录 channel_type。

**验收**: 现有 OpenAI 透传行为不变；非流式和流式都走 trait。

**依赖**: 无（重构现有代码）。

### F8.2 — Claude Messages API 适配

**范围**: Claude 原生协议。

**实现**: Claude Messages API: URL `{base_url}/v1/messages`，header `x-api-key` + `anthropic-version`，请求体格式不同（`messages` 数组、`max_tokens` 必填、`system` 独立字段）。流式格式不同（`event: ` 前缀，`content_block_delta` 等事件类型）。错误分类: 400 不重试，429 重试，5xx 重试，401/403 计熔断不重试。

**验收**: Claude 格式请求 → 转发到 Claude 渠道 → 正确响应；SSE 格式正确转发。

**依赖**: F8.1.

### F8.3 — Gemini generateContent 适配

**范围**: Gemini 原生协议。

**实现**: Gemini: URL `{base_url}/v1beta/models/{model}:generateContent?key={api_key}`，请求体 `contents` 数组，响应 `candidates` 结构。流式用 `streamGenerateContent` + SSE。错误分类同上。

**验收**: Gemini 格式请求 → 转发到 Gemini 渠道 → 正确响应。

**依赖**: F8.1.

### F8.4 — OpenAI ↔ Claude 协议互转

**范围**: 跨协议转换。

**实现**: 客户端发 OpenAI 格式，channel 是 Claude 类型 → 请求体转换 (messages 映射、role 映射、max_tokens 补默认、system 提取)。响应体转换回 OpenAI 格式 (content → choices, usage 字段映射)。流式: 逐 chunk 转换 Claude 事件为 OpenAI SSE 格式。反向（Claude 请求 → OpenAI 渠道）同理。

**验收**: OpenAI 格式请求 → Claude 渠道 → 返回 OpenAI 格式响应，内容正确；流式同理。

**依赖**: F8.2.

### F8.5 — 协议路由

**范围**: dispatch 层根据 channel_type 选择 adapter + 转换。

**实现**: 请求到达时，从 body 取 `model` → resolve 得到 channel → 读 channel_type → 选 adapter。客户端请求格式由 Content-Type 或 body 结构判定（OpenAI 有 `messages`，Claude 有 `messages` + `max_tokens` 必填，Gemini 有 `contents`）。格式不匹配时触发转换。同格式直接透传。

**验收**: OpenAI 请求 → OpenAI 渠道（透传）；OpenAI 请求 → Claude 渠道（转换）；Claude 请求 → Claude 渠道（透传）。

**依赖**: F8.4.

**注**: F8.5 当前不涉及 failover（见 deferred 区）。单 model 多 channel 的 failover 由路由调度工作统一处理。

---

## 5. F10 — 计费系统

拆为 4 个子任务。

### F10.1 — 倍率配置

**范围**: 模型定价。

**实现**: 存 kv_store，key = `pricing:{model}`，value = JSON `{input_per_1k, output_per_1k, multiplier}`。缺失时降级：默认按 1:1 处理（1 token = 1 quota）。后续要看面板/报表聚合时迁 UNLOGGED 表。

**验收**: 配置一个 pricing → 读取生效；未配置不报错。

**依赖**: 无。

### F10.2 — 预扣 (reserve)

**范围**: 请求前预扣估算额度。

**实现**: 请求前: `UPDATE tokens SET used_quota = used_quota + $estimate WHERE key = $1 AND (quota - used_quota) >= $estimate RETURNING used_quota`。0 行返回 → 402 insufficient quota。估算值: 固定 1000 tokens 或 model 历史均值。预扣记录到内存 (request_id → estimate)，结算时用。

**决策 C**: 失败请求（upstream 5xx/network）也消耗 reserve，不补偿。这与现有 ratelimit 行为一致（失败请求计数防滥用）。在运维文档里说明。

**确认**: identity.rs 的 quota 检查是 read-only（不 decrement），与 reserve 不重复计数。无需修改。

**验收**: 余额足够 → 预扣成功，used_quota 增加；余额不足 → 402；upstream 失败 → used_quota 不退还（按决策 C）。

**依赖**: F10.1, G1.
### F10.4 — 充值 API

**范围**: 管理面充值。

**实现**: `POST /admin/recharge`，输入: `token_key` + `amount`。`UPDATE tokens SET quota = quota + $amount WHERE key = $1 RETURNING quota`。需 admin 认证。

**验收**: 充值后 quota 增加；之前 402 的 token 充值后可请求成功。

**依赖**: G1, F10.2.

---

## 6. 依赖关系

```
F1.1 (stream status) ──→ F5.2 (流式错误路径能写正确 status)

G1 (admin auth) ─┬─→ F5.3
                 ├─→ F6.1, F6.2, F6.3
                 ├─→ F7.1, F7.2, F7.3
                 └─→ F10.4

F5.1 → F5.2 → F5.3
F6.1 → F6.3
F7.1 → F7.3
F8.1 → F8.2 → F8.4 → F8.5
F8.1 → F8.3
F10.1 → F10.2 → F10.3
F10.2 → F10.4
F10.2 + F5.2 → F10.3
```

**推荐执行波次**:

| 波次 | 子任务 | 状态 | 理由 |
|---|---|---|---|
| 1 | F1.1, G1 | ✅ 完成 | 当前代码修正 + 唯一前置 |
| 2 | F5.1, F5.2, F6.1, F7.1 | ✅ 完成 | 最小可用：日志写 + token/channel 创建 |
| 3 | F5.3, F6.2, F6.3, F7.2, F7.3 | 待开发 | 管理面补全 |
| 4 | F10.1, F10.2, F10.3, F10.4 | 待开发 | 计费闭环（依赖日志的 token 数） |
| 5 | F8.1 | 待开发 | 协议 trait 重构 |
| 6 | F8.2, F8.3 | 待开发 | Claude + Gemini 适配 |
| 7 | F8.4, F8.5 | 待开发 | 跨协议转换 + 路由 |

F1.1 排在波次 1，是因为它是当前 F1 代码的修正，F5.2 的流式错误路径日志依赖它先完成。

**路由调度（deferred，见 §7）**在所有上面波次之后，且比数据库正式化还要晚。

---

## 7. Deferred — 路由调度与数据库正式化

### 7.1 路由调度（含 G3 故障转移 + F9 健康状态机）

整个调度体系视为同一项工作，包含但不限于：

- **G3 基础故障转移**: `RouteIndex` 改为 model → `Vec<ResolvedRoute>`，handler 加重试循环，错误码映射（400/422 不重试，429/5xx/network/timeout 重试），请求级 exclude。
- **F9.1 健康状态**: kv_store `health:{channel_id}`，存 state/consecutive_failures/cooldown_until/version。
- **F9.2 状态转换**: healthy → calm → dormant，CAS 更新。
- **F9.3 冷却恢复**: 懒恢复（无定时任务），resolve 时检查 cooldown 到期。
- **F9.4 dispatch 集成**: resolve 时过滤不健康渠道。
- **加权选择**: priority 分层 + 同层 weight。
- **breaker**: 401/403 计熔断。
- **affinity/sticky**（可选）: 用户→channel 绑定。

**为什么 deferred**: 你正在 new-api 试点，先验证算法和效果，确认后再迁到 ferrite。ferrite 单 channel 单 model 现状够用，调度需求不紧迫。

**触发条件**（任一）:
- 需要单 model 多 channel（多供应商 fallback）
- 单一渠道故障影响业务，需要自动切换
- 需要 channel 间的负载分担

**优先级**: 比你之前提的数据库正式化还要晚。当前 ferrite 是单 channel per model，能跑通就够。

### 7.2 数据库正式化

**包含**:
- 引入 sqlx migrate
- 把 `logs` (从 kv_store)、`channel_health` (从 kv_store)、`model_pricing` (从 kv_store) 迁到 UNLOGGED 表
- 加索引支持聚合查询（按 user/channel/model 维度）
- 必要时升级到 LOGGED 表

**触发条件**（任一）:
- kv_store 扫描成为性能瓶颈（>10k 条日志时 GET /admin/logs 慢）
- 需要跨进程共享状态（多副本部署）
- 需要事务一致性（如计费+日志原子写）

**优先级**: 在路由调度之前还是之后仍未定，但都比 F5-F10 主线晚。

---

## 8. 子任务总表

| 编号 | 名称 | 规模 | 依赖 | 状态 |
|---|---|---|---|---|
| F1.1 | Stream status 前置检查 | 小 | — | ✅ 完成 |
| G1 | 管理面认证 | 小 | — | ✅ 完成 |
| F5.1 | 日志存储（滚动 JSON 文件） | 小 | — | ✅ 完成 |
| F5.2 | 日志写入（TraceLayer span + record） | 小 | F5.1 | ✅ 完成 |
| F5.3 | 日志查询 API（读文件过滤） | 小 | F5.2, G1 | ✅ 完成 |
| F6.1 | POST /admin/tokens | 小 | G1 | 待开发 |
| F6.2 | GET /admin/tokens | 小 | G1 | 待开发 |
| F6.3 | DELETE /admin/tokens | 小 | F6.1, G1 | 待开发 |
| F7.1 | POST /admin/channels | 小 | G1 | 待开发 |
| F7.2 | GET /admin/channels | 小 | G1 | 待开发 |
| F7.3 | PUT/DELETE /admin/channels | 小 | F7.1, G1 | 待开发 |
| F8.1 | 协议 trait 重构 | 中 | — | 待开发 |
| F8.2 | Claude 适配 | 中 | F8.1 | 待开发 |
| F8.3 | Gemini 适配 | 中 | F8.1 | 待开发 |
| F8.4 | OpenAI ↔ Claude 互转 | 大 | F8.2 | 待开发 |
| F8.5 | 协议路由 | 中 | F8.4 | 待开发 |
| F10.1 | 倍率配置 (kv_store) | 小 | — | 待开发 |
| F10.2 | 预扣 | 中 | F10.1, G1 | 待开发 |
| F10.3 | 结算 | 中 | F10.2, F5.2 | 待开发 |
| F10.4 | 充值 API | 小 | G1, F10.2 | 待开发 |

**Active**: 20 个子任务（5 已完成，15 待开发）。
**Deferred** (§7): 路由调度全部 + 数据库正式化。

---

## 9. 附录：原始功能清单 (TODO.md)

> 合并自 `todo/issue-temp/TODO.md`，2026-08-22。功能已全部拆分到 §1-§8。

### 小功能（改 1-2 个文件，1-2 小时）

**F1 — Stream SSE 转发** ✅ 完成
- 文件: `adapter.rs`, `gateway.rs`
- 验收: `curl -N` 带 `stream:true` 能看到逐字输出

**F2 — OpenAI 格式错误响应** ✅ 完成
- 文件: `gateway.rs`
- 验收: 所有错误路径返回标准 OpenAI error 格式

**F3 — /v1/models 端点** ✅ 完成
- 文件: `gateway.rs`, `dispatch.rs`
- 验收: `curl /v1/models` 返回 `{"object":"list","data":[...]}`

**F4 — 渠道热重载** ✅ 完成
- 文件: `gateway.rs`, `main.rs`
- 验收: PG 插入新渠道 → POST reload → 新模型立即可用

### 中功能（跨 2-3 个文件，半天）

**F5 — 请求日志** 开发中（F5.1+F5.2 完成）
- 文件: `logging.rs`, `gateway.rs`
- 验收: 发一个请求 → 查 `kv_store` 能看到完整记录

**F6 — Token 管理 API** 待开发
- 文件: `identity.rs`, `gateway.rs`
- 验收: curl 创建 token → 用新 token 请求成功 → 删除 → 请求被拒

**F7 — 渠道管理 API** 待开发
- 文件: `dispatch.rs`, `gateway.rs`
- 验收: curl 创建渠道 → reload → 新模型可路由

### 大功能（多天）

**F8 — 多协议适配** 待开发
- 文件: 重构 `adapter.rs`
- 验收: Claude 格式请求 → 转发到 Claude 渠道 → 正确响应；OpenAI 格式 → 转发到 Claude 渠道 → 自动转换

**F9 — 健康状态机** 待开发（deferred 到路由调度）
- 文件: `dispatch.rs`, 新建 `health.rs`
- 验收: 模拟渠道失败 → 进入 calm → 时间到期恢复 → 多次失败进入 dormant

**F10 — 计费系统** 待开发
- 文件: 新建 `billing.rs`, 改 `gateway.rs`, `identity.rs`
- 验收: 请求前后 used_quota 变化正确；余额不足时返回 402
