# `gateway-metering`

## 文件

- `src/lib.rs` — 公开预扣和结算入口。
- `src/ledger.rs` — 保存用户额度、hold 和释放状态。
- `src/pricing.rs` — 保存模型输入、输出和缓存价格。
- `src/estimate.rs` — 按厂商规则估算 token。
- `src/scanner.rs` — 从流式响应读取 usage。
- `src/settle.rs` — 按 usage 和价格结算或释放 hold。

