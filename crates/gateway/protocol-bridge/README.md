# protocol-bridge

上游协议编解码 + provider-neutral IR。

## 职责

把 pipeline 上下文翻译成各上游 API 的请求格式，再把上游响应（含 SSE）翻译回来。
`ir` 是中间的 provider 中立表示；各 `*.rs` 是具体协议的 codec；`error_mapping` 把
上游错误归一到 ferrite 的 `contract` 错误词表。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/ir.rs` | provider-neutral 中间表示（请求/响应/错误统一形状） |
| `src/openai.rs` | OpenAI Chat Completions codec |
| `src/responses.rs` | OpenAI Responses API codec |
| `src/claude.rs` | Anthropic Messages codec |
| `src/gemini.rs` | Gemini codec |
| `src/sse.rs` | SSE 分帧与增量解析 |
| `src/format_codec.rs` | 格式编解码组合入口 |
| `src/error_mapping.rs` | 上游错误 → `contract` 错误 |
| `src/adaptor.rs` | pipeline 上下文 ↔ codec 输入输出适配 |
| `src/stage.rs` | 作为 Stage 挂进 pipeline |
| `src/lib.rs` | crate 导出面 |

## 依赖

只依赖 `contract`。

## 验收

```bash
cargo check -p gateway-protocol-bridge
cargo test -p gateway-protocol-bridge      # CI（ir / error_mapping / format_codec / sse_scanner）
```
