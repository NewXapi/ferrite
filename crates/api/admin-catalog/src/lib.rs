//! # catalog — 渠道 / 模型 / 分组 / Token (平表直连)
//!
//! 单机平表架构下各实体的 CRUD 与业务校验全部在本 crate，直接以
//! `sqlx::PgPool` 读写 `api_channels` / `api_models` / `api_groups` /
//! `api_tokens` 平表。auth 域负责用户管理，不在本 crate 重复。
//!
//! 路由单元（group × model × channel）不落库：由 `apps/api` 的 snapshot
//! 从 `api_channels.models` JSONB × `groups TEXT[]` 在内存展开
//! （见 `apps/api/src/snapshot.rs::expand_models_json`）。
//!
//! ## 模块地图
//!
//! | 模块 | 职责 |
//! |------|------|
//! | [`channels`]   | 渠道 CRUD + 校验 + 探活触发 |
//! | [`groups`]     | 分组 CRUD + 白名单 |
//! | [`tokens`]     | 令牌生命周期 (创建含一次性明文/吊销) |
//! | [`models`]     | 模型 CRUD + 缺失模型发现 |

pub mod channels;
pub mod groups;
pub mod models;
pub mod tokens;
