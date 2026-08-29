# Ferrite 根契约目录

此目录是 center PostgreSQL、edge embedded store、Rust capability crate 和 Web/API 之间的逻辑契约边界。

## 目录规划

```text
contract/
├── README.md
├── records/       RouteUnit、Channel、Token、UsageEvent、HealthObservation、QualityBucket
├── mutations/     Mutation、MutationId、Cursor、VersionSummary、DeltaRange
├── api/           Web/API request-response shapes
└── schema/        序列化格式、版本兼容规则、fixtures
```

## 约束

- 逻辑 schema 对所有节点相同；物理后端可以不同；
- 不放 PostgreSQL 专属 DDL；
- 不放 Fjall/redb 专属 key encoding；
- 不放 Axum handler 实现；
- 每个同步 record 有 key、schema version、logical version、origin 和时间；
- 新字段必须有兼容默认值；删除字段必须经过停写和升级窗口；
- `MutationId` 重复应用必须幂等；
- Web/API DTO 从契约转换，不直接暴露存储 record。

具体 record 只有在对应路线的 D0/S0 phase 开始时创建，避免提前编造字段。
