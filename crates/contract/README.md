# `contract`

全 workspace 的逻辑契约层 (single source of truth)：center PostgreSQL、edge Fjall、
web (wasm) 与 apps/api 都依赖本 crate 获得相同的类型定义。

## 文件

- `src/lib.rs` — 公开 API DTO、records、mutation、错误和 schema。
- `src/api.rs` — 响应信封和 API 模块。
- `src/api/admin.rs` — 渠道、用户组、路由、系统管理请求和响应。
- `src/api/auth.rs` — 登录、刷新和会话请求和响应。
- `src/api/token.rs` — API Key 创建、列举和状态。
- `src/api/usage.rs` — 用量日志和统计查询。
- `src/api/user.rs` — 用户资料、配额和角色。
- `src/records.rs` — 领域记录模块。
- `src/records/channel.rs` — Channel 和凭据元数据。
- `src/records/identity.rs` — 用户、Token、会话和授权。
- `src/records/routing.rs` — 模型、组和路由选择配置。
- `src/records/usage.rs` — 请求 token、成本、延迟和状态。
- `src/records/billing.rs` — 订单、兑换码和余额交易。
- `src/mutations.rs` — MutationId、Cursor、版本摘要和 Ack。
- `src/error.rs` — 内部错误码和 HTTP/API 映射。
- `src/schema.rs` — DTO 和 records 的 JSON 兼容规则。
- `tests/login_response_shape.rs` — 登录响应形状回归测试（真实抓包 JSON）。

## 约束

- 不依赖 tokio、sqlx、axum、reqwest、dioxus。
- 必须通过：

```sh
cargo check -p contract --target wasm32-unknown-unknown
```

- SQL DDL 和存储编码不在本 crate。

## 与后端的对齐约定（2026-09 admin-ui 集成实测）

DTO 的唯一事实来源是**各 admin-api crate 的 `*View` 结构体**（如
`admin-catalog::tokens::TokenView`、`admin-observe::logs::LogView`、
`auth::service::UserView`）。改后端序列化时必须同步本 crate，反之亦然。
以下规则全部来自真实抓包回归测试，改动前先读对应测试。
（注：总览/账户的 DTO 在 #104/#109 合流时以 main 版为准——同结论的并行实现
在 merge 中被裁决收敛,本表描述的是 **main 现行设计**,历史备选写法见 PR #89 讨论。）

### 1. 字段名与信封形状

- 所有 DTO 一律 `#[serde(rename_all = "camelCase")]`。
- 列表端点统一包 `{"items": [...], "total": N}`；auth 系
  （login/register/refresh/self）返回裸 JSON，无信封。
- 客户端（`admin-client`）对两种形状都兼容：先试
  `{success,message,data}` 信封，失败则按裸 JSON 解。

### 2. 容易踩的字段错位（已有测试锁定）

| 位置 | 后端真实形状 | 契约写法 |
|---|---|---|
| `UserDto.role` | 整数 `1/10/100` | `role: u16` 原样承载,语义化走 `role_label()`;消费方不得假设它是字符串 |
| `UserDto.requestCount` | **后端不下发** | `Option<u64>`(None = 后端未统计) |
| `TokenDto` 列表 | 字段直名 `key_preview`(无 `maskedKey`/`plainKey`) | 契约字段就叫 `key_preview`;列表包装用 `TokenList` |
| 创建 token 响应 | 嵌套 `{plaintext, token}` | `CreateTokenResult`，明文只出现一次 |
| `UsageLogDto` | LogView 形状：`modelName`/`createdAt`(RFC3339 字符串)/`quota`/`useTimeMs`/`logType`，**无** success/cost/cached/firstToken 列 | 前端从 `logType==2` 派生成功态、`quota/500000` 换算费用 |
| 兑换码 | `{codePreview, quota, status}`，`DELETE = 停用`（无硬删、无启用） | 前端不提供"重新启用" |

### 3. 数值与状态词表

- **计费单位**：`500000 = ¥1`（`quota`、`used_quota`、兑换码面额同量纲）。
- **token/channel status**：`1=启用 2=停用`，后端写库前校验 `[1,2]`。
- **兑换码 status**：`1=未用 2=停用 3=已核销`；`logType`：`2=消费`。
- **分页参数**：后端 `LogQuery` 是 `page/size`（不是 `p/page_size`），
  `size` 服务端 clamp 到 100。

### 4. 错误语义

- `AuthError::Jwt(_)` 必须 401（过期/签名/格式坏 = 凭证无效），
  前端 401-refresh 链路依赖该语义；`Db/Crypto/Internal` 才是 500。
  锁定测试：`auth/tests/error_status.rs`（auth crate）。

### 5. 分页与过滤参数（admin 端点实测）

| 端点 | 参数 |
|---|---|
| `GET /api/log` | `logType/username/tokenName/modelName/start/end/page/size` |
| `GET /api/log/top` | `by=user\|model&start&end&limit(1-50)` |
| `GET /api/log/trend` | `granularity=hour\|day\|month&start&end`（桶×模型 pivot） |
| `GET /api/redemption` | `status&page&size` |
| `GET /api/monitor` | `days(1-90)` |

`start/end` 均为 RFC3339 (UTC)。

## 测试

```sh
cargo test -p contract --test login_response_shape
```

新增/修改 DTO 时：先用真实抓包 JSON 写回归测试再改实现；
**用 `git log --all --grep` 判断"某修复是否已在 main"会误判
（可能命中的是其他分支），必须 `git merge-base --is-ancestor` 验证。**

## 开发

新增管理 API 字段时，先改 `src/api/` 和 `src/records/`，再改 `admin-api` 和 `admin-web`。
