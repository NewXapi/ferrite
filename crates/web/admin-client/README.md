# `client`

## 文件

- `src/lib.rs` — 定义 ApiClient、Envelope、ApiResult 和 ApiError。
- `src/setup_client.rs` — 创建同源 gloo-net client，添加请求头并解码响应。
- `src/wire.rs` — 网关渠道健康本地 wire DTO 与 `fetch_gateway_health` 客户端封装（对应 `GET /api/gateway/health`）。
- `src/manage_auth_token.rs` — 保存 access token，注册 refresher，收到 401 后刷新一次。
- `tests/auth_state.rs` — 测试 token 保存、读取和清除。
- `tests/envelope.rs` — 测试管理 API 响应信封解码。

