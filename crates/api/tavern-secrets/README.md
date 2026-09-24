# tavern-secrets

用户自己的上游密钥。

## 职责

存用户自带的上游 API key。对外返回一律脱敏，`masking` 测试锁定脱敏形状。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/http.rs` | axum 端点 |
| `src/lib.rs` | 密钥存取 + 脱敏 |

## 验收

```bash
cargo check -p tavern-secrets
cargo test -p tavern-secrets               # CI
```
