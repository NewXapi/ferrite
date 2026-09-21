# tavern-generate

生成请求转发 + SSE 透传 + 中止。

## 职责

把酒馆的生成请求转发到上游，SSE 逐 chunk 透传给前端，支持客户端中止。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 转发 + SSE 透传 + 中止 |

## 依赖

`harness/prompt`（api → harness 越界边）。

## 验收

```bash
cargo check -p tavern-generate
cargo test -p tavern-generate              # CI
```
