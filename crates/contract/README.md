# `contract`

## src/lib.rs

- `模块入口` — 公开 API DTO、records、mutation、错误和 schema。

## src/api.rs

- `公共 API` — 响应信封与 API 模块。

## src/api/admin.rs

- `管理 DTO` — 渠道、用户组、路由、系统管理请求响应。

## src/api/auth.rs

- `认证 DTO` — 登录、刷新和会话请求响应。

## src/api/token.rs

- `Token DTO` — API Key 创建、列举和状态。

## src/api/usage.rs

- `用量 DTO` — 日志和统计查询响应。

## src/api/user.rs

- `用户 DTO` — 用户资料、配额和角色。

## src/records.rs

- `记录入口` — 领域记录模块。

## src/records/channel.rs

- `渠道记录` — Channel、RouteUnit 与凭据元数据。

## src/records/identity.rs

- `身份记录` — 用户、Token、会话和授权。

## src/records/routing.rs

- `路由记录` — 模型、组和路由选择配置。

## src/records/usage.rs

- `用量记录` — 请求 token、成本、延迟和状态。

## src/records/billing.rs

- `计费记录` — 订单、兑换码和余额交易。

## src/mutations.rs

- `同步 mutation` — MutationId、Cursor、版本摘要和 Ack。

## src/error.rs

- `错误码` — 内部错误码和 HTTP/API 映射。

## src/schema.rs

- `兼容规则` — DTO 和 records 的 JSON 兼容 fixture。

