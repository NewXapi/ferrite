# benches/

Gateway 压测工具。测 gateway 本身性能（gate/dispatch/forward 各层延迟、吞吐、错误率），上游模型只是产生 200 响应。

## 目录

```
benches/
├── README.md           # 本文件
├── data/
│   ├── prompts.jsonl   # 测试 prompts（短/中/长）
│   └── scenarios.yaml  # 场景定义 + 速率控制
├── scripts/
│   └── bench_gateway.sh
├── workflows/
│   └── bench.yml       # GitHub Actions
└── results/            # gitignore，原始结果 + 分析
```

## 使用

```bash
# 列出场景
./benches/scripts/bench_gateway.sh

# 运行单个场景
./benches/scripts/bench_gateway.sh baseline

# 运行所有启用的场景
./benches/scripts/bench_gateway.sh all
```

## 场景

| 场景 | 描述 | RPS | 并发 | 模型 |
|---|---|---|---|---|
| baseline | 低并发基线 | 2 | 5 | gpt-oss-20b |
| medium_load | 中等并发 | 5 | 20 | gpt-5.6-sol |
| multi_model | 多模型轮换 | 3 | 15 | 轮换 |
| streaming | 流式请求 | 2 | 10 | gpt-oss-20b |
| stress | 压力测试（默认关闭） | 10 | 50 | gpt-oss-20b |

## 结果

结果存 `benches/results/<scenario>_<timestamp>/`：
- `hey_output.txt`：hey 原始输出
- `analysis.json`：延迟分布（p50/p95/p99）、吞吐、错误率
- `gateway_log_before/after.log`：gateway 日志快照

## 防封号

- 控制 RPS（不是跑满）
- 遇 429 立即退避
- 单模型 RPM 上限 60
- 多模型轮换分散压力

## 依赖

- `hey`：`go install github.com/rakyll/hey@latest`
- `jq`
- `python3`

## 模型选择

从 wildtoken 渠道选免费、RPM 宽松的模型：

| 渠道 | 模型 | 特点 |
|---|---|---|
| 52mxw | gpt-oss-20b | 免费、快 |
| 52mxw | nemotron-3-ultra | 免费 |
| 星剑雅 | gpt-5.6-sol | RPM 相对宽松 |

## 业界参考

- [hyperium/hyper](https://github.com/hyperium/hyper) — `cargo bench` + `github-action-benchmark` + gh-pages 存结果
- [mlpack/benchmarks](https://github.com/mlpack/benchmarks) — 独立 repo，YAML 配置驱动，SQLite 存历史
- [tokio-rs/tokio](https://github.com/tokio-rs/tokio) — `benches/` + criterion.rs，CI 回归检测
