//! # billing — 商业化域 (center)
//!
//! 管两类钱:
//! 1. **请求消耗** (钱包/订阅双资金源) — 资金源选择与请求级计费会话属于
//!    metering (热路径); 本 crate 管**商业产品生命周期**;
//! 2. **充值/订阅/兑换** (运营面) — 订单、支付回调、订阅周期、兑换码。
//!
//! ## 与 metering 的边界 (调查后定死, 防 drain)
//!
//! ```text
//! metering: prehold → settle (本地内存 + WAL, edge 侧)
//! billing : plan → order → webhook → subscription → 资金入账 (center, PG 权威)
//! ```
//! metering 结算后经 sync 上报 usage → center 收敛 → 扣减资金。
//! billing 不知道单个请求的存在。
//!
//! ## 幂等纪律 (sub2api 教训)
//!
//! 所有资金写操作必须挂 idempotency_records; 支付回调尤其如此:
//! 同一通知重放 = 成功回执; 同 key 不同内容 = 冲突; 过期 = CAS 重领。
//!
//! ## 模块地图
//!
//! | 模块 | 职责 |
//! |------|------|
//! | [`orders`]       | 订单状态机 + 回调验证 |
//! | [`providers`]    | 支付渠道抽象 + 注册表 |
//! | [`subscriptions`]| 订阅分配/到期/周期重置 |
//! | [`redeem`]       | 兑换码生成/核销 |
//! | [`idempotency`]  | 幂等护栏 (通用) |

pub mod redeem;

pub use redeem::{ensure_table, router, RedeemAppState, RedeemService};
