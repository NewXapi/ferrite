//! # ops — 运维域 (center): 任务、设置、通知
//!
//! ## 系统任务 runner — 替代 sub2api Redis 队列的 PG 原生方案
//!
//! 参考 new-api store_system_task.go (DB lease + heartbeat) 与 sub2api 的
//! `FOR UPDATE SKIP LOCKED` 认领。**无 Redis 的多实例正确性靠 PG**:
//!
//! ```sql
//! -- 认领: 多实例安全, 谁抢到谁执行
//! UPDATE system_jobs SET state='running', leased_by=$node, lease_expires=now()+interval '5 min'
//! WHERE id = (
//!   SELECT id FROM system_jobs
//!   WHERE state='pending' OR (state='running' AND lease_expires < now())
//!   ORDER BY id FOR UPDATE SKIP LOCKED LIMIT 1
//! );
//! -- 心跳续租; 崩溃实例的租约过期 → 任务自动回到可认领态
//! ```
//!
//! 任务类型 (首批, 对齐 new-api schedule_tasks.go):
//! channel 探活 / usage 清理 / 订阅周期重置 / observe 回填。
//! TODO(#800): system_jobs 表 DDL (type/payload JSONB/state/leased_by/
//! lease_expires/heartbeat_at/result/progress) 进 store/migrations。
//!
//! ## 选主 (singleton 周期任务)
//!
//! sub2api 用 Redis SetNX; 我们用 PG advisory lock (`pg_try_advisory_lock`),
//! 天然随会话释放, 崩溃自动掉主 — 与"没有证据不做全局 lease"原则不冲突:
//! 这只影响**谁跑周期任务**, 不影响数据正确性。
//!
//! ## 设置 (options)
//!
//! one-api 的 Option{key,value} 单表 + new-api 的类型化校验合并:
//! 值域校验放注册表 (每种 option 一个 validator), 变更后发 revision 事件
//! (经 sync 域下发 edge)。TODO(#801): option key 清单 + validator 注册表。
//!
//! ## 通知
//!
//! new-api notify_user (email/webhook HMAC/Bark/Gotify, SSRF 控制) +
//! 渠道异常告警。TODO(#802): 渠道熔断通知属 observe/dispatch 的健康事件
//! 下游 — 先占位, 邮件渠道二期。
