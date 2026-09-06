# `gateway-security`

配置词表过滤：对模型请求上下文（输入）与响应（输出，含流式）扫描替换命中词。

## 文件

- `src/lib.rs` — 导出 `WordFilter` / `StreamFilter` / `FilterConfig`。
- `src/wordlist.rs` — `FilterConfig`：词表、替换文本、请求/响应开关。
- `src/scan.rs` — `WordFilter`（一次性文本）+ `StreamFilter`（跨 chunk 流式）。

## 配置

```toml
[security]
words = ["sensitive", "classified"]
replacement = "***"
filter_request = true
filter_response = true
```

`words` 为空即不过滤，`WordFilter::is_empty()` 让调用方整条跳过。

## 接线（后续 PR，属 `forward` 域）

- 请求侧：`forward::stage::ForwardStage::handle` 读 body 后过 `WordFilter::filter`。
- 响应侧：`forward::stream::pipe_chunk` 逐 chunk 过 `StreamFilter::push`，流结束调 `flush`。

## 为什么不手写 AC 自动机

`aho-corasick` 是 Rust regex 引擎用的实现，久经考验且有 SIMD 优化。
手写 `[256]i32` 转移表是重造，且更容易在跨 chunk 边界上出错。

## 验收

```bash
cargo test -p gateway-security
```
