# tokenizer

真实 BPE tokenizer（替换早先的字符数估算）。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/engine.rs` | 分词引擎 |
| `src/registry.rs` | tokenizer 注册表（按模型名查） |
| `src/error.rs` | 错误类型 |
| `src/lib.rs` | crate 导出面 |

## 当前状态

**零消费方**：`harness-runtime/src/bias.rs` 的 encode 由调用方注入，仓库里没有 crate
依赖本 crate。验收只能靠自身 `tests/`。

## 验收

```bash
cargo check -p harness-tokenizer
cargo test -p harness-tokenizer            # CI
```
