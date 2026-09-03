# contract — 逻辑契约 crate

全 workspace 的单一事实来源: center PostgreSQL / edge Fjall / web wasm / apps/gateway 共享同一份类型。

```text
src/
├── lib.rs        模块树 + 铁律
├── api.rs        前端 ↔ console 的 REST DTO (Envelope / UserDto / TokenDto / UsageLogDto + 端点登记表)
├── records.rs    领域实体 (SyncMeta 信封 + Channel/Group/User/Token/RouteUnit/UsageEvent/Health)
├── mutations.rs  增量同步: MutationId / DeltaRange / VersionSummary / Ack
└── schema.rs     版本兼容规则 + fixtures (mock crate 退役后的替代品)
```

## 铁律

1. **零 runtime 依赖**: 不许出现 tokio / sqlx / axum / reqwest / dioxus。
   机器检查: `cargo check -p contract --target wasm32-unknown-unknown` 必须通过。
2. **不放物理编码**: 无 SQL DDL (→ `service/store/migrations/`), 无 Fjall key encoding (→ store)。
3. **DTO 与 Record 分离**: 前端只看 `api.rs` 的 Dto, 存储细节 (key_hash / origin) 不出契约。
   转换用 `From<&Record> for Dto` 写在本 crate, 三方共享。
4. **字段变更走兼容规则**: 见 `schema.rs` 头注释 (新增要 default, 删除要走停写窗口)。

## TODO 编号段 (issue 待建, 段位预留)

| 段 | 域 |
|----|----|
| #2xx | contract 本体 (字段/端点/fixtures 待定项) |
| #3xx | gateway 域 (admission/dispatch/forward/metering) |
| #4xx | service 域 (catalog/sync/store) |
