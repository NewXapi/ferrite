# tavern-chats

聊天记录存取：JSONL，一行一条消息。

## 职责

聊天的增删改查与消息追加。消息落盘格式是 JSONL——一行一条消息，追加写，天然适合
流式生成边收边落。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/http.rs` | axum 端点 |
| `src/lib.rs` | 聊天/消息存取 |

## 验收

```bash
cargo check -p tavern-chats
cargo test -p tavern-chats                 # CI
```
