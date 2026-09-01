//! 快照组装与原子替换。
//!
//! snapshot 是 (domain → 不可变快照对象) 的载荷:
//! - Catalog → dispatch 用的路由索引 (group, model → 候选);
//! - Identity → admission 用的 TokenSnapshot (hash → token)。
//!
//! 替换不变量 (对齐 new-api apps/gateway/src/config/snapshot.rs):
//! 1. 单调推进 — 新快照 revision 必须 ≥ 旧值, 否则拒绝 (乱序防护);
//! 2. 旧 Arc 保留 — 在途请求继续用旧快照直到自然结束;
//! 3. 原子指针切换 — 无"半新半旧"窗口。
//!
//! TODO(#432): snapshot 解码 (serde_json::Value → 强类型) + 校验 (checksum);
//! 与 apps/gateway 现有 config/snapshot.rs 的合并策略 (它已是实现, 迁移到本 crate)。

use contract::mutations::SnapshotEnvelope;

/// 从整包快照载荷解码某域的记录集。
/// TODO(#432): 按 domain 分发到具体 Record 类型; 解码失败 → SyncError。
pub fn decode(_env: &SnapshotEnvelope) -> Result<Vec<serde_json::Value>, super::SyncError> {
    Ok(Vec::new())
}
