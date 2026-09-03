# `gateway-security`

## 目录

```text
src/
├── lib.rs
├── aho_corasick.rs
├── ctx_tail.rs
├── moderation.rs
├── sanitize.rs
├── stage.rs
└── wordlist.rs
```

## 要实现

- 敏感词词库。
- Aho-Corasick 扫描。
- 输入输出替换。
- 跨 chunk 匹配。
- 审核接口。
- Pipeline SecurityStage。
