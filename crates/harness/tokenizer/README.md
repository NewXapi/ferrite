# Tokenizer Harness Crate

This crate provides a tokenizer harness for loading and managing tokenizer models, primarily focusing on HuggingFace tokenizer models in JSON format and a guesstimate engine for approximate token counting.

## Overview

The `harness-tokenizer` crate implements a tokenizer loading strategy similar to SillyTavern's `src/endpoints/tokenizers.js`:

- **HuggingFace Tokenizer Models**: Supports HuggingFace tokenizers JSON (e.g., from Claude, llama3) using the `tokenizers` crate's `Tokenizer::from_file()` method
- **Guesstimate Engine**: Provides approximate token counting based on byte length using the formula `ceil(bytes / 3.35)`
- **Model Registry**: Can load and manage multiple tokenizer models from a directory

## Architecture

### Core Components

1. **TokenizerEngine** (`src/engine.rs`)
   - `HuggingFace(tokenizers::Tokenizer)`: Real tokenizer loaded from JSON
   - `Guesstimate`: Approximate token counting engine

2. **TokenModelError** (`src/error.rs`)
   - `Io`: I/O errors when reading model files
   - `Json`: JSON parsing errors
   - `UnsupportedFormat`: Unsupported model formats (e.g., SentencePiece .model files)
   - `Infallible`: Other errors

3. **TokenizerRegistry** (`src/registry.rs`)
   - Manages multiple tokenizer models by name
   - Can load models from a directory (`.json` files)
   - Tracks loading errors separately from successful models

### Key Constants

- `BYTES_PER_TOKEN`: Hardcoded to 3.35 (used by guesstimate engine)

### Public API

The following items are exported for external use:

```rust
pub use error::TokenModelError;
pub use error::TokenizerError;
pub use engine::TokenizerEngine;
pub use registry::TokenizerRegistry;
```

## Usage Examples

### Loading a HuggingFace Tokenizer Model

```rust
use harness_tokenizer::TokenizerEngine;
use std::path::Path;

let engine = TokenizerEngine::from_json_file(Path::new("model.json"))
    .expect("Failed to load tokenizer");

match engine {
    TokenizerEngine::HuggingFace(tokenizer) => {
        let tokens = tokenizer.encode("Hello world", true)
            .expect("Failed to encode text");
        let decoded = tokenizer.decode(tokens.ids(), true)
            .expect("Failed to decode tokens");
    },
    TokenizerEngine::Guesstimate => {
        // Approximate token counting only
    }
}
```

### Using the Guesstimate Engine

```rust
use harness_tokenizer::TokenizerEngine;

// Exact calculation
let count = TokenizerEngine::guesstimate("abcdef"); // 6 bytes / 3.35 = 1.79 -> ceil = 2
let count = TokenizerEngine::guesstimate("");       // 0
let count = TokenizerEngine::guesstimate("a");       // 1 byte / 3.35 = 0.298 -> ceil = 1

// Using the engine instance
let engine = TokenizerEngine::Guesstimate;
let count = engine.count_tokens("hello world");     // Approximate only
let encoded = engine.encode("text");                // Errors (not supported)
let decoded = engine.decode(&[1, 2, 3]);             // Errors (not supported)
```

### Using the Registry

```rust
use harness_tokenizer::TokenizerRegistry;

let mut registry = TokenizerRegistry::default();

// Register models directly
registry.register("guesstimate".to_string(), TokenizerEngine::Guesstimate);

// Load models from a directory
let (registry, errors) = TokenizerRegistry::from_dir(Path::new("/path/to/models"))
    .expect("Failed to load models from directory");

if !errors.is_empty() {
    eprintln!("Failed to load {} models:", errors.len());
    for (name, error) in errors {
        eprintln!("  {}: {}", name, error);
    }
}
```

### Testing

Run tests with:
```bash
cpulimit -l 70 -i -- cargo test -p harness-tokenizer
```

## Model Files

### Supported Formats

- **HuggingFace Tokenizers JSON**: Directly supported via `tokenizers::Tokenizer::from_file()`
  - Examples: Claude, llama3, other models using the HuggingFace tokenizer format
  - Files should have a `.json` extension

### Unsupported Formats

- **SentencePiece .model files**: Currently unsupported
  - These would require additional dependencies (e.g., `sentencepiece` crate)
  - See `TokenModelError::UnsupportedFormat` for more details

### Model Loading Strategy

1. The registry scans directories for files with `.json` extensions
2. Each `.json` file is attempted to be loaded as a HuggingFace tokenizer
3. If loading fails, the error is stored and the file is skipped
4. Successfully loaded models are registered using the file stem as the name

## Constants

- `BYTES_PER_TOKEN`: 3.35 - Used by the guesstimate engine for token approximation

## Error Handling

### TokenModelError

This error type covers issues with loading tokenizer models from files:

- `Io`: I/O errors (file not found, permission denied, etc.)
- `Json`: JSON parsing errors
- `UnsupportedFormat`: Unsupported model file format
- `Infallible`: Other errors

### TokenizerError

This error type covers issues during tokenization operations:

- `NotLoaded`: Attempted to use a tokenizer that hasn't been loaded
- `UnsupportedOperation`: Attempted to use an operation not supported by the engine (e.g., encode/decode on guesstimate engine)
- `Tokenizers`: Underlying tokenizers crate errors
- `Utf8`: UTF-8 decoding errors

## Limitations

1. **SentencePiece Support**: Not currently supported. The crate only handles HuggingFace JSON format models.
2. **Wasm**: This crate does not include `wasm-bindgen` support and is not suitable for WebAssembly targets.
3. **Performance**: The tokenizers crate dependency may introduce performance overhead for simple use cases.

## Future Enhancements

1. Add support for SentencePiece models
2. Add wasm-bindgen support for WebAssembly targets
3. Add more sophisticated guesstimate algorithms
4. Add caching mechanisms for frequently used models
5. Add command-line tools for model management

## License

MIT