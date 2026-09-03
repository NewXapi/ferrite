# `tavern-secrets`

## src/lib.rs

- `read/write/remove` — 服务端密钥读写删除。
- `state` — 返回 key → 是否已配置，供前端显示。

## src/http.rs

- `router` — `GET /tavern/secrets` 与 `PUT/DELETE /tavern/secrets/{key}`。
- `PutBody` — 写入请求体。

## tests/masking.rs

- `密钥测试` — 写读删和状态响应不含明文。

