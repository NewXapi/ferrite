# harness-vectors

向量检索记忆模块 —— ST `vectors` 扩展 + `endpoints/vectors.js` 的 Ferrite 移植。

纯算法层实现：**无 tokio / 无 reqwest / 无异步运行时**，完全 `wasm32-unknown-unknown` 兼容。

---

## 模块对照表

| 模块 | 文件 | ST 源码对应 | 功能 |
|------|------|-------------|------|
| `hash` | `src/hash.rs` | `utils.js:522` `getStringHash` | cyrb53 变体字符串哈希 (u64) |
| `chunk` | `src/chunk.rs` | `utils.js:1157` `splitRecursive` | 递归分块、合并短块 |
| `index` | `src/index.rs` | `endpoints/vectors.js` | 本地 JSON 向量索引：load/save/upsert/query |
| `recall` | `src/recall.rs` | `endpoints/vectors.js` `getQueryText` / `rearrange` | 消息哈希、查询文本构建、相关性检索 |

---

## 快速开始

```toml
[dependencies]
harness-vectors = { path = "crates/harness/vectors" }
```

```rust
use harness_vectors::{string_hash, split_by_chunks, VectorIndex, VectorItem, hash_messages, build_query_text, retrieve_relevant};

// 1. Hash 文本
let h = string_hash("hello world");

// 2. 分块
let chunks = split_by_chunks("para1\n\npara2\n\npara3", 100);

// 3. 向量索引
let mut idx = VectorIndex::default();
idx.upsert(vec![VectorItem { hash: h, index: 0, text: "hello".into(), vector: vec![0.1; 384] }]);
idx.save("index.json")?;

// 4. 召回流程
let messages = vec!["msg1".into(), "msg2".into(), "msg3".into()];
let hashed = hash_messages(&messages, 2);
let query_text = build_query_text(&hashed);
// => 使用 embedding 模型对 query_text 编码得到 query_vector
// let hits = idx.query(&query_vector, 5, Some(0.7));
// let relevant = retrieve_relevant(&messages, &hits, 1); // protect_tail=1
```

---

## 核心 API

### hash.rs

```rust
pub fn string_hash(text: &str) -> u64
```

- 入口：ST `utils.js:522` `getStringHash` (cyrb53 变体)
- 种子：`h1 = 0xdeadbeef`, `h2 = 0x41c6ce57`
- 逐字符：`h1 = (h1 ^ ch).wrapping_mul(2654435761)` (i32 wrapping mul，等价 JS `Math.imul`)
- 收尾：`h1 = imul(h1 ^ (h1 >>> 16), 2246822507) ^ imul(h2 ^ (h2 >>> 13), 3266489909)`，`h2` 同理
- 结果：`(2097151 & h2) << 32 | (h1 as u32)`
- **注意**：JS 逐 `charCodeAt` (UTF-16 code unit)，Rust 逐 `char` (Unicode scalar value)，非 BMP 字符边界可能差 1。

---

### chunk.rs

```rust
pub const DEFAULT_CHUNK_DELIMITERS: &[&str] = &["\n\n", "\n", " ", ""];
pub fn split_recursive(input: &str, length: usize, delimiters: &[&str]) -> Vec<String>
pub fn split_by_chunks(text: &str, chunk_size: usize) -> Vec<String>
```

- 入口：ST `utils.js:1157` `splitRecursive`
- 语义：
  - `length <= 0` 或 `delimiters` 空 → 返回原串单元素
  - 按首分隔符 `split`；无分隔 → 递归剩余分隔符
  - 超长 → 递归 `delimiters[1..]`
  - 短块合并：`current + delim + next` 长度 ≤ `length` 则合并
- `split_by_chunks`：`chunk_size <= 0` 返回整段；否则用 `DEFAULT_CHUNK_DELIMITERS`

**UTF-8/Unicode 差异**：JS 用 UTF-16 code unit 计数，Rust 用 `char` 计数。ASCII/BMP 完全一致；Emoji 等增补字符边界可能差 1。

---

### index.rs

```rust
pub struct VectorItem { pub hash: u64, pub index: usize, pub text: String, pub vector: Vec<f32> }
pub struct VectorIndex { items: Vec<VectorItem> }
pub struct QueryHit { pub hash: u64, pub index: usize, pub text: String, pub similarity: f32 }

impl VectorIndex {
    pub fn load(path: impl AsRef<Path>) -> Result<Self>
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()>
    pub fn upsert(&mut self, items: Vec<VectorItem>) -> usize  // 去重，返回新增数
    pub fn query(&self, query_vector: &[f32], top_k: usize, threshold: Option<f32>) -> Vec<QueryHit>
    pub fn items(&self) -> &[VectorItem]
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
}
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32  // 零向量返回 0
```

- `load`：文件不存在 → 空索引；JSON 解析失败 → Err
- `save`：pretty JSON
- `upsert`：按 `hash` 去重，首次出现者保留
- `query`：余弦相似度降序；`NaN`/`Inf` 跳过；`threshold` 过滤；零向量 query 返回空
- 序列化：`serde_json`，`VectorItem` 完整序列化

---

### recall.rs

```rust
pub struct HashedMessage { pub text: String, pub hash: u64, pub index: usize }

pub fn hash_messages(messages: &[String], query: usize) -> Vec<HashedMessage>
pub fn build_query_text(hashed: &[HashedMessage]) -> String
pub fn retrieve_relevant(messages: &[String], hits: &[QueryHit], protect_tail: usize) -> Vec<String>
```

对应 ST 流程：

1. **hash_messages** (`getQueryText` 前半)：哈希 → 过滤空串 → 反转 → `take(query)` → 再反转回原序
2. **build_query_text** (`getQueryText` 后半)：用 `\n` join 并 `trim`
3. **retrieve_relevant** (`rearrange`)：
   - 排除最后 `protect_tail` 条（受保护不被检索）
   - 按 `similarity` 降序
   - 按 `hash` 去重（保留首次/最高相似度）
   - 返回文本列表

---

## 测试

```bash
# 单元测试（含集成测试）
cpulimit -l 70 -i -- cargo test -p harness-vectors

# wasm32 编译检查（无需 wasm 运行时）
cpulimit -l 70 -i -- cargo check --target wasm32-unknown-unknown -p harness-vectors
```

---

## wasm-safe 保证

- 无 `tokio`、`reqwest`、异步运行时、线程、文件系统 I/O（除 `std::fs` 用于 `load/save`，仅在非 wasm 环境使用）
- 无 `std::time::Instant` 等 wasm 不稳定 API
- 纯计算逻辑：哈希、分块、线性代数、JSON (de)序列化
- 所有公共 API `#[cfg(not(target_arch = "wasm32"))]` 之外均可用

> **注意**：`VectorIndex::load/save` 使用 `std::fs`，在 wasm 环境需由调用方通过虚拟文件系统或 IndexedDB 实现存储抽象。核心算法（`query`/`upsert`/`cosine_similarity` 等）完全 wasm 兼容。

---

## 与 ST 的差异点

| 项 | ST (JS) | harness-vectors (Rust) |
|----|---------|------------------------|
| 哈希计数单位 | UTF-16 code unit | Unicode scalar value (`char`) |
| 分块计数单位 | UTF-16 code unit | Unicode scalar value (`char`) |
| 文件 IO | Node `fs.promises` | `std::fs` (同步) |
| 异步 | `async/await` | 同步 API |
| 余弦相似度 | 自定义实现 | 手写 `cosine_similarity` (零向量返回 0) |

---

## 许可证

MIT