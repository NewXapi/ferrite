# tavern-storage

酒馆数据根目录与文件读写底座。

## 职责

所有 tavern-* crate 的存储底座：解析 DataRoot、按用户隔离的路径生成、文件原子读写。
tavern 域其他 crate 不允许自己拼路径。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | DataRoot 解析 + 用户隔离路径 + 原子读写 |

## 验收

```bash
cargo check -p tavern-storage
cargo test -p tavern-storage               # CI
```
