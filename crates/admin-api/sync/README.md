# `sync`

## 文件

- `src/lib.rs` — 公开同步配置、snapshot 和 mutation API。
- `src/edge.rs` — gateway 拉版本摘要、增量和完整 snapshot。
- `src/center.rs` — 中心提供 delta、push 和确认接口。
- `src/snapshot.rs` — 序列化、校验和原子安装配置快照。

