//! # metering — 计量 (热路径旁路)
//!
//! 三个子职责, 全部围绕一条原则: **转发路径上只做本地内存操作**。
//!
//! ```text
//! ① prehold  预扣: admission 通过后, 按预估 token 扣本地余额位
//! ② scan     流扫描: SSE 管道旁挂扫描器, 边透传边抓 usage / 累计字符
//! ③ settle   结算: 请求结束 → 生成 UsageEvent → 追加本地 WAL (Fjall)
//!                  └→ 差额回补 (预扣 > 实际) 在内存账本上完成, 不回写 DB
//! ```
//!
//! WAL 推送 center 由 service/sync 驱动; 本模块只管"产生正确的事件"。

use bytes::Bytes;
use contract::records::UsageEventRecord;

/// 预扣凭据 — admission 返回的 hold_id 即由本模块发放。
/// TODO(#330): hold_id 生成 — 节点内 u64 计数器足够 (只在本地有意义)。
pub struct Hold {
    pub id: u64,
    /// 预扣额度 (内部计费单位)。
    pub amount: i64,
}

/// 内存账本 trait: prehold/settle/release 三操作必须原子 (DashMap + per-user 锁)。
///
/// 崩溃语义: 预扣只存在内存, 崩溃即丢 — 可接受, 因为 UsageEvent 才是权威
/// 账单来源 (WAL), 余额以 center 收敛后的 used_quota 为准。
/// TODO(#331): 该取舍写进设计文档, 让"余额可能短暂偏高/偏低但事件不丢"成为明示决策。
pub trait Ledger: Send + Sync {
    fn prehold(&self, user_key: &str, estimated: i64) -> Result<Hold, Insufficient>;
    /// 实际用量落定, 返回应退回的差额 (负数 = 需补扣)。
    fn settle(&self, hold: &Hold, actual: i64) -> i64;
    /// 请求在预扣后、结算前失败 (连接中断等) → 全额释放。
    fn release(&self, hold: &Hold);
}

#[derive(Debug, thiserror::Error)]
#[error("insufficient balance: need {need}, have {have}")]
pub struct Insufficient {
    pub need: i64,
    pub have: i64,
}

/// 流式 token 扫描器 — 挂在 forward 的响应管道里。
///
/// 工作方式: 每收到一块 Bytes, `push()` 原样返回 (透传零拷贝), 内部增量解析:
/// - 上游给 usage 字段 (OpenAI stream_options / Claude message_delta) → 直接采信;
/// - 没有 usage 的流 → 按 BPE 近似估算 (TODO(#332): 估算器实现, tiktoken-rs 或字符比)。
/// 流结束时 `finish()` 产出结算所需的计数。
pub struct StreamScanner {
    // TODO(#333): 状态机 — 当前 SSE 事件缓冲、累计 prompt/completion/cache tokens。
}

impl StreamScanner {
    pub fn push(&mut self, chunk: &Bytes) {
        let _ = chunk; // 透传; 解析逻辑待实现
    }
    /// 流结束, 产出最终计数。prompt 侧由请求体预扫得出, 这里补 completion 侧。
    /// TODO(#334): 请求体 prompt token 预扫 (适配器转换后、发上游前做一次)。
    pub fn finish(self) -> TokenCounts {
        TokenCounts::default()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TokenCounts {
    pub prompt: u64,
    pub completion: u64,
    pub cached: u64,
}

/// 结算入口: 扫描结果 + 定价 → UsageEvent (写 WAL)。
///
/// 定价来源: catalog 快照的模型价格表。
/// TODO(#335): 定价表结构 — contract 加 PriceRecord? 还是 GroupRecord 内嵌?
///             对齐 mock::models::GroupPrice 的三列 (input/output/cache) 先定字段。
pub fn settle_event(
    _counts: TokenCounts,
    _hold: &Hold,
) -> UsageEventRecord {
    unimplemented!("TODO(#335): 定价表定型后实现")
}
