# `crates/admin-web`

## 功能 crate

- `client/` — 管理 API 请求、响应信封和 token 注入。
- `session/` — 登录、刷新、登出和全局会话。
- `ui/` — 管理端共享 Dioxus 组件。
- `mock/` — 页面开发的管理数据。
- `page-auth/` — 认证页面。
- `page-overview/` — 总览、模型和排行榜。
- `page-account/` — 个人 Key、用量和奖励。
- `page-admin/` — 渠道、分组、兑换、订阅和系统。
- `page-users/` — 用户列表和用户管理。

## MVP：client + session

### `client/src/setup_client.rs`

- 指向 `apps/api` 的 `/admin/*` API。
- 自动写 Authorization Bearer header。
- 解码 `Envelope<T>`。
- 401 时调用 session refresher。

### `session/src/login.rs`

- 登录，保存 AuthBundle。
- 读取当前用户。

### `session/src/refresh_token.rs`

- refresh API 调用和 token 更新。

### `session/src/manage_session.rs`

- 初始化、保存、读取、清除 SessionState。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p client -p session
```

## MVP：page-admin

依赖 client/session 和 `admin-api/catalog`。

### `page-admin/src/entities.rs`

- 渠道、Token 的表单字段和 DTO 映射。

### `page-admin/src/pages.rs`

- Channels 页面：渠道 CRUD、凭据掩码、测试按钮。
- Routes 页面：模型 + group 到 channel 映射。
- Tokens 页面：创建、列举、启用、删除、模型白名单。

### `page-admin/src/groups.rs`

- 用户组倍率和模型白名单表单。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p page-admin
```

浏览器完成：建渠道 → 绑定模型 → 建 Token → 复制明文 key。

## MVP：page-overview

依赖 `admin-api/observe` 和 `admin-api/ops`。

### `page-overview/src/api.rs`

- 请求总数、成功率、成本、token、模型排行和渠道状态接口。

### `page-overview/src/overview.rs`

- 请求量、token、成本、成功率统计卡。

### `page-overview/src/models.rs`

- 模型请求量和成本分布。

### `page-overview/src/leaderboard.rs`

- 用户、模型、渠道日排行。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p page-overview
```

浏览器完成：发送一次 `/v1/chat/completions` 后，页面显示对应用量。

## MVP：page-account

### `page-account/src/keys.rs`

- 当前用户 API Key 列表、创建、删除和状态切换。

### `page-account/src/usage_logs.rs`

- 当前用户请求日志、模型、token、成本和时间。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p page-account
```

## 后续 crate

- `page-auth/`：注册、二次验证和密码重置。
- `page-users/`：管理员用户管理。
- `mock/`：真实 API 接完后删除页面 API 对 mock 的引用。

## 当前进度（2026-09）

所有页面均已接入真实 admin-api，不再使用 mock 数据：

- **总览**：`/api/dashboard` 统计卡、`/api/log/trend` 逐时/逐日趋势、
  `/api/log/top` 消耗 Top10、`/api/monitor` 渠道可用率。
- **账户**：`/api/token` 密钥管理、`/api/log/self*` 用量日志、
  `/api/user/self` 资料与会话。
- **管理**：`/api/user/users`+`manage`、`/api/group`、`/api/channel`、
  `/api/route_unit`（网络拓扑）、`/api/models`（别名）、
  `/api/redemption`（兑换码）、`/api/system-info`（系统诊断）。
- 订阅页后端暂未实现，页面显示诚实空态。
- 登录态持久化（remember me）+ 401 静默刷新
  （`POST /api/user/refresh`，轮换式 refresh token，并发安全）。

## 管理前端布局约定

页面内容放进同一套栏数的两层网格（统计带独立一行，面板区独立一行），
用栏数分三档：

| 断点 | 宽度 | 栏数 |
|------|------|------|
| 手机（默认） | < 768px | 1 栏 |
| 平板（`md:`） | ≥ 768px | 3 栏 |
| Web（`xl:`） | ≥ 1280px | 5 栏 |

```text
grid grid-cols-1 gap-3 p-4 md:grid-cols-3 md:gap-4 md:p-6 xl:grid-cols-5
```

- **小统计卡**：占 1 栏。
- **宽面板并排**：热力图/图表 `md:col-span-2 xl:col-span-3`，
  列表/分布 `md:col-span-1 xl:col-span-2`。
- **满宽**：`col-span-full`。
- **手机端**全部堆叠；固定最小宽度的内容包 `overflow-x-auto` + `min-w-[Npx]`。

## UI 验证约定

交互元素加 `data-testid`（用 `name` 属性值）；容器加 `role` +
`aria-label`；每页一份 `specs/ui/<page>.yaml` 契约（本地文件，不入库），
供 agent 驱动浏览器做结构化断言（`tab.ariaSnapshot()`）。
