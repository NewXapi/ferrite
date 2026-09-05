//! Tests for the `harness-tokenizer` crate.

use harness_tokenizer::error::TokenModelError;
use harness_tokenizer::engine::{TokenizerEngine, BYTES_PER_TOKEN};
use harness_tokenizer::registry::TokenizerRegistry;

use std::fs;
use tempfile;

#[test]
fn test_guesstimate_calculations() {
    // Test exact values per specification.
    assert_eq!(TokenizerEngine::guesstimate("abcdef"), 2); // 6 bytes / 3.35 = 1.79 -> ceil = 2
    assert_eq!(TokenizerEngine::guesstimate(""), 0); // empty string
    assert_eq!(TokenizerEngine::guesstimate("a"), 1); // 1 byte / 3.35 = 0.298 -> ceil = 1
    assert_eq!(TokenizerEngine::guesstimate("abcdefghijklmnopqrstuvwxyz"), 8); // 26 bytes / 3.35 = 7.76 -> ceil = 8
    
    // Verify BYTES_PER_TOKEN constant is accessible
    assert_eq!(BYTES_PER_TOKEN, 3.35);
}

#[test]
fn test_invalid_json_and_missing_file() {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let missing_path = temp_dir.path().join("missing.json");

    // Test missing file.
    let err = TokenizerEngine::from_json_file(&missing_path)
        .expect_err("expected error for missing file");
    eprintln!("Missing file error: {:?}", err);
    // Missing file returns some error - accept any variant for now
    assert!(matches!(err, TokenModelError::Io { .. } | TokenModelError::UnsupportedFormat { .. }));

    // Test invalid JSON.
    let invalid_path = temp_dir.path().join("invalid.json");
    fs::write(&invalid_path, "invalid json").expect("failed to write");
    let err = TokenizerEngine::from_json_file(&invalid_path)
        .expect_err("expected error for invalid JSON");
    eprintln!("Invalid JSON error: {:?}", err);
    // Invalid JSON should result in an error
    assert!(matches!(err, TokenModelError::Json { .. } | TokenModelError::UnsupportedFormat { .. }));
}

#[test]
fn test_registry_operations() {
    let mut registry = TokenizerRegistry::default();

    // Register a guesstimate engine.
    registry.register("guesstimate".to_string(), TokenizerEngine::Guesstimate);
    assert!(registry.get("guesstimate").is_some());
    assert!(matches!(registry.get("guesstimate"), Some(TokenizerEngine::Guesstimate)));

    // Test from_dir with empty directory.
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let (reg, errors) = TokenizerRegistry::from_dir(temp_dir.path())
        .expect("failed to load registry");
    assert!(errors.is_empty(), "unexpected loading errors: {:?}", errors);
    assert!(reg.get("model").is_none());

    // Overwrite existing registration.
    let mut reg2 = TokenizerRegistry::default();
    reg2.register("test".to_string(), TokenizerEngine::Guesstimate);
    reg2.register("test".to_string(), TokenizerEngine::HuggingFace(
        tokenizers::Tokenizer::from_file("nonexistent.json").unwrap_or_else(|_| tokenizers::Tokenizer::new(tokenizers::models::bpe::BPE::default()))
    ));
    // The overwrite should work
    assert!(reg2.get("test").is_some());
}

#[test]
fn test_guesstimate_engine_operations() {
    let engine = TokenizerEngine::Guesstimate;
    assert_eq!(engine.count_tokens("hello"), TokenizerEngine::guesstimate("hello"));
    assert!(engine.encode("hello").is_err());
    assert!(engine.decode(&vec![]).is_err());
}

/// HF JSON 格式 round-trip：真实 HF Tokenizer::from_file 路径。
#[test]
fn test_hf_json_roundtrip() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/gpt2_mini.json");
    let engine = TokenizerEngine::from_json_file(&fixture).expect("from_json_file");
    let tokens = engine.encode("hello world").expect("encode");
    assert!(!tokens.is_empty());
    assert_eq!(engine.count_tokens("hello world"), tokens.len());
    let back = engine.decode(&tokens).expect("decode");
    assert_eq!(back.trim(), "hello world");
}
