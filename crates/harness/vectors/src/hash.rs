//! String hashing using a cyrb53 variant (ported from ST `utils.js:522` `getStringHash`).
//!
//! The original JavaScript implementation (from bryc's cyrb53):
//! ```js
//! const cyrb53 = function(str, seed = 0) {
//!   let h1 = 0xdeadbeef ^ seed, h2 = 0x41c6ce57 ^ seed;
//!   for(let i = 0, ch; i < str.length; i++) {
//!     ch = str.charCodeAt(i);
//!     h1 = Math.imul(h1 ^ ch, 2654435761);
//!     h2 = Math.imul(h2 ^ ch, 1597334677);
//!   }
//!   h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507);
//!   h1 ^= Math.imul(h2 ^ (h2 >>> 13), 3266489909);
//!   h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507);
//!   h2 ^= Math.imul(h1 ^ (h1 >>> 13), 3266489909);
//!   return 4294967296 * (2097151 & h2) + (h1 >>> 0);
//! };
//! ```
//!
//! This Rust implementation precisely replicates the JS behavior:
//! - Uses `i32` with `wrapping_mul` to match JS `Math.imul` (signed 32-bit multiply with wrapping)
//! - Uses `u32` for logical right shift `>>>` (unsigned shift)
//! - All constants are expressed as `u32` then cast to `i32` for `wrapping_mul`

/// Computes a 53-bit hash of the input string using a cyrb53 variant.
///
/// This is a direct port of ST's `getStringHash` (utils.js:522).
/// - Uses `i32` wrapping multiplication to match JS `Math.imul`
/// - Uses logical right shift (`>>>`) via `u32` cast
/// - Returns `u64` combining the two 32-bit hash parts
///
/// # Examples
///
/// ```
/// use harness_vectors::hash::string_hash;
/// let h1 = string_hash("abc");
/// let h2 = string_hash("abc");
/// assert_eq!(h1, h2); // Same input produces same hash
/// assert_ne!(string_hash("abc"), string_hash("abd")); // Different input differs
/// ```
pub fn string_hash(text: &str) -> u64 {
    // In JS, 0xdeadbeef becomes -559038737 when stored as signed 32-bit
    // 0xdeadbeef = 3735928559, which as i32 = -559038737
    let mut h1: i32 = 0xdeadbeef_u32 as i32;
    // 0x41c6ce57 = 1103515223, fits in i32
    let mut h2: i32 = 0x41c6ce57 as i32;

    // Constants as u32 for clarity, cast to i32 for wrapping_mul
    const MUL1: i32 = 2654435761u32 as i32; // 0x9E3779B1
    const MUL2: i32 = 1597334677u32 as i32; // 0x5F4A4C4D
    const MUL3: i32 = 2246822507u32 as i32; // 0x85EBCA73
    const MUL4: i32 = 3266489909u32 as i32; // 0xC2B2AE35

    for ch in text.encode_utf16() {
        let ch_i32 = ch as i32;
        // Math.imul does signed 32-bit multiplication with wrapping
        h1 = (h1 ^ ch_i32).wrapping_mul(MUL1);
        h2 = (h2 ^ ch_i32).wrapping_mul(MUL2);
    }

    // Finalization - match JS exactly
    // h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507);
    let h1_u32 = h1 as u32;
    h1 = (h1 ^ (h1_u32 >> 16) as i32).wrapping_mul(MUL3);

    // h1 ^= Math.imul(h2 ^ (h2 >>> 13), 3266489909);
    let h2_u32 = h2 as u32;
    h1 ^= (h2 ^ (h2_u32 >> 13) as i32).wrapping_mul(MUL4);

    // h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507);
    h2 = (h2 ^ (h2_u32 >> 16) as i32).wrapping_mul(MUL3);

    // h2 ^= Math.imul(h1 ^ (h1 >>> 13), 3266489909);
    let h1_u32_final = h1 as u32;
    h2 ^= (h1 ^ (h1_u32_final >> 13) as i32).wrapping_mul(MUL4);

    // return 4294967296 * (2097151 & h2) + (h1 >>> 0);
    let h2_final = (h2 as u32) & 0x1FFFFF; // 2097151 = 0x1FFFFF
    let h1_final = h1 as u32; // h1 >>> 0 in JS is just unsigned
    ((h2_final as u64) << 32) | (h1_final as u64)
}
