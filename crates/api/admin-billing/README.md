# admin-billing

商业化域：钱包、兑换码、订单、订阅、联盟奖励、币种。

## 职责

多货币计费系统的后端主体。计费单位 `500000 = ¥1`（quota / used_quota / 兑换码面额
同量纲）。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/wallet.rs` | 钱包余额 |
| `src/redeem.rs` | 兑换码核销 |
| `src/topup.rs` | 充值 |
| `src/topup_epay.rs` | 易支付通道 |
| `src/subscriptions.rs` | 订阅 |
| `src/affiliate.rs` | 联盟奖励 |
| `src/currency.rs` | 币种与汇率换算 |
| `src/lib.rs` | crate 导出面 |

## 验收

```bash
cargo check -p admin-billing
cargo test -p admin-billing                # CI（10 个测试文件）
```
