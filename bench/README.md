# bench/

Gateway 压测工具。测 gateway 本身性能，上游模型只是产生 200 响应。

## 目录

```
bench/
├── data/
│   ├── prompts.jsonl      # 测试 prompts（短/中/长）
│   └── scenarios.yaml     # 场景定义 + 速率控制
├── scripts/
│   └── bench_gateway.sh   # 主压测脚本
├── workflows/
│   └── bench.yml          # GitHub Actions
└── results/               # gitignore，原始结果 + 分析
```

## 使用

```bash
# 列出场景
./bench/scripts/bench_gateway.sh

# 运行单个场景
./bench/scripts/bench_gateway.sh baseline

# 运行所有启用的场景
./bench/scripts/bench_gateway.sh all
```

## 场景

| 场景 | 描述 | RPS | 并发 |
|---|---|---|---|
| baseline | 低并发基线 | 2 | 5 |
| medium_load | 中等并发 | 5 | 20 |
| multi_model | 多模型轮换 | 3 | 15 |
| streaming | 流式请求 | 2 | 10 |
| stress | 压力测试（默认关闭） | 10 | 50 |

## 防封号

- 控制 RPS（不是跑满）
- 遇 429 立即退避
- 单模型 RPM 上限 60
- 多模型轮换分散压力

## 依赖

- `hey`：`go install github.com/rakyll/hey@latest`
- `jq`
- `python3`
