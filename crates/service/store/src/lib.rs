//! # store — 存储端口 + 两个实现
//!
//! ```text
//! capability CRUD API
//!   → Store port (traits.rs, 本模块)
//!       ├── pg/       PostgresStore (SQLx, center)   ← migrations/ DDL 在此
//!       └── embedded/ EmbeddedStore (Fjall, edge)    ← key encoding 锁在此
//! ```
//!
//! 铁律 (02-b 路线图):
//! 1. handler/业务代码**永不**出现 backend 分支 — 同一 trait 两个实现;
//! 2. 写 record 与写 mutation journal 必须同一事务/写批原子完成;
//! 3. 重复 append 同一 MutationId 幂等去重。

pub mod embedded;
pub mod error;
pub mod migrations;
pub mod pg;
pub mod traits;

pub use error::StoreError;
pub use traits::{
    ChannelStore, GroupStore, RouteUnitStore, TokenStore, UsageStore, UserStore,
};
