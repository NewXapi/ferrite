# proto: shoes 客户端协议链基础设施（骨架）

从 shoes（MIT，https://github.com/cfal/shoes）移植。零额外进程的多协议出口。

本 PR 只搬基础设施；协议握手实现见后续 PR。

## 模块

- `async_stream.rs` — `AsyncStream` / `AsyncMessageStream` 流抽象
- `address.rs` — `Address` / `NetLocation` / `ResolvedLocation`
- `stream_reader.rs` — 握手期读缓冲
- `proxy_connector.rs` — `ProxyConnector` trait

## 验收

```bash
cargo test -p gateway-proxy
```

## 压测

Gateway 性能压测工具。测 gateway 本身（gate/dispatch/forward 各层延迟、吞吐、错误率），上游模型只是产生 200 响应。

```bash
# 1. 安装依赖
go install github.com/rakyll/hey@latest

# 2. 启动 gateway（如果还没跑）
./target/debug/gateway &

# 3. 跑压测
./bench/scripts/bench_gateway.sh baseline
```

### 目录结构

```
bench/
├── README.md           # 详细文档
├── data/
│   ├── prompts.jsonl   # 测试 prompts（短/中/长）
│   └── scenarios.yaml  # 场景定义 + 速率控制
├── scripts/
│   └── bench_gateway.sh
├── workflows/
│   └── bench.yml       # GitHub Actions
└── results/            # gitignore，原始结果 + 分析
```

### 场景

| 场景 | RPS | 并发 | 模型 |
|---|---|---|---|
| baseline | 2 | 5 | gpt-oss-20b |
| medium_load | 5 | 20 | gpt-5.6-sol |
| multi_model | 3 | 15 | 轮换 |
| streaming | 2 | 10 | gpt-oss-20b |
| stress | 10 | 50 | gpt-oss-20b（默认关闭） |

### 结果

结果存 `bench/results/<scenario>_<timestamp>/`：
- `hey_output.txt`：hey 原始输出
- `analysis.json`：延迟分布（p50/p95/p99）、吞吐、错误率
- `gateway_log_before/after.log`：gateway 日志快照

### 防封号

- 控制 RPS（不是跑满）
- 遇 429 立即退避
- 单模型 RPM 上限 60
- 多模型轮换分散压力

详细文档见 [bench/README.md](bench/README.md)。
