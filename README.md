# Ferrite

多模型 AI 网关。把多个上游供应商收敛成一个本地端点，agent 只需改一个环境变量。

```bash
gateway --config ~/.config/ferrite/config.toml
export OPENAI_BASE_URL=http://127.0.0.1:8080/v1
```

## 现状

骨架阶段。workspace 结构、crate 边界、依赖纪律已就位；业务逻辑待实现。

- 构建：`cargo build` → `target/debug/gateway`
- 纯度守卫：`./scripts/check-purity.sh`
- 设计文档：`docs/`

## 设计

- `docs/08-mvp.md` — MVP 的功能模块、数据流动、目录结构、性能取舍
- `docs/09-roadmap.md` — 路线图：单应用能力 → 应用交互 → 集群增强
- `docs/01-07` — 现有 Go 实现的调查，仅作参考

## 结构

```
apps/cli/     单二进制 gateway：argv 解析 + 装配 + 信号处理，不含业务逻辑
crates/       9 个库 crate，依赖单向无环
docs/         设计文档
scripts/      纯度守卫
```

crate 分层：

- 叶子（不依赖任何内部 crate）：`config`、`protocol`、`usage`、`observe`
- 中层：`provider`、`router`、`stream`、`upstream`
- 汇聚：`orchestrator` —— 唯一同时看到 router / protocol / provider / upstream / stream
  的地方，重试循环归它

## 依赖纪律

`protocol` 与 `provider` 是纯函数：输入请求出请求，输入响应出 usage。
它们的 `Cargo.toml` 不允许出现 `reqwest` / `axum` / `tokio` / `sqlx` / `redis`。
`router` 只回答「该试哪些渠道」，也不该有 HTTP 客户端。

这条靠 `scripts/check-purity.sh` 在 CI 里守，不靠人记。

## 许可

MIT
