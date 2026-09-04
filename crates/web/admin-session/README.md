# `session`

## 文件

- `src/lib.rs` — 定义 User、SessionInfo、AuthBundle、SessionState 和公开认证函数。
- `src/login.rs` — 调用登录和二次验证 API。
- `src/manage_session.rs` — 初始化、保存、清除和读取全局 session signal。
- `src/refresh_token.rs` — 调用 refresh API 并更新 access token。

