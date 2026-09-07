//! AEAD 公共常量（ported from shoes, MIT）。

/// AEAD tag 长度（所有支持的 cipher 一致）。
pub const TAG_LEN: usize = 16;

/// 与 shoes util::allocate_vec 相同：分配未初始化长度为 len 的 Vec。
/// 调用方保证随后会写入全部元素（性能敏感的热路径）。
#[inline]
#[allow(clippy::uninit_vec)]
pub fn allocate_vec<T>(len: usize) -> Vec<T> {
    let mut ret = Vec::with_capacity(len);
    unsafe {
        ret.set_len(len);
    }
    ret
}
