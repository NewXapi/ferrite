# `billing`

## 文件

- `src/lib.rs` — 公开订单和余额变更服务。
- `src/orders.rs` — 订单状态转换和支付确认。
- `src/idempotency.rs` — 保存请求 key 和首次响应。
- `src/providers.rs` — 支付提供方请求和回调验证。
- `src/redeem.rs` — 兑换码生成和核销。
- `src/subscriptions.rs` — 订阅周期与额度重置。

