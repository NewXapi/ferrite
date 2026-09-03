# `gateway-security`

## 文件

- `src/lib.rs` — 公开内容安全 stage。
- `src/wordlist.rs` — 加载和管理敏感词表。
- `src/aho_corasick.rs` — 构建字节级关键词扫描器。
- `src/sanitize.rs` — 替换命中输入或输出片段。
- `src/ctx_tail.rs` — 保留流 chunk 尾部并匹配跨 chunk 关键词。
- `src/moderation.rs` — 定义第三方审核请求与响应。
- `src/stage.rs` — 在 pipeline 流中执行扫描和替换。

