# `apps`

## 应用

- `api/` — 后端进程，组装 gateway、admin-api、tavern-api 和 harness runtime。
- `web/` — 管理后台浏览器应用，组装 admin-web。
- `tavern-web/` — 酒馆浏览器应用，组装 tavern-web 和 harness-ui。

## `api/` MVP

### `api/src/main.rs`

- 读取 `config/config.toml`。
- 初始化 PostgreSQL。
- 初始化日志。
- 加载渠道、Token、RouteUnit 到 gateway 内存快照。
- 执行 store migration。
- 组装 `/v1/*`、`/admin/*`、`/tavern/*` 路由。

### `api/src/tavern.rs`

- 初始化 tavern DataRoot 和 default-user 目录。
- 挂载 characters、chats、settings、secrets、generate 路由。

### `api/tests/tavern.rs`

- 角色创建、列表、读取和删除 HTTP 往返。
- 聊天保存、读取和最近列表 HTTP 往返。
- 设置未知字段往返。
- 密钥状态不含明文。
- 路径穿越拒绝。

### 下一项

- 把现有 `api/src/{gateway,dispatch,adapter,identity,billing,ratelimit}.rs` 的单渠道转发迁到 `crates/gateway/*`。
- 挂载 `admin-api/catalog` 管理路由。
- 生成结束后调用 `admin-api/observe::record_usage`。

## `tavern-web/` MVP

### `tavern-web/Cargo.toml`

- package 名 `tavern-web-app`。
- 依赖 `tavern-web/{client,state,ui,page-characters,page-chat,page-settings}`。
- 加入根 workspace members。

### `tavern-web/src/main.rs`

- 启动 Dioxus web 应用。

### `tavern-web/src/lib.rs`

- 角色、聊天、设置三个路由。
- 顶部导航。
- 向页面注入 TavernState。

### `tavern-web/Dioxus.toml`

- 参照 `apps/admin-web/Dioxus.toml` 配置 web target、资产目录和开发端口。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-web-app
```

浏览器走通：创建角色 → 进入聊天 → 发消息 → 流式回复 → 中止 → swipe → 刷新恢复。

## `web/` MVP

### `web/src/lib.rs`

- 组装 admin-web 页面路由。
- 用真实 `page-*/api.rs` 数据替换 mock。

### 验收

浏览器走通：登录 → 建渠道 → 建 Token → 查看总览和用量。
