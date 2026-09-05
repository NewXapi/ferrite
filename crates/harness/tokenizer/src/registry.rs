//! Tokenizer registry for loading and managing tokenizer models.

use std::{collections::HashMap, path::Path};

use super::error::TokenModelError;
use super::engine::TokenizerEngine;

/// Registry for loading and managing tokenizer models.
#[derive(Debug, Default)]
pub struct TokenizerRegistry {
    /// Internal map of model names to tokenizer engines.
    engines: HashMap<String, TokenizerEngine>,
}

impl TokenizerRegistry {
    /// Register a tokenizer model under a given name.
    ///
    /// If a model with the same name is already registered, it will be replaced.
    ///
    /// # Arguments
    /// * `name` - The name to register the model under.
    /// * `engine` - The tokenizer engine to register.
    pub fn register(&mut self, name: String, engine: TokenizerEngine) {
        self.engines.insert(name, engine);
    }

    /// Get a reference to a tokenizer engine by name.
    ///
    /// Returns `None` if no engine is registered under the given name.
    ///
    /// # Arguments
    /// * `name` - The name of the model to retrieve.
    pub fn get(&self, name: &str) -> Option<&TokenizerEngine> {
        self.engines.get(name)
    }

    /// Load tokenizer models from a directory.
    ///
    /// Scans the directory for files with a `.json` extension and attempts to load
    /// each as a HuggingFace tokenizer model using `TokenizerEngine::from_json_file`.
    /// If loading fails, the error is stored and the file is skipped.
    ///
    /// # Arguments
    /// * `dir` - The directory to scan for model files.
    ///
    /// Returns a vector of tuples `(name, TokenModelError)` for files that failed to load.
    /// The registry will contain only the successfully loaded models.
    ///
    /// # Errors
    /// This function can return `std::io::Error` if the directory cannot be read.
    pub fn from_dir(
        dir: &Path,
    ) -> Result<(Self, Vec<(String, TokenModelError)>), std::io::Error> {
        let mut engines = HashMap::new();
        let mut errors = Vec::new();

        let mut entries = std::fs::read_dir(dir)?;
        while let Some(entry) = entries.next() {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                // Use the file stem as the model name.
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    match TokenizerEngine::from_json_file(&path) {
                        Ok(engine) => {
                            engines.insert(name.to_string(), engine);
                        }
                        Err(e) => {
                            errors.push((name.to_string(), e));
                        }
                    }
                }
            }
        }

        Ok((Self { engines }, errors))
    }
}