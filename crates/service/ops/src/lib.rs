//! # ops — 运维域 (center): 任务、设置、通知
//!
//! ## 系统任务 runner — 替代 sub2api Redis 队列的 PG 原生方案
//!
//! 参考: new-api store_system_task.go (DB lease + heartbeat) 与 sub2api 的
//! `FOR UPDATE SKIP LOCKED` 认领。**无 Redis 的多实例正确性靠 PG**:
//!
//! ```sql
//! UPDATE system_jobs SET state='running', leased_by=$node,
//!        lease_expires=now() + interval '5 min'
//! WHERE key = (
//!   SELECT key FROM system_jobs
//!   WHERE state='pending' OR (state='running' AND lease_expires < now())
//!   ORDER BY created_at FOR UPDATE SKIP LOCKED LIMIT 1
//! ) RETURNING *;
//! ```
//! 心跳续租; 崩溃实例租约过期 → 任务自动回可认领态。
//!
//! ## 选主 (singleton 周期任务)
//!
//! sub2api 用 Redis SetNX; 我们用 `pg_try_advisory_lock` — 会话级, 崩溃自动掉主。
//! 只影响"谁跑周期任务", 不影响数据正确性 (原则 10 的例外边界已明示)。
//!
//! ## 模块地图
//!
//! | 模块 | 职责 |
//! |------|------|
//! | [`jobs`]    | 任务 runner + handler 注册表 |
//! | [`options`] | 运行时选项 (key/value + 类型化校验) |
//! | [`notify`]  | 通知 (email/webhook/bark) — 渠道熔断告警的下游 |
//! | [`probe`]   | 渠道探活 handler (observe::monitor 的执行侧) |

pub mod jobs;
pub mod notify;
pub mod options;
pub mod probe;
