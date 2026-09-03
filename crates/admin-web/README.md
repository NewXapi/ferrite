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

- 渠道、Token、RouteUnit 的表单字段和 DTO 映射。

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
