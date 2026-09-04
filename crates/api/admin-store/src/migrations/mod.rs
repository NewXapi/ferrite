//! migrations — DDL 唯一存放地 (contract 铁律 2)。
//!
//! 迁移纪律:
//! - 文件名 `V<n>__<slug>.sql`, 单向只增; 已应用的版本记在 `_migrations` 表;
//! - 嵌入 (include_str!) 保证二进制与 schema 同步;
//! - 破坏性变更 (删列/改类型) 必须写迁移注释 + 停写窗口说明。

/// 已登记的迁移清单 (顺序执行)。
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("V1__core.sql", include_str!("V1__core.sql")),
];

/// 迁移执行器 — pg/embedded 各自实现 (SQLx migrate / Fjall 元数据表)。
pub trait MigrationRunner: Send + Sync {
    fn run_all(&self) -> impl Future<Output = Result<(), crate::error::StoreError>> + Send;
}
