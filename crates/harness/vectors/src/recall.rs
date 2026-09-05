//! Message recall and relevance retrieval (ported from ST `endpoints/vectors.js`).
//!
//! This module implements the ST recall pipeline:
//! 1. `hash_messages` - hash messages and take the last `query` non-empty ones (ST `getQueryText`)
//! 2. `build_query_text` - join hashed messages with "\n" and trim (ST `getQueryText`)
//! 3. `retrieve_relevant` - exclude protected tail, sort by similarity, deduplicate (ST `rearrange`)

use crate::hash::string_hash;
use crate::index::QueryHit;

/// A message with its hash and original index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashedMessage {
    /// The original message text
    pub text: String,
    /// Hash of the text (from `string_hash`)
    pub hash: u64,
    /// Original position in the message list
    pub index: usize,
}

/// Hashes messages and returns the last `query` non-empty hashed messages.
///
/// Corresponds to ST `getQueryText` logic:
/// 1. Hash each message
/// 2. Filter out empty messages (after trimming)
/// 3. Reverse the list
/// 4. Take the first `query` items
/// 5. Return in original order (reversed again)
pub fn hash_messages(messages: &[String], query: usize) -> Vec<HashedMessage> {
    // 单次遍历取末尾 query 条非空消息（等价 ST：filter 非空 → reverse → take(query)），
    // index 用**原始消息下标**——retrieve_relevant 的 protect_tail 以原始位置计算。
    let start = messages.len().saturating_sub(query);
    messages[start..]
        .iter()
        .enumerate()
        .filter_map(|(offset, text)| {
            if text.trim().is_empty() {
                None
            } else {
                Some(HashedMessage {
                    text: text.clone(),
                    hash: string_hash(text),
                    index: start + offset,
                })
            }
        })
        .collect()
}

/// Builds a query text by joining hashed messages with newline and trimming.
///
/// Corresponds to ST `getQueryText` joining logic.
pub fn build_query_text(hashed: &[HashedMessage]) -> String {
    hashed
        .iter()
        .map(|hm| hm.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Retrieves relevant messages based on vector search hits.
///
/// Corresponds to ST `rearrange` logic:
/// 1. Exclude the last `protect_tail` messages (protected from retrieval)
/// 2. Sort hits by similarity descending
/// 3. Deduplicate by hash (keep first/highest similarity)
/// 4. Return the text of matched messages in relevance order
pub fn retrieve_relevant(
    messages: &[String],
    hits: &[QueryHit],
    protect_tail: usize,
) -> Vec<String> {
    // 保护尾条：以原始消息下标计（与 hash_messages 的 index 对齐，空消息也占位）。
    let protected_start = messages.len().saturating_sub(protect_tail);
    // 去重键 = (hash, text) 复合——hash 碰撞不丢内容（ocr bug·high）。
    let mut seen: std::collections::HashSet<(u64, &str)> = std::collections::HashSet::new();
    let mut picked: Vec<(f32, String)> = Vec::new();
    for hit in hits {
        let Some(text) = messages.get(hit.index) else {
            continue;
        };
        if hit.index >= protected_start {
            continue;
        }
        if seen.insert((hit.hash, text.as_str())) {
            picked.push((hit.similarity, text.clone()));
        }
    }
    // ST rearrange：按相关度降序排列后注入
    picked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    picked.into_iter().map(|(_, text)| text).collect()
}
