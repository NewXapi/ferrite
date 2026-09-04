# `tavern-auth`

## src/lib.rs

- `Identity` — 当前酒馆用户 handle。
- `DEFAULT_USER` — 单机 MVP 默认用户。
- `resolve` — 从请求头解析身份；当前返回默认用户。
- `Identity::dirs` — 身份映射到 tavern-storage 的 UserDirs。

## tests/identity.rs

- `身份目录测试` — 验证默认用户落到自己的数据目录。

