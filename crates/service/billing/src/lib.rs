//! # billing — 商业化域 (center)
//!
//! 管两类钱:
//! 1. **请求消耗** (wallet/subscription 双资金源) — 资金源选择与请求级
//!    计费会话属于 metering (热路径); 本 crate 只管**商业产品生命周期**;
//! 2. **充值/订阅/兑换** (运营面) — 订单、支付回调、订阅周期、兑换码、签到。
//!
//! ## 与 metering 的边界 (调查后定死, 防 drain)
//!
//! ```text
//! metering: prehold → settle (本地内存 + WAL, edge 侧)
//! billing : plan → order → webhook → subscription → 资金入账 (center, PG 权威)
//! ```
//! metering 结算后经 sync 上报 usage 事件 → center 收敛 → 扣减资金。
//! billing 不知道单个请求的存在。
//!
//! ## 幂等纪律 (sub2api 的教训)
//!
//! 所有资金写操作必须挂 idempotency_records (scope, key_hash, fingerprint,
//! state 机: processing/succeeded/failed_retryable), 支付回调尤其如此:
//! 同一通知重放 = 成功回执; 不同内容同 key = 冲突; 过期 = CAS 重领。

/// 支付渠道抽象 — epay / stripe / creem / waffo 各一实现。
///
/// 参考 new-api topup_{stripe,creem,waffo,waffo_pancake}.go +
/// payment_webhook_availability.go (回调可用性开关)。
/// TODO(#600): trait 形状 — create_order(返回支付 URL) + verify_webhook(验签
/// + 幂等) + refund? 二期再定 refund。adapter 注册进 apps/console。
pub trait PaymentProvider: Send + Sync {
    /// 渠道标识 ("epay" | "stripe" | ...)。
    fn id(&self) -> &'static str;
    /// 该渠道当前是否可用 (配置完整性/合规开关, 对齐 payment_compliance.go)。
    fn available(&self) -> bool;
}

// 订单生命周期状态机。
// TODO(#601): 状态枚举进 contract::records (PaymentOrderRecord) — pending/
// paid/failed/refunded + 终态不可变。避免现在编字段, 接 console 支付页时定。
//
// 订阅周期 (对齐 new-api reset_subscription_cycle.go + sub2api user_subscription):
// - 购买成功 → user_subscriptions 行 (窗口: daily/weekly/monthly/custom);
// - 到期降级要尊重其它活跃升级订阅 (不叠加降级);
// - 周期重置 = 幂等的 pre-consume 记录 + 定时任务 (跑在 ops::job runner 上)。
//
// 兑换码 (对齐 one-api redemptions + new-api store_redemption):
// - 批量生成 (batch 生成 N 条唯一 code), 单次核销 (行锁/CAS), 事务内入账;
// - TODO(#602): 与 promo_code (注册赠) 合并还是分表? sub2api 分了两张 —
//   我们先只做 redeem_codes, promo 二期。
//
// 签到 (对齐 do_checkin.go): 唯一约束 (user, date) + 随机额度区间 + 月历史查询。
// TODO(#603): 签到记录表 + 接口 — console 用户页接入时定字段。
