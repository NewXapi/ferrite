# `gateway-metering`

## 目录

```text
src/
├── lib.rs
├── estimate.rs
├── ledger.rs
├── pricing.rs
├── scanner.rs
└── settle.rs
```

## 要实现

- 配额预扣、结算和释放。
- 上游 usage 提取。
- token 估算。
- 模型定价。
- 流式用量扫描。
