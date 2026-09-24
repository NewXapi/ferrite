# security

内容扫描：词库、输入替换、跨 chunk 扫描与审核结果。

## 职责

对请求/响应内容做敏感词扫描与替换。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/scan.rs` | 跨 chunk 扫描引擎（处理词被 SSE 分帧切断的情况） |
| `src/wordlist.rs` | 词库 |
| `src/lib.rs` | crate 导出面 |

## 当前状态

**零消费方**：`apps/api` 与 `apps/gateway` 均未依赖本 crate。改动它不会被任何生产
链路感知，验收只能靠自身 `tests/`。

## 验收

```bash
cargo check -p gateway-security
cargo test -p gateway-security             # CI
```
