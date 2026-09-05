//! Local vector index with JSON persistence (ported from ST `endpoints/vectors.js`).
//!
//! This module provides a pure-Rust, wasm-safe vector index with:
//! - `VectorItem`: hash, index, text, and embedding vector
//! - `VectorIndex`: load/save from JSON, upsert with deduplication, cosine similarity query
//!
//! Corresponds to ST's vector storage and retrieval logic.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A single vector entry with its hash, index, text, and embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorItem {
    /// 53-bit hash of the text (from `string_hash`)
    pub hash: u64,
    /// Original position/index in the source sequence
    pub index: usize,
    /// The original text content
    pub text: String,
    /// Embedding vector (typically 384, 768, 1024, or 1536 dimensions)
    pub vector: Vec<f32>,
}

/// Query result with similarity score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHit {
    /// Hash of the matched item
    pub hash: u64,
    /// Original index of the matched item
    pub index: usize,
    /// Text content of the matched item
    pub text: String,
    /// Cosine similarity score (0.0 to 1.0, higher is more similar)
    pub similarity: f32,
}

/// Local vector index persisted as JSON.
///
/// Provides:
/// - `load(path)`: load index from file (returns empty if not found)
/// - `save(path)`: persist index to file (pretty JSON)
/// - `upsert(items)`: add new items, skip duplicates by hash, return count of added
/// - `query(vector, top_k, threshold)`: cosine similarity search
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorIndex {
    items: Vec<VectorItem>,
}

impl VectorIndex {
    /// Loads a vector index from a JSON file.
    ///
    /// If the file does not exist, returns an empty index.
    /// If the file exists but is invalid JSON, returns an error.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        let index: VectorIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    /// Saves the vector index to a JSON file with pretty formatting.
    pub fn save(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Inserts new items, skipping any whose hash already exists.
    ///
    /// Returns the number of newly added items.
    pub fn upsert(&mut self, items: Vec<VectorItem>) -> usize {
        let mut added = 0;
        // Build a set of existing hashes for O(1) lookup
        let mut existing_hashes: std::collections::HashSet<u64> =
            self.items.iter().map(|i| i.hash).collect();

        for item in items {
            if existing_hashes.insert(item.hash) {
                self.items.push(item);
                added += 1;
            }
        }
        added
    }

    /// Returns all items in the index.
    pub fn items(&self) -> &[VectorItem] {
        &self.items
    }

    /// Returns the number of items in the index.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Searches for the most similar vectors using cosine similarity.
    ///
    /// # Arguments
    /// - `query_vector`: The query embedding vector
    /// - `top_k`: Maximum number of results to return
    /// - `threshold`: Optional minimum similarity (0.0 to 1.0). Results below this are filtered out.
    ///
    /// # Returns
    /// Vector of `QueryHit` sorted by similarity descending.
    ///
    /// # Behavior
    /// - Items with zero-magnitude vectors are skipped (similarity = 0)
    /// - Items producing NaN or infinite similarity are skipped
    /// - Results are sorted by similarity descending
    /// - If `threshold` is Some(v), only hits with similarity >= v are returned
    pub fn query(
        &self,
        query_vector: &[f32],
        top_k: usize,
        threshold: Option<f32>,
    ) -> Vec<QueryHit> {
        if query_vector.is_empty() || self.items.is_empty() {
            return Vec::new();
        }

        let query_norm = vector_norm(query_vector);
        if query_norm == 0.0 {
            return Vec::new();
        }

        let mut hits = Vec::new();

        for item in &self.items {
            if item.vector.len() != query_vector.len() {
                // Dimension mismatch - skip
                continue;
            }
            if item.vector.is_empty() {
                continue;
            }

            let item_norm = vector_norm(&item.vector);
            if item_norm == 0.0 {
                continue;
            }

            let sim = cosine_similarity(query_vector, &item.vector);
            if sim.is_nan() || sim.is_infinite() {
                continue;
            }

            if let Some(th) = threshold {
                if sim < th {
                    continue;
                }
            }

            hits.push(QueryHit {
                hash: item.hash,
                index: item.index,
                text: item.text.clone(),
                similarity: sim,
            });
        }

        // Sort by similarity descending
        hits.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Truncate to top_k
        if hits.len() > top_k {
            hits.truncate(top_k);
        }

        hits
    }
}

/// Computes cosine similarity between two vectors.
///
/// Returns 0.0 if either vector has zero magnitude.
/// Returns NaN if computation produces invalid result.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

fn vector_norm(v: &[f32]) -> f32 {
    let sum: f32 = v.iter().map(|x| x * x).sum();
    sum.sqrt()
}
