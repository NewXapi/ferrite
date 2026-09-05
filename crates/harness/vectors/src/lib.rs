//! `harness-vectors` — 向量检索记忆（ST `vectors` 扩展 + `endpoints/vectors.js`
//! 的 Ferrite 移植）。纯算法层：分块 / hash / 余弦相似度 / 本地 JSON 索引；
//! embedding 调用与文件 IO 编排归调用方。
