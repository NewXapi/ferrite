# tavern-auth

请求身份到用户目录的唯一入口。

## 职责

把 HTTP 请求里的身份解析成 tavern 存储用的用户目录。tavern 域其他 crate 不各自做
鉴权。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 请求身份 → 用户目录解析 |

## 验收

```bash
cargo check -p tavern-auth
cargo test -p tavern-auth                  # CI
```
