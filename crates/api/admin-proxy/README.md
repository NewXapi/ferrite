# 功能 crate 约定

每个功能 crate 是独立 Cargo library crate，拥有独立的 `Cargo.toml`、`src/lib.rs` 与 `tests/`。

## 文件结构

- `Cargo.toml` — 依赖声明（跨端依赖必须走 `crates/contract` DTO）
- `src/lib.rs` — 尽量只放共用结构体与 trait，实现在各子文件里
- `src/*.rs` — 按模块拆分实现
- `tests/*.rs` — 集成测试（同层 `tests/`，禁止在 `src/` 留 `#[cfg(test)]`）

## 职责

本 crate 属于 `api` 域（后端服务大域），实现代理节点池管理（proxy node pool）相关功能。

## 验收命令

```sh
cargo check -p <crate_name> --all-targets
cargo test -p <crate_name>        # 本地只调单个用例, 全量推 CI
```
