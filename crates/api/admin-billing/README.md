# `billing`

商业化域：多货币钱包、兑换码、充值、拉人奖励。

## 文件

- `src/lib.rs` — 模块导出与域边界文档。
- `src/currency.rs` — `CurrencyService`：注册 seed（幂等）、折算综合可用值 `available_i64`、货币定义 admin CRUD；`BillingErr` 域错误。
- `src/wallet.rs` — `WalletService`：多货币扣费（rate 降序 + FOR UPDATE + 不足 clamp）、兑换码/充值/奖励三个入账入口（ON CONFLICT 叠加）、余额视图。
- `src/redeem.rs` — 兑换码生成与核销（CAS）；入账走 `WalletService::credit_redeem_in_tx`（`user_balances(FREE)`，不再写 `auth_users.quota`）。
- `src/topup.rs` — `TopupService`：开单 pending（provider 占位）+ admin 手工 settle（CAS pending→paid 幂等）。
- `src/affiliate.rs` — `AffiliateService`：邀请归属绑定（自邀请拒绝、一人一主）、invite 领奖（关系校验 + DB 级幂等，入账与审计行同事务）、真实统计（`user_overview`）；金额读 `options` 表 `site.affiliate_reward`，回落常量。

## 表

- `currency_defs(code PK, name, internal_rate, enabled)` — 货币注册表，seed `FREE`
- `user_balances(user_key, currency_code, amount)` — PK 复合，稀疏桶
- `billing_topups(key PK, user_key, currency, amount, state, provider)` — 充值订单
- `affiliate_links(invitee_key PK, inviter_key)` — 邀请归属，一人一主防重复绑定/领奖
- `affiliate_rewards(key PK, inviter_key, invitee_key, kind, amount)` — 奖励入账审计；局部唯一索引 `(invitee_key) WHERE kind='invite'` 做领奖幂等护栏
- 迁移：`db/migrations/0007_currency_wallet.sql`、`0008_topups.sql`、`0009_affiliate_links.sql`

## 边界

- 网关计量零改动：`UsageEventRecord.cost: i64` 是唯一货币无关输出，货币换算全在本域（`available_i64 = Σ amount × internal_rate`）
- settle 闭环已接（`PgSettleSink` → `deduct_by_cost`，#187）；邀请码字符串 → `inviter_key` 解析待 auth 域 `aff_code` 列（#188），`bind_inviter` 只收已解析 UUID
