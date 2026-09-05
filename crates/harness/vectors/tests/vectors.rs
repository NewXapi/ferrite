//! Integration tests for `harness-vectors` crate.
//!
//! Tests cover all modules: hash, chunk, index, recall.

use harness_vectors::{
    chunk::{DEFAULT_CHUNK_DELIMITERS, split_by_chunks, split_recursive},
    hash::string_hash,
    index::{QueryHit, VectorIndex, VectorItem, cosine_similarity},
    recall::{HashedMessage, build_query_text, hash_messages, retrieve_relevant},
};

#[test]
fn test_hash_consistency() {
    // Same input always produces same hash
    assert_eq!(string_hash("abc"), string_hash("abc"));
    assert_eq!(string_hash(""), string_hash(""));
    assert_eq!(string_hash("你好"), string_hash("你好"));
    assert_eq!(string_hash("🎉"), string_hash("🎉"));

    // Different inputs produce different hashes (with high probability)
    assert_ne!(string_hash("abc"), string_hash("abd"));
    assert_ne!(string_hash("a"), string_hash("b"));
    assert_ne!(string_hash("hello"), string_hash("world"));
}

#[test]
fn test_hash_non_ascii_stable() {
    // Non-ASCII should be stable across calls
    let h1 = string_hash("你好世界");
    let h2 = string_hash("你好世界");
    assert_eq!(h1, h2);

    let h3 = string_hash("🎉🎊🎈");
    let h4 = string_hash("🎉🎊🎈");
    assert_eq!(h3, h4);

    // Mixed scripts
    let h5 = string_hash("Hello 你好 🎉");
    let h6 = string_hash("Hello 你好 🎉");
    assert_eq!(h5, h6);
}

#[test]
fn test_split_recursive_basic() {
    let result = split_recursive("hello world", 5, &[" "]);
    assert_eq!(result, vec!["hello", "world"]);
}

#[test]
fn test_split_recursive_length_zero_returns_original() {
    let result = split_recursive("hello world", 0, &[" "]);
    assert_eq!(result, vec!["hello world"]);
}

#[test]
fn test_split_recursive_empty_delimiters_returns_original() {
    let result = split_recursive("hello world", 5, &[]);
    assert_eq!(result, vec!["hello world"]);
}

#[test]
fn test_split_recursive_no_split_falls_back() {
    // Delimiter "|" not found, falls back to " "
    let result = split_recursive("hello world", 5, &["|", " "]);
    assert_eq!(result, vec!["hello", "world"]);
}

#[test]
fn test_split_recursive_merge_short_chunks() {
    // ["ab", "cd"] with delim " " and length 5 -> "ab cd" (5 chars)
    let result = split_recursive("ab cd", 5, &[" "]);
    assert_eq!(result, vec!["ab cd"]);
}

#[test]
fn test_split_recursive_merge_boundary() {
    // "ab cd ef" with delim " " length 5 -> ["ab cd", "ef"]
    let result = split_recursive("ab cd ef", 5, &[" "]);
    assert_eq!(result, vec!["ab cd", "ef"]);
}

#[test]
fn test_split_recursive_long_part_recurses() {
    // Part longer than length recurses with next delimiter
    let result = split_recursive("helloworld", 5, &[" ", ""]);
    // No space, so falls back to "" (char split)
    // "hello", "world" each 5 chars
    assert_eq!(result, vec!["hello", "world"]);
}

#[test]
fn test_split_recursive_unicode_char_count() {
    // Unicode chars count as 1 each (not UTF-16 code units)
    let result = split_recursive("🎉🎊🎈", 2, &["", ""]);
    assert_eq!(result, vec!["🎉🎊", "🎈"]);
}

#[test]
fn test_split_recursive_empty_string() {
    let result = split_recursive("", 10, &[" "]);
    assert_eq!(result, vec![""]);
}

#[test]
fn test_split_recursive_single_char_delimiter() {
    let result = split_recursive("abc", 2, &[""]);
    assert_eq!(result, vec!["ab", "c"]);
}

#[test]
fn test_split_by_chunks_zero_returns_original() {
    let result = split_by_chunks("hello world", 0);
    assert_eq!(result, vec!["hello world"]);
}

#[test]
fn test_split_by_chunks_uses_default_delimiters() {
    let text = "para1\n\npara2\n\npara3";
    let result = split_by_chunks(text, 10);
    for chunk in &result {
        assert!(chunk.chars().count() <= 10, "Chunk too long: {:?}", chunk);
    }
}

#[test]
fn test_split_by_chunks_merge_paragraphs() {
    let text = "a\n\nb";
    let result = split_by_chunks(text, 10);
    assert_eq!(result, vec!["a\n\nb"]);
}

#[test]
fn test_default_chunk_delimiters() {
    assert_eq!(DEFAULT_CHUNK_DELIMITERS, &["\n\n", "\n", " ", ""]);
}

#[test]
fn test_vector_item_serialization() {
    let item = VectorItem {
        hash: 123,
        index: 0,
        text: "test".to_string(),
        vector: vec![1.0, 2.0, 3.0],
    };
    let json = serde_json::to_string(&item).unwrap();
    let deserialized: VectorItem = serde_json::from_str(&json).unwrap();
    assert_eq!(item.hash, deserialized.hash);
    assert_eq!(item.index, deserialized.index);
    assert_eq!(item.text, deserialized.text);
    assert_eq!(item.vector, deserialized.vector);
}

#[test]
fn test_vector_index_load_save_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.json");

    let mut idx = VectorIndex::default();
    idx.upsert(vec![VectorItem {
        hash: 1,
        index: 0,
        text: "test".to_string(),
        vector: vec![1.0, 0.0],
    }]);

    idx.save(&path).unwrap();
    let loaded = VectorIndex::load(&path).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.items()[0].hash, 1);
    assert_eq!(loaded.items()[0].text, "test");
    assert_eq!(loaded.items()[0].vector, vec![1.0, 0.0]);
}

#[test]
fn test_vector_index_load_nonexistent_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    let loaded = VectorIndex::load(&path).unwrap();
    assert!(loaded.is_empty());
}

#[test]
fn test_vector_index_upsert_deduplication() {
    let mut idx = VectorIndex::default();
    let items = vec![
        VectorItem {
            hash: 1,
            index: 0,
            text: "a".to_string(),
            vector: vec![1.0],
        },
        VectorItem {
            hash: 2,
            index: 1,
            text: "b".to_string(),
            vector: vec![2.0],
        },
        VectorItem {
            hash: 1,
            index: 2,
            text: "a duplicate".to_string(),
            vector: vec![3.0],
        },
    ];
    let added = idx.upsert(items);
    assert_eq!(added, 2);
    assert_eq!(idx.len(), 2);
    // First item with hash 1 should be kept
    assert_eq!(idx.items()[0].text, "a");
}

#[test]
fn test_vector_index_query_sorting() {
    let mut idx = VectorIndex::default();
    idx.upsert(vec![
        VectorItem {
            hash: 1,
            index: 0,
            text: "a".to_string(),
            vector: vec![1.0, 0.0],
        },
        VectorItem {
            hash: 2,
            index: 1,
            text: "b".to_string(),
            vector: vec![0.0, 1.0],
        },
        VectorItem {
            hash: 3,
            index: 2,
            text: "c".to_string(),
            vector: vec![1.0, 1.0],
        },
    ]);

    let hits = idx.query(&[1.0, 0.0], 10, None);
    assert_eq!(hits.len(), 3);
    // Most similar to [1,0] is [1,0] (hash 1), then [1,1] (hash 3), then [0,1] (hash 2)
    assert_eq!(hits[0].hash, 1);
    assert_eq!(hits[1].hash, 3);
    assert_eq!(hits[2].hash, 2);
}

#[test]
fn test_vector_index_query_top_k() {
    let mut idx = VectorIndex::default();
    idx.upsert(vec![
        VectorItem {
            hash: 1,
            index: 0,
            text: "a".to_string(),
            vector: vec![1.0, 0.0],
        },
        VectorItem {
            hash: 2,
            index: 1,
            text: "b".to_string(),
            vector: vec![0.0, 1.0],
        },
        VectorItem {
            hash: 3,
            index: 2,
            text: "c".to_string(),
            vector: vec![1.0, 1.0],
        },
    ]);

    let hits = idx.query(&[1.0, 0.0], 2, None);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].hash, 1);
    assert_eq!(hits[1].hash, 3);
}

#[test]
fn test_vector_index_query_threshold() {
    let mut idx = VectorIndex::default();
    idx.upsert(vec![
        VectorItem {
            hash: 1,
            index: 0,
            text: "a".to_string(),
            vector: vec![1.0, 0.0],
        },
        VectorItem {
            hash: 2,
            index: 1,
            text: "b".to_string(),
            vector: vec![0.0, 1.0],
        },
    ]);

    // Threshold 0.9 - only exact match (cosine=1.0) should pass
    let hits = idx.query(&[1.0, 0.0], 10, Some(0.9));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].hash, 1);

    // Threshold 0.0 - both should pass
    let hits = idx.query(&[1.0, 0.0], 10, Some(0.0));
    assert_eq!(hits.len(), 2);
}

#[test]
fn test_vector_index_query_zero_vector_returns_empty() {
    let mut idx = VectorIndex::default();
    idx.upsert(vec![VectorItem {
        hash: 1,
        index: 0,
        text: "a".to_string(),
        vector: vec![1.0, 0.0],
    }]);

    // Zero query vector returns empty
    let hits = idx.query(&[0.0, 0.0], 10, None);
    assert!(hits.is_empty());
}

#[test]
fn test_vector_index_query_dimension_mismatch_skipped() {
    let mut idx = VectorIndex::default();
    idx.upsert(vec![VectorItem {
        hash: 1,
        index: 0,
        text: "a".to_string(),
        vector: vec![1.0, 0.0],
    }]);

    // Query with different dimension - skipped
    let hits = idx.query(&[1.0, 0.0, 0.0], 10, None);
    assert!(hits.is_empty());
}

#[test]
fn test_vector_index_query_nan_inf_handling() {
    let mut idx = VectorIndex::default();
    idx.upsert(vec![VectorItem {
        hash: 1,
        index: 0,
        text: "a".to_string(),
        vector: vec![f32::NAN, 0.0],
    }]);

    let hits = idx.query(&[1.0, 0.0], 10, None);
    assert!(hits.is_empty());
}

#[test]
fn test_cosine_similarity_identical() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![1.0, 2.0, 3.0];
    assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_orthogonal() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_zero_vector() {
    let a = vec![0.0, 0.0];
    let b = vec![1.0, 2.0];
    assert_eq!(cosine_similarity(&a, &b), 0.0);
    assert_eq!(cosine_similarity(&b, &a), 0.0);
}

#[test]
fn test_cosine_similarity_empty_vectors() {
    assert_eq!(cosine_similarity(&[], &[]), 0.0);
    assert_eq!(cosine_similarity(&[1.0], &[]), 0.0);
}

#[test]
fn test_hash_messages_takes_last_n() {
    let messages = vec!["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];
    let hashed = hash_messages(&messages, 2);
    assert_eq!(hashed.len(), 2);
    assert_eq!(hashed[0].text, "msg2");
    assert_eq!(hashed[1].text, "msg3");
}

#[test]
fn test_hash_messages_filters_empty_and_whitespace() {
    let messages = vec![
        "msg1".to_string(),
        "".to_string(),
        "   ".to_string(),
        "msg2".to_string(),
    ];
    let hashed = hash_messages(&messages, 10);
    assert_eq!(hashed.len(), 2);
    assert_eq!(hashed[0].text, "msg1");
    assert_eq!(hashed[1].text, "msg2");
}

#[test]
fn test_hash_messages_query_zero_returns_empty() {
    let messages = vec!["a".to_string(), "b".to_string()];
    let hashed = hash_messages(&messages, 0);
    assert!(hashed.is_empty());
}

#[test]
fn test_hash_messages_empty_input() {
    let hashed = hash_messages(&[], 5);
    assert!(hashed.is_empty());
}

#[test]
fn test_hash_messages_preserves_original_relative_order() {
    let messages = vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
    ];
    let hashed = hash_messages(&messages, 3);
    assert_eq!(hashed.len(), 3);
    assert_eq!(hashed[0].text, "b");
    assert_eq!(hashed[1].text, "c");
    assert_eq!(hashed[2].text, "d");
}

#[test]
fn test_build_query_text_joins_with_newline() {
    let hashed = vec![
        HashedMessage {
            text: "hello".to_string(),
            hash: 1,
            index: 0,
        },
        HashedMessage {
            text: "world".to_string(),
            hash: 2,
            index: 1,
        },
    ];
    let query = build_query_text(&hashed);
    assert_eq!(query, "hello\nworld");
}

#[test]
fn test_build_query_text_empty() {
    let hashed = vec![];
    let query = build_query_text(&hashed);
    assert_eq!(query, "");
}

#[test]
fn test_build_query_text_trims() {
    let hashed = vec![HashedMessage {
        text: "  hello  ".to_string(),
        hash: 1,
        index: 0,
    }];
    let query = build_query_text(&hashed);
    assert_eq!(query, "hello");
}

#[test]
fn test_retrieve_relevant_excludes_protected_tail() {
    let messages = vec![
        "msg1".to_string(),
        "msg2".to_string(),
        "msg3".to_string(),
        "msg4".to_string(), // protected tail (protect_tail=1)
    ];
    let hits = vec![
        QueryHit {
            hash: string_hash("msg2"),
            index: 1,
            text: "msg2".to_string(),
            similarity: 0.9,
        },
        QueryHit {
            hash: string_hash("msg1"),
            index: 0,
            text: "msg1".to_string(),
            similarity: 0.8,
        },
    ];
    let relevant = retrieve_relevant(&messages, &hits, 1);
    assert_eq!(relevant.len(), 2);
    assert_eq!(relevant[0], "msg2");
    assert_eq!(relevant[1], "msg1");
}

#[test]
fn test_retrieve_relevant_protect_tail_excludes_all() {
    let messages = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let hits = vec![QueryHit {
        hash: string_hash("c"),
        index: 2,
        text: "c".to_string(),
        similarity: 0.9,
    }];
    let relevant = retrieve_relevant(&messages, &hits, 1);
    assert!(relevant.is_empty());
}

#[test]
fn test_retrieve_relevant_deduplicates_by_hash() {
    let messages = vec!["a".to_string(), "b".to_string()];
    let hash_a = string_hash("a");
    let hash_b = string_hash("b");
    // Create hits with DIFFERENT hashes (simulating two different messages)
    let hits = vec![
        QueryHit {
            hash: hash_a,
            index: 0,
            text: "a".to_string(),
            similarity: 0.9,
        },
        QueryHit {
            hash: hash_b,
            index: 1,
            text: "b".to_string(),
            similarity: 0.8,
        }, // different hash
    ];
    let relevant = retrieve_relevant(&messages, &hits, 0);
    // Both messages should be returned since they have different hashes
    assert_eq!(relevant.len(), 2);
    // Order should be by similarity descending
    assert_eq!(relevant[0], "a");
    assert_eq!(relevant[1], "b");
}

#[test]
fn test_retrieve_relevant_sorts_by_similarity_descending() {
    let messages = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let hash_a = string_hash("a");
    let hash_b = string_hash("b");
    let hash_c = string_hash("c");

    let hits = vec![
        QueryHit {
            hash: hash_b,
            index: 1,
            text: "b".to_string(),
            similarity: 0.7,
        },
        QueryHit {
            hash: hash_a,
            index: 0,
            text: "a".to_string(),
            similarity: 0.9,
        },
        QueryHit {
            hash: hash_c,
            index: 2,
            text: "c".to_string(),
            similarity: 0.5,
        },
    ];
    let relevant = retrieve_relevant(&messages, &hits, 0);
    assert_eq!(relevant, vec!["a", "b", "c"]);
}

#[test]
fn test_retrieve_relevant_empty_inputs() {
    assert!(retrieve_relevant(&[], &[], 0).is_empty());
    assert!(retrieve_relevant(&["a".to_string()], &[], 0).is_empty());
}

#[test]
fn test_retrieve_relevant_full_protection() {
    let messages = vec!["a".to_string(), "b".to_string()];
    let hits = vec![QueryHit {
        hash: string_hash("a"),
        index: 0,
        text: "a".to_string(),
        similarity: 0.9,
    }];
    let relevant = retrieve_relevant(&messages, &hits, 5);
    assert!(relevant.is_empty());
}
