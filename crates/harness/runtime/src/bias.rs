//! Logit bias 组装 helper（照搬 SillyTavern `logit-bias.js` 的 word→bias 映射语义）。
//!
//! OpenAI 兼容 provider 接受 `logit_bias` 字段：`token_id 字符串 → 偏置值(-100..100)`。
//! 本模块把「词列表 + 统一偏置」展开为该 map；词→token_id 的编码由调用方提供
//! （harness-tokenizer 已合并进上游 main，可直接 `TokenizerEngine::encode`）。

use std::collections::BTreeMap;

/// 把一组词按统一偏置展开为 `logit_bias` map。
///
/// 每个词先由 `encode` 编码为 token_id 序列（十进制字符串），每个 id 都应用
/// 同样的 `bias`。若一词编出多个 token，全部写入——与 ST 行为一致。
///
/// `bias` 会被 clamp 到 `[-100, 100]`（OpenAI 约束）。
pub fn apply_logit_bias<S, F>(words: &[S], bias: i32, encode: F) -> BTreeMap<String, i32>
where
    S: AsRef<str>,
    F: Fn(&str) -> Option<Vec<u32>>,
{
    let bias = bias.clamp(-100, 100);
    let mut map = BTreeMap::new();
    for word in words {
        if let Some(ids) = encode(word.as_ref()) {
            for id in ids {
                map.insert(id.to_string(), bias);
            }
        }
    }
    map
}
