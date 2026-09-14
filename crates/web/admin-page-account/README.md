# `page-account`

## 文件

- `src/lib.rs` — 导出 KeysPanel、UsageLogsPanel、RewardsPanel。
- `src/api.rs` — 账户页数据来源:`*_api` 真实调用 (走 `client::ApiClient`),`fetch_*` 仅存无端点区块的 mock。
- `src/keys.rs` — 用户 API Key 创建、列举、删除和状态切换。
- `src/usage_logs.rs` — 用量日志筛选与分页。
- `src/rewards.rs` — 奖励面板:钱包 / 拉人统计 / 兑换码 / 充值开单。
- `tests/rewards_wire.rs` — RewardsPanel 所接 billing 端点的 wire 契约对账。

## 奖励面板接线状态 (#179/#187)

真实端点:钱包 `GET /api/user/wallet`、拉人统计 `GET /api/affiliate/overview`、
兑换码 `POST /api/user/topup` (核销入账后刷新钱包)、充值开单
`POST /api/user/topup/orders` (仅建 pending 单,支付 provider 占位,admin settle 后入账)。

保持 mock 的区块 (无后端列表端点,UI 内有「演示数据」标注):充值记录、邀请链接、被邀人列表。

交互元素 `data-testid`:`wallet-available`、`wallet-balance-<code>`、`wallet-empty`、
`wallet-skeleton`、`wallet-error`、`topup-currency`、`topup-amount`、`topup-order-submit`、
`topup-order-result`、`topup-order-error`、`topup-code`、`topup-submit`、`topup-result`、
`topup-error`、`affiliate-invite-count`、`affiliate-total-reward`、`affiliate-skeleton`、
`affiliate-error`、`invite-link`、`invite-copy`、`invitee-count`。容器带 `role`+`aria-label`
(钱包 / 拉人统计 region,兑换码充值 / 充值开单 group)。`specs/ui/` 本地契约目录不存在,未落 yaml。
