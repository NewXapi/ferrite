//! Error types for tokenizer model loading and operations.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading tokenizer models from files.
#[derive(Debug, Error)]
pub enum TokenModelError {
    /// I/O error when reading the model file.
    #[error("failed to read tokenizer model from {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// JSON parsing error when deserializing the model file.
    #[error("failed to parse tokenizer model JSON from {}: {source}", path.display())]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    /// The model file format is not supported (e.g., SentencePiece .model files).
    #[error("unsupported tokenizer model format: {}", path.display())]
    UnsupportedFormat { path: PathBuf },

    /// An unexpected/infallible error occurred.
    #[error("infallible error: {0}")]
    Infallible(String),
}

/// Errors that can occur during tokenization operations.
#[derive(Debug, Error)]
pub enum TokenizerError {
    /// The tokenizer model has not been loaded.
    #[error("tokenizer not loaded")]
    NotLoaded,

    /// The tokenizer does not support this operation (e.g., guesstimate engine cannot encode/decode).
    #[error("operation not supported by this tokenizer engine")]
    UnsupportedOperation,

    /// Underlying tokenizers crate error.
    #[error("tokenizers error: {source}")]
    Tokenizers {
        #[from]
        source: tokenizers::Error,
    },

    /// UTF-8 decoding error.
    #[error("utf-8 error: {source}")]
    Utf8 {
        #[source]
        source: std::string::FromUtf8Error,
    },
}


/// Result type for tokenizer model operations.
pub type TokenModelResult<T> = Result<T, TokenModelError>;

/// Result type for tokenizer operations.
pub type TokenizerResult<T> = Result<T, TokenizerError>;
