//! Recursive text splitting (ported from ST `utils.js:1157` `splitRecursive`).
//!
//! The original JavaScript implementation:
//! ```js
//! function splitRecursive(text, length, delimiters) {
//!   if (length <= 0) return [text];
//!   if (delimiters.length === 0) return [text];
//!   const delimiter = delimiters[0];
//!   const parts = text.split(delimiter);
//!   if (parts.length === 1) {
//!     return splitRecursive(text, length, delimiters.slice(1));
//!   }
//!   const result = [];
//!   let current = "";
//!   for (const part of parts) {
//!     const next = current + (current ? delimiter : "") + part;
//!     if (next.length <= length) {
//!       current = next;
//!     } else {
//!       if (current) result.push(current);
//!       if (part.length > length) {
//!         result.push(...splitRecursive(part, length, delimiters.slice(1)));
//!       } else {
//!         current = part;
//!       }
//!     }
//!   }
//!   if (current) result.push(current);
//!   return result;
//! }
//! ```
//!
//! Key differences from JS version:
//! - JS counts UTF-16 code units (`.length`), Rust counts `char` (Unicode scalar values).
//!   This means the split boundaries may differ for non-BMP characters (emoji, etc.).
//! - The algorithm logic is identical otherwise.
//! - For `split_by_chunks`, we use the same `char`-counting approach.

/// Default delimiter hierarchy for recursive splitting.
/// Corresponds to ST's default: `["\n\n", "\n", " ", ""]`
pub const DEFAULT_CHUNK_DELIMITERS: &[&str] = &["\n\n", "\n", " ", ""];

/// Recursively splits text into chunks of at most `length` characters.
///
/// Ported from ST `utils.js:1157` `splitRecursive`.
///
/// # Algorithm
/// 1. If `length <= 0`, return the entire text as a single chunk.
/// 2. Try to split by the first delimiter.
/// 3. If no split occurs, recurse with the next delimiter.
/// 4. Merge adjacent parts while the combined length (including delimiter) ≤ `length`.
/// 5. If a part exceeds `length`, recursively split it with remaining delimiters.
///
/// # UTF-8 / Unicode Note
/// The original JS uses `String.length` (UTF-16 code units). This implementation
/// uses `char` count (Unicode scalar values). For ASCII and BMP characters this
/// is identical; for supplementary characters (emoji, etc.) the chunk boundaries
/// may differ by up to 1 char per supplementary code point.
pub fn split_recursive(input: &str, length: usize, delimiters: &[&str]) -> Vec<String> {
    if length == 0 || delimiters.is_empty() {
        return vec![input.to_string()];
    }

    let delimiter = delimiters[0];
    let parts: Vec<&str> = input.split(delimiter).collect();

    if parts.len() == 1 {
        // No split occurred, try next delimiter
        return split_recursive(input, length, &delimiters[1..]);
    }

    let mut result = Vec::new();
    let mut current = String::new();

    for part in parts {
        let next = if current.is_empty() {
            part.to_string()
        } else {
            current.clone() + delimiter + part
        };

        // Count chars (Unicode scalar values) instead of UTF-16 code units
        if next.chars().count() <= length {
            current = next;
        } else {
            if !current.is_empty() {
                result.push(current.clone()); // clone to fix move error
            }
            if part.chars().count() > length {
                // Part itself is too long, recurse with remaining delimiters
                result.extend(split_recursive(part, length, &delimiters[1..]));
                current.clear();
            } else {
                current = part.to_string();
            }
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

/// Splits text into chunks of at most `chunk_size` characters using default delimiters.
///
/// Corresponds to ST's `splitByChunks` / `message_chunk_size` logic:
/// - If `chunk_size <= 0`, returns the entire text as a single chunk.
/// - Otherwise uses `DEFAULT_CHUNK_DELIMITERS` for recursive splitting.
pub fn split_by_chunks(text: &str, chunk_size: usize) -> Vec<String> {
    if chunk_size == 0 {
        return vec![text.to_string()];
    }
    split_recursive(text, chunk_size, DEFAULT_CHUNK_DELIMITERS)
}
