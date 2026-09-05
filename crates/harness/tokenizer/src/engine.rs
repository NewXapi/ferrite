//! Tokenizer engine implementation.

use std::{fmt, path::Path};

use super::error::{TokenModelError, TokenizerError};

/// Tokenizer engine that can encode/decode and count tokens.
#[derive(Debug, Clone)]
pub enum TokenizerEngine {
    /// HuggingFace tokenizer (loaded from JSON).
    HuggingFace(tokenizers::Tokenizer),
    /// Guesstimate engine (approximate token count based on byte length).
    Guesstimate,
}

impl TokenizerEngine {
    /// Load a tokenizer model from a JSON file (HuggingFace format).
    ///
    /// Supports HuggingFace tokenizers JSON (e.g., from `claude`, `llama3`).  Other formats (e.g., SentencePiece `.model`) return `TokenModelError::UnsupportedFormat`.
    pub fn from_json_file(path: &Path) -> Result<Self, TokenModelError> {
        // The tokenizers crate provides a `Tokenizer::from_file` that reads JSON.
        match tokenizers::Tokenizer::from_file(path) {
            Ok(t) => Ok(TokenizerEngine::HuggingFace(t)),
            Err(_) => {
                // Determine error kind: unsupported path extension? tokenizers crate returns Error::InvalidFile
                // For simplicity, treat any non-JSON error as UnsupportedFormat.
                Err(TokenModelError::UnsupportedFormat { path: path.to_path_buf() })
            }
        }
    }

    /// Approximate token count using byte-based estimation.
    ///
    /// Formula: `ceil(bytes / 3.35)` (same as in SillyTavern `endpoints/tokenizers.js`).
    /// Returns `0` for empty input.
    pub fn guesstimate(text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        let bytes = text.len() as f64;
        (bytes / 3.35).ceil() as usize
    }

    /// Encode text into token IDs.
    ///
    /// Returns `TokenizerError::UnsupportedOperation` for the Guesstimate engine because it cannot actually tokenize.
    pub fn encode(&self, text: &str) -> Result<Vec<u32>, TokenizerError> {
        match self {
            TokenizerEngine::HuggingFace(t) => {
                let encoding = t.encode(text, true).map_err(TokenizerError::from)?;
                Ok(encoding.get_ids().to_vec())
            }
            TokenizerEngine::Guesstimate => Err(TokenizerError::UnsupportedOperation),
        }
    }

    /// Decode token IDs back into text.
    ///
    /// Returns `TokenizerError::UnsupportedOperation` for the Guesstimate engine.
    pub fn decode(&self, tokens: &[u32]) -> Result<String, TokenizerError> {
        match self {
            TokenizerEngine::HuggingFace(t) => t.decode(tokens, true).map_err(TokenizerError::from),
            TokenizerEngine::Guesstimate => Err(TokenizerError::UnsupportedOperation),
        }
    }

    /// Count tokens in text (actual tokenization for HF, estimate for Guesstimate).
    pub fn count_tokens(&self, text: &str) -> usize {
        match self {
            TokenizerEngine::HuggingFace(t) => {
                let encoding = t.encode(text, true).map_err(TokenizerError::from);
                encoding.map_or_else(|_| Self::guesstimate(text), |e| e.len())
            }
            TokenizerEngine::Guesstimate => Self::guesstimate(text),
        }
    }
}

impl fmt::Display for TokenizerEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenizerEngine::HuggingFace(_) => write!(f, "HuggingFace"),
            TokenizerEngine::Guesstimate => write!(f, "Guesstimate"),
        }
    }
}

/// Bytes per token for guesstimate approximation (hard‑coded from ST endpoints).
pub const BYTES_PER_TOKEN: f64 = 3.35;