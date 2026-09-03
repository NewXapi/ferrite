# `contract`

## 文件

- `src/lib.rs` — 公开 API DTO、records、mutation、错误和 schema。
- `src/api.rs` — 响应信封和 API 模块。
- `src/api/admin.rs` — 渠道、用户组、路由、系统管理请求和响应。
- `src/api/auth.rs` — 登录、刷新和会话请求和响应。
- `src/api/token.rs` — API Key 创建、列举和状态。
- `src/api/usage.rs` — 用量日志和统计查询。
- `src/api/user.rs` — 用户资料、配额和角色。
- `src/records.rs` — 领域记录模块。
- `src/records/channel.rs` — Channel、RouteUnit 和凭据元数据。
- `src/records/identity.rs` — 用户、Token、会话和授权。
- `src/records/routing.rs` — 模型、组和路由选择配置。
- `src/records/usage.rs` — 请求 token、成本、延迟和状态。
- `src/records/billing.rs` — 订单、兑换码和余额交易。
- `src/mutations.rs` — MutationId、Cursor、版本摘要和 Ack。
- `src/error.rs` — 内部错误码和 HTTP/API 映射。
- `src/schema.rs` — DTO 和 records 的 JSON 兼容规则。

## 约束

- 不依赖 tokio、sqlx、axum、reqwest、dioxus。
- 必须通过：

```sh
cargo check -p contract --target wasm32-unknown-unknown
```

- SQL DDL 和存储编码不在本 crate。

## 开发

新增管理 API 字段时，先改 `src/api/` 和 `src/records/`，再改 `admin-api` 和 `admin-web`。
